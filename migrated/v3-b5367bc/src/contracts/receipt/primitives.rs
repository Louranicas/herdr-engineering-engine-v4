//! Fixed primitive and reference vocabulary; no ambient schema registry.
use super::{Address, Error, MAX_ITEMS, SCHEMA_IDS, Validate};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::marker::PhantomData;

/// Exact JSON integer in 0..=4294967295; fractions/exponents are refused.
pub type Count = u32;
/// JSON boolean, never an integer surrogate.
pub type Bool = bool;
impl Validate for u32 {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
impl Validate for bool {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}

macro_rules! string_type {
    ($name:ident, $check:expr) => {
        #[doc = concat!("Validated RC04 ", stringify!($name), " scalar.")]
        #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);
        impl $name {
            /// Construct an owned scalar without trimming or normalization.
            /// # Errors
            /// Refuses invalid spelling or encoded byte length.
            pub fn new(value: impl Into<String>) -> Result<Self, Error> {
                let value = Self(value.into());
                value.validate()?;
                Ok(value)
            }
            /// Borrow the exact admitted spelling.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl Validate for $name {
            fn validate(&self) -> Result<(), Error> {
                ($check)(self.0.as_str())
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
                Self::new(String::deserialize(de)?).map_err(serde::de::Error::custom)
            }
        }
    };
}
string_type!(Id, |s: &str| crate::contracts::UuidV4::parse(s)
    .map(|_| ())
    .map_err(|_| Error::Scalar));
string_type!(Sha, |s: &str| crate::contracts::Sha256Digest::parse(s)
    .map(|_| ())
    .map_err(|_| Error::Scalar));
string_type!(U64, |s: &str| crate::contracts::parse_u64_decimal(s)
    .map(|_| ())
    .map_err(|_| Error::Scalar));
string_type!(Generation, |s: &str| {
    let n = crate::contracts::parse_u64_decimal(s).map_err(|_| Error::Scalar)?;
    if n == 0 { Err(Error::Scalar) } else { Ok(()) }
});
string_type!(Text, |s: &str| if s.len() <= 4096 {
    Ok(())
} else {
    Err(Error::Bound)
});
string_type!(Name, |s: &str| {
    if s.is_empty() || s.len() > 128 {
        Err(Error::Bound)
    } else if s.is_ascii() {
        Ok(())
    } else {
        Err(Error::Scalar)
    }
});
string_type!(Reason, |s: &str| {
    if s.is_empty() || s.len() > 4096 {
        Err(Error::Bound)
    } else {
        Ok(())
    }
});
string_type!(RelPath, |s: &str| {
    if s.is_empty() || s.len() > 4096 {
        return Err(Error::Bound);
    }
    if s.contains('\0')
        || s.split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        Err(Error::Scalar)
    } else {
        Ok(())
    }
});
impl U64 {
    /// Numerical value of a validated canonical spelling.
    #[must_use]
    pub fn get(&self) -> u64 {
        self.0
            .bytes()
            .fold(0, |value, digit| value * 10 + u64::from(digit - b'0'))
    }
}
impl Generation {
    /// Numerical value of a validated canonical nonzero spelling.
    #[must_use]
    pub fn get(&self) -> u64 {
        self.0
            .bytes()
            .fold(0, |value, digit| value * 10 + u64::from(digit - b'0'))
    }
}

/// A bounded ordered inline array, with no implicit truncation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct List<T>(Vec<T>);
impl<T> List<T> {
    /// # Errors
    /// Refuses more than 256 elements; recursive content is checked by Validate.
    pub fn new(values: Vec<T>) -> Result<Self, Error> {
        if values.len() > MAX_ITEMS {
            Err(Error::Bound)
        } else {
            Ok(Self(values))
        }
    }
    /// Borrow the original order.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
    /// Consume the bounded container.
    #[must_use]
    pub fn into_inner(self) -> Vec<T> {
        self.0
    }
}
impl<T: Validate> Validate for List<T> {
    fn validate(&self) -> Result<(), Error> {
        if self.0.len() > MAX_ITEMS {
            return Err(Error::Bound);
        }
        for value in &self.0 {
            value.validate()?;
        }
        Ok(())
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for List<T> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Items<T>(PhantomData<T>);
        struct Excess;
        impl<'de> serde::de::DeserializeSeed<'de> for Excess {
            type Value = ();
            fn deserialize<D: Deserializer<'de>>(self, _: D) -> Result<(), D::Error> {
                Err(serde::de::Error::custom("receipt array bound exceeded"))
            }
        }
        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Items<T> {
            type Value = List<T>;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("at most 256 values")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<List<T>, A::Error> {
                let mut values = Vec::new();
                while values.len() < MAX_ITEMS {
                    match seq.next_element()? {
                        Some(value) => values.push(value),
                        None => return Ok(List(values)),
                    }
                }
                seq.next_element_seed(Excess)?;
                Ok(List(values))
            }
        }
        de.deserialize_seq(Items(PhantomData))
    }
}

fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(de: D) -> Result<Option<T>, D::Error> {
    Option::deserialize(de)
}
/// Exactly one available value or explicit unavailable reason; both keys required.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Maybe<T> {
    /// Present value or explicit null.
    pub value: Option<T>,
    /// Explicit reason or null. Empty Text remains valid under RC04.
    pub unavailable_reason: Option<Text>,
}
impl<T> Maybe<T> {
    /// Construct the available branch.
    pub fn present(value: T) -> Self {
        Self {
            value: Some(value),
            unavailable_reason: None,
        }
    }
    /// Construct the unavailable branch.
    #[must_use]
    pub fn unavailable(reason: Text) -> Self {
        Self {
            value: None,
            unavailable_reason: Some(reason),
        }
    }
}
impl<T: Validate> Validate for Maybe<T> {
    fn validate(&self) -> Result<(), Error> {
        match (&self.value, &self.unavailable_reason) {
            (Some(value), None) => value.validate(),
            (None, Some(reason)) => reason.validate(),
            _ => Err(Error::Invariant),
        }
    }
}

/// Exact descriptive metadata; validation does not resolve its bytes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Ref {
    pub artifact_id: Id,
    pub sha256: Sha,
    pub byte_length: Count,
    pub media_type: Name,
    pub schema_id: Name,
}
impl Validate for Ref {
    fn validate(&self) -> Result<(), Error> {
        self.artifact_id.validate()?;
        self.sha256.validate()?;
        self.media_type.validate()?;
        self.schema_id.validate()?;
        if self.schema_id.as_str() == Raw::SCHEMA_ID
            || SCHEMA_IDS.contains(&self.schema_id.as_str())
        {
            Ok(())
        } else {
            Err(Error::Schema)
        }
    }
}
/// Marker for opaque bytes; not an addressable JSON record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Raw;
impl Address for Raw {
    const SCHEMA_ID: &'static str = "hee3.raw/1";
}
/// Ref metadata whose schema target is fixed by its field's Rust type.
pub struct TypedRef<T> {
    reference: Ref,
    marker: PhantomData<fn() -> T>,
}
impl<T> std::fmt::Debug for TypedRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.reference.fmt(f)
    }
}
impl<T> Clone for TypedRef<T> {
    fn clone(&self) -> Self {
        Self {
            reference: self.reference.clone(),
            marker: PhantomData,
        }
    }
}
impl<T> PartialEq for TypedRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.reference == other.reference
    }
}
impl<T> Eq for TypedRef<T> {}
impl<T: Address> TypedRef<T> {
    /// # Errors
    /// Refuses invalid metadata or a different schema target.
    pub fn new(reference: Ref) -> Result<Self, Error> {
        let value = Self {
            reference,
            marker: PhantomData,
        };
        value.validate()?;
        Ok(value)
    }
    /// Consume the target constraint without resolving the reference.
    #[must_use]
    pub fn into_inner(self) -> Ref {
        self.reference
    }
}
impl<T: Address> Validate for TypedRef<T> {
    fn validate(&self) -> Result<(), Error> {
        self.reference.validate()?;
        if self.reference.schema_id.as_str() == T::SCHEMA_ID {
            Ok(())
        } else {
            Err(Error::Schema)
        }
    }
}
impl<T> Serialize for TypedRef<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.reference.serialize(serializer)
    }
}
impl<'de, T: Address> Deserialize<'de> for TypedRef<T> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        Self::new(Ref::deserialize(de)?).map_err(serde::de::Error::custom)
    }
}
/// A target-constrained reference to opaque exact bytes.
pub type Payload = TypedRef<Raw>;

// Derive normally admits positional sequences for a struct. This adapter forces
// its generated field visitor through deserialize_map at each named boundary.
pub(super) struct MapOnly<D>(pub D);
impl<'de, D: Deserializer<'de>> Deserializer<'de> for MapOnly<D> {
    type Error = D::Error;
    fn deserialize_any<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0.deserialize_any(visitor)
    }
    fn deserialize_struct<V: serde::de::Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0.deserialize_map(visitor)
    }
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map
        enum identifier ignored_any
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Maybe<T> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields, bound(deserialize = "T: Deserialize<'de>"))]
        struct Fields<T> {
            #[serde(deserialize_with = "nullable")]
            value: Option<T>,
            #[serde(deserialize_with = "nullable")]
            unavailable_reason: Option<Text>,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            value: fields.value,
            unavailable_reason: fields.unavailable_reason,
        })
    }
}
impl<'de> Deserialize<'de> for Ref {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            artifact_id: Id,
            sha256: Sha,
            byte_length: Count,
            media_type: Name,
            schema_id: Name,
        }
        let fields = Fields::deserialize(MapOnly(de))?;
        Ok(Self {
            artifact_id: fields.artifact_id,
            sha256: fields.sha256,
            byte_length: fields.byte_length,
            media_type: fields.media_type,
            schema_id: fields.schema_id,
        })
    }
}

impl<T> AsRef<Ref> for TypedRef<T> {
    fn as_ref(&self) -> &Ref {
        &self.reference
    }
}
