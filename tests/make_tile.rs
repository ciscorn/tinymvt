use std::fs;

use prost::Message;
use tinymvt::geometry::GeometryEncoder;
use tinymvt::tag::{TagsEncoder, Value};
use tinymvt::vector_tile::{
    tile::{Feature, GeomType, Layer},
    Tile,
};

#[test]
fn make_tile() {
    let extent = 4096;

    let layer = {
        let mut tags_enc = TagsEncoder::new();

        let points_feature = {
            let mut geom_enc = GeometryEncoder::new();
            geom_enc.add_points([]); // empty
            geom_enc.add_points([
                [300, 300],
                [300, extent - 300],
                [extent - 300, extent - 300],
                [extent - 300, 300],
                [900, 900],
                [900, extent - 900],
                [extent - 900, extent - 900],
                [extent - 900, 900],
            ]);

            tags_enc.add("foo", 10);
            tags_enc.add("bar", 20.5);

            Feature {
                id: Some(1),
                tags: tags_enc.take_tags(),
                r#type: Some(GeomType::Point as i32),
                geometry: geom_enc.into_vec(),
            }
        };

        let line_strings_feature = {
            let mut geom_enc = GeometryEncoder::default();
            geom_enc.add_linestring([]); // empty
            geom_enc.add_linestring([[0, 0], [0, extent], [extent, extent], [extent, 0]]);
            geom_enc.add_linestring([
                [500, 500],
                [500, extent - 500],
                [extent - 500, extent - 500],
                [extent - 500, 500],
            ]);
            geom_enc.add_linestring([
                [700, 700],
                [700, extent - 700],
                [extent - 700, extent - 700],
                [extent - 700, 700],
            ]);

            tags_enc.add("uint", Value::Uint(10));
            tags_enc.add("sint", Value::SInt(-10));
            tags_enc.add("int", Value::Int(10));
            tags_enc.add("string", Value::String("string".to_string()));
            tags_enc.add("float", 10.5f32);
            tags_enc.add("double", 10.5f64);
            tags_enc.add("bool", Value::Bool(true));

            Feature {
                id: Some(2),
                tags: tags_enc.take_tags(),
                r#type: Some(GeomType::Linestring as i32),
                geometry: geom_enc.into_vec(),
            }
        };

        let polygon_feature = {
            let mut geom_enc = GeometryEncoder::new();
            geom_enc.add_ring([]); // empty

            geom_enc.add_ring([[1000, 1000], [1000, 1500], [1500, 1500], [1500, 1000]]);
            geom_enc.add_ring([[1100, 1100], [1200, 1100], [1200, 1200], [1100, 1200]]);
            geom_enc.add_ring([[1200, 1200], [1300, 1200], [1300, 1300], [1200, 1300]]);

            geom_enc.add_ring([[2000, 2000], [2000, 2500], [2500, 2500], [2500, 2000]]);
            geom_enc.add_ring([[2100, 2100], [2200, 2100], [2200, 2200], [2100, 2200]]);
            geom_enc.add_ring([[2200, 2200], [2300, 2200], [2300, 2300], [2200, 2300]]);
            geom_enc.add_ring([[2300, 2300], [2400, 2300], [2400, 2400], [2300, 2400]]);

            tags_enc.add("fizz", 10);
            tags_enc.add("buzz", 20.5);

            Feature {
                id: Some(3),
                tags: tags_enc.take_tags(),
                r#type: Some(GeomType::Polygon as i32),
                geometry: geom_enc.into_vec(),
            }
        };

        let (keys, values) = tags_enc.into_keys_and_values();

        Layer {
            version: 2,
            name: "road".to_string(),
            features: vec![points_feature, line_strings_feature, polygon_feature],
            keys,
            values,
            extent: Some(4096),
        }
    };

    let buf = Tile {
        layers: vec![layer],
    }
    .encode_to_vec();

    let expected = fs::read("tests/fixtures/make_tile.pbf").unwrap();

    assert_eq!(buf, expected);
}
