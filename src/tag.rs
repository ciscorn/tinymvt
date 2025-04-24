//! Tags (attributes) encoder for MVT.

use std::hash::BuildHasher;

use foldhash::fast::RandomState;
use indexmap::IndexSet;

use crate::vector_tile::tile;

/// Utility for encoding MVT tags (attributes).
pub struct TagsEncoder<S = RandomState> {
    keys: IndexSet<String, S>,
    values: IndexSet<Value, S>,
    tags: Vec<u32>,
}

impl TagsEncoder {
    /// Creates a new encoder with a default hasher.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S: Default> Default for TagsEncoder<S> {
    #[inline]
    fn default() -> Self {
        Self {
            keys: IndexSet::default(),
            values: IndexSet::default(),
            tags: Vec::default(),
        }
    }
}

impl<S: BuildHasher + Clone> TagsEncoder<S> {
    #[inline]
    pub fn with_hasher(hasher: S) -> Self {
        Self {
            keys: IndexSet::with_hasher(hasher.clone()),
            values: IndexSet::with_hasher(hasher),
            tags: Vec::default(),
        }
    }
}

impl<S: BuildHasher> TagsEncoder<S> {
    /// Adds a key-value pair for the current feature.
    #[inline]
    pub fn add(&mut self, key: &str, value: impl Into<Value>) {
        self.add_inner(key, value.into());
    }

    #[inline]
    fn add_inner(&mut self, key: &str, value: Value) {
        let key_idx = match self.keys.get_index_of(key) {
            None => self.keys.insert_full(key.to_string()).0,
            Some(idx) => idx,
        };
        let value_idx = match self.values.get_index_of(&value) {
            None => self.values.insert_full(value).0,
            Some(idx) => idx,
        };
        self.tags.extend([key_idx as u32, value_idx as u32]);
    }

    /// Takes the key-value index buffer for the current feature.
    #[inline]
    pub fn take_tags(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.tags)
    }

    /// Consumes the encoder and returns the keys and values for a layer.
    #[inline]
    pub fn into_keys_and_values(self) -> (Vec<String>, Vec<tile::Value>) {
        let keys = self.keys.into_iter().collect();
        let values = self
            .values
            .into_iter()
            .map(|v| v.into_tile_value())
            .collect();
        (keys, values)
    }
}

/// Comparable wrapper for the MVT values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Value {
    String(String),
    Float([u8; 4]),
    Double([u8; 8]),
    Int(i64),
    Uint(u64),
    SInt(i64),
    Bool(bool),
}

impl Value {
    #[inline]
    pub fn into_tile_value(self) -> tile::Value {
        use Value::*;
        match self {
            String(v) => tile::Value {
                string_value: Some(v),
                ..Default::default()
            },
            Float(v) => tile::Value {
                float_value: Some(f32::from_ne_bytes(v)),
                ..Default::default()
            },
            Double(v) => tile::Value {
                double_value: Some(f64::from_ne_bytes(v)),
                ..Default::default()
            },
            Int(v) => tile::Value {
                int_value: Some(v),
                ..Default::default()
            },
            Uint(v) => tile::Value {
                uint_value: Some(v),
                ..Default::default()
            },
            SInt(v) => tile::Value {
                sint_value: Some(v),
                ..Default::default()
            },
            Bool(v) => tile::Value {
                bool_value: Some(v),
                ..Default::default()
            },
        }
    }
}

impl From<&str> for Value {
    #[inline]
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}
impl From<String> for Value {
    #[inline]
    fn from(v: String) -> Self {
        Value::String(v)
    }
}
impl From<u64> for Value {
    #[inline]
    fn from(v: u64) -> Self {
        Value::Uint(v)
    }
}
impl From<u32> for Value {
    #[inline]
    fn from(v: u32) -> Self {
        Value::Uint(v as u64)
    }
}
impl From<i64> for Value {
    #[inline]
    fn from(v: i64) -> Self {
        if v >= 0 {
            Value::Uint(v as u64)
        } else {
            Value::SInt(v)
        }
    }
}
impl From<i32> for Value {
    #[inline]
    fn from(v: i32) -> Self {
        if v >= 0 {
            Value::Uint(v as u64)
        } else {
            Value::SInt(v as i64)
        }
    }
}
impl From<f32> for Value {
    #[inline]
    fn from(v: f32) -> Self {
        Value::Float(v.to_ne_bytes())
    }
}
impl From<f64> for Value {
    #[inline]
    fn from(v: f64) -> Self {
        Value::Double(v.to_ne_bytes())
    }
}
impl From<bool> for Value {
    #[inline]
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tags_encoder() {
        let mut encoder = TagsEncoder::new();
        encoder.add("k0", "v0");
        encoder.add("k0", "v0");
        encoder.add("k1", "v0");
        encoder.add("k1", "v1");
        assert_eq!(encoder.take_tags(), [0, 0, 0, 0, 1, 0, 1, 1]);

        encoder.add("k0", "v0");
        encoder.add("k0", "v2");
        encoder.add("k1", "v2");
        encoder.add("k2", "v0".to_string());
        encoder.add("k1", "v1");
        encoder.add("k1", "v1".to_string());
        assert_eq!(encoder.take_tags(), [0, 0, 0, 2, 1, 2, 2, 0, 1, 1, 1, 1]);

        encoder.add("k1", 10i32);
        encoder.add("k2", 10.5f64);
        encoder.add("k3", 10u32);
        encoder.add("k3", -10i32);
        encoder.add("k3", true);
        encoder.add("k3", 1);
        encoder.add("k2", 10.5f32);
        encoder.add("k4", 10.5f64);
        encoder.add("k3", -10i64);
        encoder.add("k3", 10u64);
        encoder.add("k5", Value::Int(11));
        encoder.add("k5", 12i64);
        assert_eq!(
            encoder.take_tags(),
            [1, 3, 2, 4, 3, 3, 3, 5, 3, 6, 3, 7, 2, 8, 4, 4, 3, 5, 3, 3, 5, 9, 5, 10]
        );

        let (keys, values) = encoder.into_keys_and_values();
        assert_eq!(keys, vec!["k0", "k1", "k2", "k3", "k4", "k5"]);
        assert_eq!(
            values,
            vec![
                tile::Value {
                    string_value: Some("v0".to_string()),
                    ..Default::default()
                },
                tile::Value {
                    string_value: Some("v1".to_string()),
                    ..Default::default()
                },
                tile::Value {
                    string_value: Some("v2".to_string()),
                    ..Default::default()
                },
                tile::Value {
                    uint_value: Some(10),
                    ..Default::default()
                },
                tile::Value {
                    double_value: Some(10.5),
                    ..Default::default()
                },
                tile::Value {
                    sint_value: Some(-10),
                    ..Default::default()
                },
                tile::Value {
                    bool_value: Some(true),
                    ..Default::default()
                },
                tile::Value {
                    uint_value: Some(1),
                    ..Default::default()
                },
                tile::Value {
                    float_value: Some(10.5),
                    ..Default::default()
                },
                tile::Value {
                    int_value: Some(11),
                    ..Default::default()
                },
                tile::Value {
                    uint_value: Some(12),
                    ..Default::default()
                },
            ]
        );
    }

    #[test]
    pub fn with_hasher() {
        let rs = std::hash::RandomState::new();
        let _ = TagsEncoder::with_hasher(rs);
    }
}
