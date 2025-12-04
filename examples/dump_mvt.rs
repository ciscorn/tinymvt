use std::env;
use std::fs;

use prost::Message;
use std::convert::TryFrom;
use tinymvt::geometry::GeometryDecoder;
use tinymvt::tag::TagsDecoder;
use tinymvt::vector_tile::{tile::GeomType, Tile};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <path-to-mvt-file>", args[0]);
        std::process::exit(1);
    }

    let path = &args[1];
    let data = fs::read(path).expect("Failed to read MVT file");

    let tile = Tile::decode(&data[..]).expect("Failed to decode MVT protobuf");

    println!("layers={}", tile.layers.len());

    for layer in tile.layers.iter() {
        println!("layer.name={}", layer.name);
        println!("layer.version={}", layer.version);
        println!("layer.extent={}", layer.extent.unwrap_or(4096));
        println!("layer.features={}", layer.features.len());
        println!("layer.keys={}", layer.keys.len());
        println!("layer.values={}", layer.values.len());

        let tags_decoder = TagsDecoder::new(&layer.keys, &layer.values);

        for feature in layer.features.iter() {
            if let Some(id) = feature.id {
                println!("feature.id={}", id);
            }

            let geom_type = feature.r#type.and_then(|t| GeomType::try_from(t).ok());
            println!("feature.type={:?}", geom_type.unwrap_or(GeomType::Unknown));

            if !feature.tags.is_empty() {
                match tags_decoder.decode(&feature.tags) {
                    Ok(tags) => {
                        for (key, value) in tags {
                            println!("feature.tag.{}={}", key, format_value(&value));
                        }
                    }
                    Err(e) => {
                        println!("feature.tags.error={}", e);
                    }
                }
            }

            if !feature.geometry.is_empty() {
                match geom_type {
                    Some(GeomType::Point) => {
                        let mut decoder = GeometryDecoder::new(&feature.geometry);
                        match decoder.decode_points() {
                            Ok(points) => {
                                for point in points.iter() {
                                    println!("feature.point={},{}", point[0], point[1]);
                                }
                            }
                            Err(e) => println!("feature.geometry.error={}", e),
                        }
                    }
                    Some(GeomType::Linestring) => {
                        let mut decoder = GeometryDecoder::new(&feature.geometry);
                        match decoder.decode_linestrings() {
                            Ok(linestrings) => {
                                for ls in linestrings.iter() {
                                    for vertex in ls.iter() {
                                        println!(
                                            "feature.linestring.vertex={},{}",
                                            vertex[0], vertex[1]
                                        );
                                    }
                                }
                            }
                            Err(e) => println!("feature.geometry.error={}", e),
                        }
                    }
                    Some(GeomType::Polygon) => {
                        let mut decoder = GeometryDecoder::new(&feature.geometry);
                        match decoder.decode_polygons() {
                            Ok(polygons) => {
                                for polygon in polygons.iter() {
                                    for (ring_idx, ring) in polygon.iter().enumerate() {
                                        let ring_type = if ring_idx == 0 {
                                            "exterior"
                                        } else {
                                            "interior"
                                        };
                                        for vertex in ring.iter() {
                                            println!(
                                                "feature.polygon.{}.vertex={},{}",
                                                ring_type, vertex[0], vertex[1]
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => println!("feature.geometry.error={}", e),
                        }
                    }
                    _ => {
                        println!("feature.geometry.raw={}", feature.geometry.len());
                    }
                }
            }
        }
    }
}

fn format_value(value: &tinymvt::tag::Value) -> String {
    use tinymvt::tag::Value;
    match value {
        Value::String(s) => format!("\"{}\"", s),
        Value::Float(bytes) => format!("{}", f32::from_ne_bytes(*bytes)),
        Value::Double(bytes) => format!("{}", f64::from_ne_bytes(*bytes)),
        Value::Int(i) => format!("{}", i),
        Value::Uint(u) => format!("{}", u),
        Value::SInt(s) => format!("{}", s),
        Value::Bool(b) => format!("{}", b),
    }
}
