# tinymvt-rs

[![codecov](https://codecov.io/gh/ciscorn/tinymvt/graph/badge.svg?token=HSPd9MRmxC)](https://codecov.io/gh/ciscorn/tinymvt)
![Crates.io Version](https://img.shields.io/crates/v/tinymvt)

A lightweight Rust library for encoding and decoding Mapbox Vector Tiles (MVT)

License: MIT

## Features

- Protobuf ([prost](https://github.com/tokio-rs/prost)) data types for MVT
- Geometry encoder and decoder
- Tags encoder and decoder
- Conversion between Web Mercator and geographic coordinates
- Conversion between linear tile IDs (PMTiles-compliant Hilbert IDs) and XYZ tile IDs
