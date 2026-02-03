//! Tags (attributes) encoder and decoder for MVT.

use foldhash::fast::RandomState;
use indexmap::IndexSet;

use crate::vector_tile::tile;

#[derive(Default)]
pub struct TagsEncoder {
    keys: IndexSet<String, RandomState>,
    values: IndexSet<Value, RandomState>,
    tags: Vec<u32>,
}

/// Utility for encoding MVT tags (attributes).
impl TagsEncoder {
    pub fn new() -> Self {
        Default::default()
    }

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
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}
impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}
impl From<u64> for Value {
    fn from(v: u64) -> Self {
        Value::Uint(v)
    }
}
impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Value::Uint(v as u64)
    }
}
impl From<i64> for Value {
    fn from(v: i64) -> Self {
        if v >= 0 {
            Value::Uint(v as u64)
        } else {
            Value::SInt(v)
        }
    }
}
impl From<i32> for Value {
    fn from(v: i32) -> Self {
        if v >= 0 {
            Value::Uint(v as u64)
        } else {
            Value::SInt(v as i64)
        }
    }
}
impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::Float(v.to_ne_bytes())
    }
}
impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Double(v.to_ne_bytes())
    }
}
impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl Value {
    /// Creates a Value from a tile::Value.
    #[allow(clippy::manual_map)]
    pub fn from_tile_value(v: &tile::Value) -> Option<Self> {
        if let Some(s) = &v.string_value {
            Some(Value::String(s.clone()))
        } else if let Some(f) = v.float_value {
            Some(Value::Float(f.to_ne_bytes()))
        } else if let Some(d) = v.double_value {
            Some(Value::Double(d.to_ne_bytes()))
        } else if let Some(i) = v.int_value {
            Some(Value::Int(i))
        } else if let Some(u) = v.uint_value {
            Some(Value::Uint(u))
        } else if let Some(s) = v.sint_value {
            Some(Value::SInt(s))
        } else if let Some(b) = v.bool_value {
            Some(Value::Bool(b))
        } else {
            None
        }
    }
}

/// Utility for decoding MVT tags (attributes).
pub struct TagsDecoder<'a> {
    keys: &'a [String],
    values: &'a [tile::Value],
}

impl<'a> TagsDecoder<'a> {
    /// Creates a new decoder with the layer's keys and values dictionaries.
    pub fn new(keys: &'a [String], values: &'a [tile::Value]) -> Self {
        Self { keys, values }
    }

    /// Decodes tags into a vector of key-value pairs.
    pub fn decode(&self, tags: &[u32]) -> Result<Vec<(&'a str, Value)>, String> {
        if !tags.len().is_multiple_of(2) {
            return Err("Tags array must have even length".to_string());
        }

        let mut result = Vec::with_capacity(tags.len() / 2);

        for chunk in tags.chunks_exact(2) {
            let key_idx = chunk[0] as usize;
            let value_idx = chunk[1] as usize;

            if key_idx >= self.keys.len() {
                return Err(format!("Key index {} out of bounds", key_idx));
            }
            if value_idx >= self.values.len() {
                return Err(format!("Value index {} out of bounds", value_idx));
            }

            let key = &self.keys[key_idx];
            let value = Value::from_tile_value(&self.values[value_idx])
                .ok_or_else(|| format!("Invalid tile value at index {}", value_idx))?;

            result.push((key.as_str(), value));
        }

        Ok(result)
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
            [
                1, 3, 2, 4, 3, 3, 3, 5, 3, 6, 3, 7, 2, 8, 4, 4, 3, 5, 3, 3, 5, 9, 5, 10
            ]
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
    fn test_tags_decoder() {
        let mut encoder = TagsEncoder::new();
        encoder.add("name", "road");
        encoder.add("type", "highway");
        encoder.add("lanes", 4u32);
        encoder.add("maxspeed", 60.5f32);
        encoder.add("oneway", true);

        let tags = encoder.take_tags();
        let (keys, values) = encoder.into_keys_and_values();

        let decoder = TagsDecoder::new(&keys, &values);
        let decoded = decoder.decode(&tags).unwrap();

        assert_eq!(decoded.len(), 5);
        assert_eq!(decoded[0], ("name", Value::String("road".to_string())));
        assert_eq!(decoded[1], ("type", Value::String("highway".to_string())));
        assert_eq!(decoded[2], ("lanes", Value::Uint(4)));
        assert_eq!(
            decoded[3],
            ("maxspeed", Value::Float(60.5f32.to_ne_bytes()))
        );
        assert_eq!(decoded[4], ("oneway", Value::Bool(true)));
    }

    #[test]
    fn test_tags_decoder_roundtrip() {
        let mut encoder = TagsEncoder::new();
        encoder.add("uint", Value::Uint(10));
        encoder.add("sint", Value::SInt(-10));
        encoder.add("int", Value::Int(10));
        encoder.add("string", Value::String("test".to_string()));
        encoder.add("float", 10.5f32);
        encoder.add("double", 20.5f64);
        encoder.add("bool", true);

        let tags = encoder.take_tags();
        let (keys, values) = encoder.into_keys_and_values();

        let decoder = TagsDecoder::new(&keys, &values);
        let decoded = decoder.decode(&tags).unwrap();

        assert_eq!(decoded.len(), 7);
        assert_eq!(decoded[0], ("uint", Value::Uint(10)));
        assert_eq!(decoded[1], ("sint", Value::SInt(-10)));
        assert_eq!(decoded[2], ("int", Value::Int(10)));
        assert_eq!(decoded[3], ("string", Value::String("test".to_string())));
        assert_eq!(decoded[4], ("float", Value::Float(10.5f32.to_ne_bytes())));
        assert_eq!(decoded[5], ("double", Value::Double(20.5f64.to_ne_bytes())));
        assert_eq!(decoded[6], ("bool", Value::Bool(true)));
    }

    #[test]
    fn test_tags_decoder_error_odd_length() {
        let keys = vec!["key".to_string()];
        let values = vec![tile::Value {
            string_value: Some("value".to_string()),
            ..Default::default()
        }];
        let decoder = TagsDecoder::new(&keys, &values);

        let result = decoder.decode(&[0]);
        assert!(result.is_err());
    }

    #[test]
    fn test_tags_decoder_error_key_out_of_bounds() {
        let keys = vec!["key".to_string()];
        let values = vec![tile::Value {
            string_value: Some("value".to_string()),
            ..Default::default()
        }];
        let decoder = TagsDecoder::new(&keys, &values);

        let result = decoder.decode(&[99, 0]);
        assert!(result.is_err());
    }

    #[test]
    fn test_tags_decoder_error_value_out_of_bounds() {
        let keys = vec!["key".to_string()];
        let values = vec![tile::Value {
            string_value: Some("value".to_string()),
            ..Default::default()
        }];
        let decoder = TagsDecoder::new(&keys, &values);

        let result = decoder.decode(&[0, 99]);
        assert!(result.is_err());
    }
}
