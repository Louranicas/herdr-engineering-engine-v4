//! Bounds shared by the contract types.

/// Longest identifier token, in bytes.
pub const MAX_TOKEN_BYTES: usize = 128;

/// Longest refusal text, in bytes.
pub const MAX_TEXT_BYTES: usize = 512;

/// Largest `page.limit` of a paged read (API Map P-3 resolved toward 100: `roster.list`,
/// `thread.list` and `task.list` say 1..100; `tools.list.md` sub-feature catalogue-page).
pub const MAX_PAGE_LIMIT: usize = 100;

/// Longest `tools.list` `query`, in bytes (`gates/features/tools.list.md` query-filter).
pub const MAX_QUERY_BYTES: usize = 256;

/// Longest request deadline, in milliseconds (`gates/features/README.md` wire shape: deadline
/// at most 60,000 ms ahead; `tools.inspect.md` `max_deadline_ms` <= 60000).
pub const MAX_DEADLINE_MS: u64 = 60_000;

/// Longest keyset cursor `after_key`, in bytes (the same bound as a query; API Map P-3).
pub const MAX_AFTER_KEY_BYTES: usize = 256;

/// Most items one view array carries (`roster.disable.md` reply-obligations: arrays <= 100).
pub const MAX_VIEW_ITEMS: usize = 100;
