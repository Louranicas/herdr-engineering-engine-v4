//! Keyset pages, pure: `PageIn` parses `body.page`, `page` slices a sorted set after a key,
//! `PageOut` is the reply shape. A `Cursor` is pinned to the engine boot and the filter digest it
//! was issued under; resuming it under another is `resync_required` with the reason named.

use hee4_contracts::Sha256Hex;
use hee4_contracts::bounds::{MAX_AFTER_KEY_BYTES, MAX_PAGE_LIMIT};
use serde_json::{Value, json};

use crate::wire::{Code, Fault};

/// Where a listing resumes: `{after_key, boot, filter_sha256}` on the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    /// The last key of the page it was issued with; the next page starts after it.
    pub after_key: String,
    /// The engine boot (unix nanoseconds) the listing was started under.
    pub boot: u64,
    /// The digest of the filter the listing was started with.
    pub filter_sha256: Sha256Hex,
}

impl Cursor {
    /// Parse the wire object. Refusals are `invalid_argument` at `/body/page/cursor`.
    ///
    /// # Errors
    /// Not an object with `after_key` (string of at most [`MAX_AFTER_KEY_BYTES`]), `boot`
    /// (unsigned integer) and `filter_sha256` (64 lowercase hex).
    pub fn parse(v: &Value) -> Result<Self, Fault> {
        let bad = |msg: String| Fault::new(Code::InvalidArgument, "/body/page/cursor", msg);
        let Value::Object(obj) = v else {
            return Err(bad("object required".into()));
        };
        let after_key = obj
            .get("after_key")
            .and_then(Value::as_str)
            .ok_or_else(|| bad("after_key: string required".into()))?;
        if after_key.len() > MAX_AFTER_KEY_BYTES {
            return Err(bad(format!(
                "after_key: {} bytes, at most {MAX_AFTER_KEY_BYTES}",
                after_key.len()
            )));
        }
        let boot = obj
            .get("boot")
            .and_then(Value::as_u64)
            .ok_or_else(|| bad("boot: unsigned integer required".into()))?;
        let filter_sha256 = obj
            .get("filter_sha256")
            .and_then(Value::as_str)
            .ok_or_else(|| bad("filter_sha256: string required".into()))?
            .parse::<Sha256Hex>()
            .map_err(|e| bad(format!("filter_sha256: {e}")))?;
        Ok(Self {
            after_key: after_key.to_owned(),
            boot,
            filter_sha256,
        })
    }

    /// The wire object.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "after_key": self.after_key,
            "boot": self.boot,
            "filter_sha256": self.filter_sha256.to_string(),
        })
    }

    /// Whether this cursor may continue a listing under `boot` and `filter`.
    ///
    /// # Errors
    /// `resync_required` at `/body/page/cursor`, `because` "epoch moved" or "filter moved".
    pub fn resumes(&self, boot: u64, filter: &Sha256Hex) -> Result<(), Fault> {
        let refuse = |because| {
            Fault::new(
                Code::ResyncRequired,
                "/body/page/cursor",
                "start the listing again",
            )
            .with_because(because)
        };
        if self.boot != boot {
            return Err(refuse("epoch moved"));
        }
        if self.filter_sha256 != *filter {
            return Err(refuse("filter moved"));
        }
        Ok(())
    }
}

/// The parsed `body.page`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageIn {
    /// Items per page, `1..=MAX_PAGE_LIMIT`.
    pub limit: usize,
    /// Where to resume.
    pub cursor: Option<Cursor>,
}

impl PageIn {
    /// Parse `body.get("page")`. A missing or null page is `{MAX_PAGE_LIMIT, None}`.
    ///
    /// # Errors
    /// `invalid_argument` at `/body/page` (not an object), `/body/page/limit` (not an integer,
    /// 0, or over [`MAX_PAGE_LIMIT`]; the message carries both numbers) or `/body/page/cursor`.
    pub fn parse(page: Option<&Value>) -> Result<Self, Fault> {
        let obj = match page {
            None | Some(Value::Null) => {
                return Ok(Self {
                    limit: MAX_PAGE_LIMIT,
                    cursor: None,
                });
            }
            Some(Value::Object(obj)) => obj,
            Some(_) => {
                return Err(Fault::new(
                    Code::InvalidArgument,
                    "/body/page",
                    "object or null",
                ));
            }
        };
        let limit = match obj.get("limit") {
            None | Some(Value::Null) => MAX_PAGE_LIMIT,
            Some(v) => {
                let n = v.as_u64().and_then(|n| usize::try_from(n).ok());
                match n {
                    Some(n) if (1..=MAX_PAGE_LIMIT).contains(&n) => n,
                    _ => {
                        return Err(Fault::new(
                            Code::InvalidArgument,
                            "/body/page/limit",
                            format!("limit {v} is not in 1..={MAX_PAGE_LIMIT}"),
                        ));
                    }
                }
            }
        };
        let cursor = match obj.get("cursor") {
            None | Some(Value::Null) => None,
            Some(v) => Some(Cursor::parse(v)?),
        };
        Ok(Self { limit, cursor })
    }
}

/// One page of a reply: `{items, cursor}`; `cursor` is null on the last page.
#[derive(Debug, Clone, PartialEq)]
pub struct PageOut {
    /// The items.
    pub items: Vec<Value>,
    /// Where the next page starts, when there is one.
    pub cursor: Option<Cursor>,
}

impl PageOut {
    /// The wire object.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "items": self.items,
            "cursor": self.cursor.as_ref().map(Cursor::to_json),
        })
    }
}

/// The keyset slice: at most `limit` items of `sorted` (ascending by `key`) whose key is after
/// `after`, and whether more remain after the slice.
pub fn page<'a, T>(
    sorted: &'a [T],
    key: fn(&T) -> &str,
    after: Option<&str>,
    limit: usize,
) -> (&'a [T], bool) {
    let start = after.map_or(0, |after| sorted.partition_point(|t| key(t) <= after));
    let end = start.saturating_add(limit).min(sorted.len());
    (&sorted[start..end], end < sorted.len())
}

#[cfg(test)]
mod tests {
    use super::{Cursor, PageIn, page};
    use crate::wire::Code;
    use hee4_contracts::Sha256Hex;
    use hee4_contracts::bounds::MAX_PAGE_LIMIT;
    use serde_json::json;

    #[test]
    fn missing_page_is_the_default() {
        let p = PageIn::parse(None).ok();
        assert_eq!(
            p,
            Some(PageIn {
                limit: MAX_PAGE_LIMIT,
                cursor: None
            })
        );
        assert_eq!(PageIn::parse(Some(&json!(null))).ok(), p);
    }

    #[test]
    fn each_refusal_names_its_pointer() {
        let field = |v: serde_json::Value| PageIn::parse(Some(&v)).err().map(|f| (f.code, f.field));
        assert_eq!(field(json!(5)), Some((Code::InvalidArgument, "/body/page")));
        assert_eq!(
            field(json!({"limit": 0})),
            Some((Code::InvalidArgument, "/body/page/limit"))
        );
        let over = PageIn::parse(Some(&json!({"limit": MAX_PAGE_LIMIT + 1}))).err();
        let message = over.as_ref().map(|f| f.message.clone()).unwrap_or_default();
        assert!(message.contains(&MAX_PAGE_LIMIT.to_string()), "{message}");
        assert!(
            message.contains(&(MAX_PAGE_LIMIT + 1).to_string()),
            "{message}"
        );
        assert_eq!(
            field(json!({"limit": "ten"})),
            Some((Code::InvalidArgument, "/body/page/limit"))
        );
        assert_eq!(
            field(json!({"cursor": 7})),
            Some((Code::InvalidArgument, "/body/page/cursor"))
        );
        let long = "k".repeat(hee4_contracts::bounds::MAX_AFTER_KEY_BYTES + 1);
        let digest = Sha256Hex::digest(b"").to_string();
        assert_eq!(
            field(json!({"cursor": {"after_key": long, "boot": 1, "filter_sha256": digest}})),
            Some((Code::InvalidArgument, "/body/page/cursor"))
        );
    }

    #[test]
    fn a_full_page_yields_a_cursor_after_its_last_key() {
        let items = ["a", "b", "c", "d", "e"];
        let (first, more) = page(&items, |s| s, None, 2);
        assert_eq!(first, ["a", "b"]);
        assert!(more);
        let cursor = Cursor {
            after_key: (*first.last().unwrap_or(&"")).to_owned(),
            boot: 9,
            filter_sha256: Sha256Hex::digest(b""),
        };
        assert_eq!(cursor.after_key, "b");
        let (second, more) = page(&items, |s| s, Some(&cursor.after_key), 2);
        assert_eq!(second, ["c", "d"]);
        assert!(more);
        let (third, more) = page(&items, |s| s, Some("d"), 2);
        assert_eq!(third, ["e"]);
        assert!(!more);
        let (none, more) = page(&items, |s| s, Some("e"), 2);
        assert_eq!(none.len(), 0);
        assert!(!more);
        let parsed = Cursor::parse(&cursor.to_json());
        assert_eq!(parsed.ok(), Some(cursor));
    }

    #[test]
    fn a_cursor_from_another_boot_or_filter_is_refused_by_name() {
        let filter = Sha256Hex::digest(b"q");
        let cursor = Cursor {
            after_key: "b".into(),
            boot: 1,
            filter_sha256: filter,
        };
        assert_eq!(cursor.resumes(1, &filter), Ok(()));
        let epoch = cursor.resumes(2, &filter).err();
        assert_eq!(
            epoch.as_ref().map(|f| (f.code, f.field, f.because)),
            Some((
                Code::ResyncRequired,
                "/body/page/cursor",
                Some("epoch moved")
            ))
        );
        let moved = cursor.resumes(1, &Sha256Hex::digest(b"other")).err();
        assert_eq!(moved.and_then(|f| f.because), Some("filter moved"));
    }
}
