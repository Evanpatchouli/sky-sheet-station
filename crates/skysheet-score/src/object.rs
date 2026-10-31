//! Object-only deserialization for the score's named JSON records.

use serde::de::{MapAccess, Visitor, value::MapAccessDeserializer};
use serde::{Deserialize, Deserializer};
use std::{fmt, marker::PhantomData};

pub(crate) fn deserialize_object<'de, D, T>(
    deserializer: D,
    name: &'static str,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct ObjectVisitor<T> {
        name: &'static str,
        marker: PhantomData<T>,
    }

    impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectVisitor<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "an object for {}", self.name)
        }

        fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<T, M::Error> {
            // Forward the original stream, preserving duplicate fields and
            // numeric tokens for the derived field reader. Do not buffer a Value.
            T::deserialize(MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_map(ObjectVisitor {
        name,
        marker: PhantomData,
    })
}

// Keep public fields and the private derived field reader in one declaration.
// Only the outer representation is restricted; Serde retains responsibility for
// field types, defaults, missing/duplicate fields, and unknown-field rejection.
macro_rules! score_object {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $($(#[$field_meta:meta])* pub $field:ident: $ty:ty,)*
        }
    ) => {
        $(#[$meta])*
        pub struct $name {
            $($(#[$field_meta])* pub $field: $ty,)*
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Fields {
                    $($(#[$field_meta])* $field: $ty,)*
                }

                let fields = crate::object::deserialize_object::<D, Fields>(deserializer, stringify!($name))?;
                Ok(Self { $($field: fields.$field,)* })
            }
        }
    };
}

pub(crate) use score_object;
