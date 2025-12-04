//! Geometry encoder for MVT.

const GEOM_COMMAND_MOVE_TO: u32 = 1;
const GEOM_COMMAND_LINE_TO: u32 = 2;
const GEOM_COMMAND_CLOSE_PATH: u32 = 7;

const GEOM_COMMAND_MOVE_TO_WITH_COUNT1: u32 = 1 << 3 | GEOM_COMMAND_MOVE_TO;
const GEOM_COMMAND_CLOSE_PATH_WITH_COUNT1: u32 = 1 << 3 | GEOM_COMMAND_CLOSE_PATH;

/// Utility for encoding MVT geometries.
pub struct GeometryEncoder {
    buf: Vec<u32>,
    prev_x: i32,
    prev_y: i32,
}

impl GeometryEncoder {
    pub fn new() -> Self {
        // TODO: with_capacity?
        Self {
            buf: Vec::new(),
            prev_x: 0,
            prev_y: 0,
        }
    }

    /// Consumes the encoder and returns the encoded geometry.
    #[inline]
    pub fn into_vec(self) -> Vec<u32> {
        self.buf
    }

    /// Adds points.
    pub fn add_points(&mut self, iterable: impl IntoIterator<Item = [i32; 2]>) {
        let mut iter = iterable.into_iter();
        let Some([first_x, first_y]) = iter.next() else {
            return;
        };
        let dx = first_x - self.prev_x;
        let dy = first_y - self.prev_y;
        (self.prev_x, self.prev_y) = (first_x, first_y);

        // move to
        let moveto_cmd_pos = self.buf.len();
        self.buf
            .extend([GEOM_COMMAND_MOVE_TO_WITH_COUNT1, zigzag(dx), zigzag(dy)]);

        let mut count = 1;
        for [x, y] in iter {
            let dx = x - self.prev_x;
            let dy = y - self.prev_y;
            (self.prev_x, self.prev_y) = (x, y);
            if dx != 0 || dy != 0 {
                self.buf.extend([zigzag(dx), zigzag(dy)]);
                count += 1;
            }
        }

        // set length
        self.buf[moveto_cmd_pos] = GEOM_COMMAND_MOVE_TO | count << 3;
    }

    /// Adds a line string.
    pub fn add_linestring(&mut self, iterable: impl IntoIterator<Item = [i32; 2]>) {
        self.add_path(iterable, false)
    }

    /// Adds a polygon ring.
    ///
    /// A polygon consists of one exterior ring (clockwise) and optionally one or more interior rings (counter-clockwise).
    pub fn add_ring(&mut self, iterable: impl IntoIterator<Item = [i32; 2]>) {
        self.add_path(iterable, true)
    }

    /// Adds a path (line string or polygon ring).
    fn add_path(&mut self, iterable: impl IntoIterator<Item = [i32; 2]>, close: bool) {
        let mut iter = iterable.into_iter();
        let Some([first_x, first_y]) = iter.next() else {
            return;
        };
        let dx = first_x - self.prev_x;
        let dy = first_y - self.prev_y;
        (self.prev_x, self.prev_y) = (first_x, first_y);

        // move to
        self.buf
            .extend([GEOM_COMMAND_MOVE_TO_WITH_COUNT1, zigzag(dx), zigzag(dy)]);

        // line to
        let lineto_cmd_pos = self.buf.len();
        self.buf.push(GEOM_COMMAND_LINE_TO); // length will be set later
        let mut count = 0;
        for [x, y] in iter {
            let dx = x - self.prev_x;
            let dy = y - self.prev_y;
            (self.prev_x, self.prev_y) = (x, y);
            // avoid zero-length segments, in low zoom levels this can happen frequently
            if dx != 0 || dy != 0 {
                self.buf.extend([zigzag(dx), zigzag(dy)]);
                count += 1;
            }
        }
        // if line string has only one point (due to simplification), repeat it
        if count == 0 {
            self.buf.extend([0, 0]);
            count += 1;
        }

        // set length
        self.buf[lineto_cmd_pos] = GEOM_COMMAND_LINE_TO | count << 3;

        if close {
            // close path
            self.buf.push(GEOM_COMMAND_CLOSE_PATH_WITH_COUNT1);
        }
    }
}

impl Default for GeometryEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Decoded geometry types from MVT.
#[derive(Debug, Clone, PartialEq)]
pub enum DecodedGeometry {
    /// Point geometries (multiple points).
    Points(Vec<[i32; 2]>),
    /// LineString geometries (multiple linestrings).
    LineStrings(Vec<Vec<[i32; 2]>>),
    /// Polygon geometries (multiple polygons, each with rings).
    Polygons(Vec<Vec<Vec<[i32; 2]>>>),
}

/// Alias for DecodedGeometry for convenience.
pub type Geometry = DecodedGeometry;

/// Utility for decoding MVT geometries.
pub struct GeometryDecoder<'a> {
    buf: &'a [u32],
    pos: usize,
    cursor_x: i32,
    cursor_y: i32,
}

impl<'a> GeometryDecoder<'a> {
    /// Creates a new decoder for the given geometry buffer.
    pub fn new(buf: &'a [u32]) -> Self {
        Self {
            buf,
            pos: 0,
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    /// Decodes points from the geometry buffer.
    pub fn decode_points(&mut self) -> Result<Vec<[i32; 2]>, String> {
        let mut points = Vec::new();

        while self.pos < self.buf.len() {
            let cmd_int = self.buf[self.pos];
            self.pos += 1;

            let cmd = cmd_int & 0x7;
            let count = (cmd_int >> 3) as usize;

            match cmd {
                GEOM_COMMAND_MOVE_TO => {
                    for _ in 0..count {
                        let coord = self.read_coord()?;
                        points.push(coord);
                    }
                }
                _ => {
                    return Err(format!("Unexpected command {} in point geometry", cmd));
                }
            }
        }

        Ok(points)
    }

    /// Decodes linestrings from the geometry buffer.
    pub fn decode_linestrings(&mut self) -> Result<Vec<Vec<[i32; 2]>>, String> {
        let mut linestrings = Vec::new();

        while self.pos < self.buf.len() {
            let mut linestring = Vec::new();

            // MoveTo command
            let cmd_int = self.buf[self.pos];
            self.pos += 1;

            let cmd = cmd_int & 0x7;
            let count = (cmd_int >> 3) as usize;

            if cmd != GEOM_COMMAND_MOVE_TO {
                return Err(format!("Expected MoveTo command, got {}", cmd));
            }

            if count != 1 {
                return Err(format!(
                    "MoveTo count must be 1 for linestrings, got {}",
                    count
                ));
            }

            let coord = self.read_coord()?;
            linestring.push(coord);

            // LineTo command
            if self.pos >= self.buf.len() {
                return Err("Unexpected end of buffer after MoveTo".to_string());
            }

            let cmd_int = self.buf[self.pos];
            self.pos += 1;

            let cmd = cmd_int & 0x7;
            let count = (cmd_int >> 3) as usize;

            if cmd != GEOM_COMMAND_LINE_TO {
                return Err(format!("Expected LineTo command, got {}", cmd));
            }

            for _ in 0..count {
                let coord = self.read_coord()?;
                linestring.push(coord);
            }

            linestrings.push(linestring);
        }

        Ok(linestrings)
    }

    /// Decodes polygons from the geometry buffer.
    pub fn decode_polygons(&mut self) -> Result<Vec<Vec<Vec<[i32; 2]>>>, String> {
        let mut polygons: Vec<Vec<Vec<[i32; 2]>>> = Vec::new();
        let mut current_polygon: Vec<Vec<[i32; 2]>> = Vec::new();

        while self.pos < self.buf.len() {
            let mut ring = Vec::new();

            // MoveTo command
            let cmd_int = self.buf[self.pos];
            self.pos += 1;

            let cmd = cmd_int & 0x7;
            let count = (cmd_int >> 3) as usize;

            if cmd != GEOM_COMMAND_MOVE_TO {
                return Err(format!("Expected MoveTo command, got {}", cmd));
            }

            if count != 1 {
                return Err(format!(
                    "MoveTo count must be 1 for polygons, got {}",
                    count
                ));
            }

            let coord = self.read_coord()?;
            ring.push(coord);

            // LineTo command
            if self.pos >= self.buf.len() {
                return Err("Unexpected end of buffer after MoveTo".to_string());
            }

            let cmd_int = self.buf[self.pos];
            self.pos += 1;

            let cmd = cmd_int & 0x7;
            let count = (cmd_int >> 3) as usize;

            if cmd != GEOM_COMMAND_LINE_TO {
                return Err(format!("Expected LineTo command, got {}", cmd));
            }

            for _ in 0..count {
                let coord = self.read_coord()?;
                ring.push(coord);
            }

            // ClosePath command
            if self.pos >= self.buf.len() {
                return Err("Unexpected end of buffer after LineTo".to_string());
            }

            let cmd_int = self.buf[self.pos];
            self.pos += 1;

            let cmd = cmd_int & 0x7;

            if cmd != GEOM_COMMAND_CLOSE_PATH {
                return Err(format!("Expected ClosePath command, got {}", cmd));
            }

            // Calculate signed area to determine if this is an exterior or interior ring
            // MVT spec: exterior rings are clockwise (positive area), interior rings are counter-clockwise (negative area)
            let signed_area = calculate_signed_area(&ring);
            let is_exterior = signed_area > 0.0;

            // If this is an exterior ring and we already have rings, finalize the current polygon
            if is_exterior && !current_polygon.is_empty() {
                polygons.push(current_polygon);
                current_polygon = Vec::new();
            }

            current_polygon.push(ring);
        }

        // Finalize any remaining polygon
        if !current_polygon.is_empty() {
            polygons.push(current_polygon);
        }

        Ok(polygons)
    }

    /// Reads a coordinate pair from the buffer.
    fn read_coord(&mut self) -> Result<[i32; 2], String> {
        if self.pos + 1 >= self.buf.len() {
            return Err("Unexpected end of buffer while reading coordinates".to_string());
        }

        let dx = unzigzag(self.buf[self.pos]);
        let dy = unzigzag(self.buf[self.pos + 1]);
        self.pos += 2;

        self.cursor_x += dx;
        self.cursor_y += dy;

        Ok([self.cursor_x, self.cursor_y])
    }
}

/// Calculates the signed area of a ring using the shoelace formula
/// Positive area means clockwise (exterior ring), negative means counter-clockwise (interior ring)
fn calculate_signed_area(ring: &[[i32; 2]]) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }

    let mut area = 0i64;
    for i in 0..ring.len() {
        let j = (i + 1) % ring.len();
        area += ring[i][0] as i64 * ring[j][1] as i64;
        area -= ring[j][0] as i64 * ring[i][1] as i64;
    }
    area as f64 / 2.0
}

/// zig-zag encoding
///
/// See: https://protobuf.dev/programming-guides/encoding/#signed-ints
#[inline]
fn zigzag(v: i32) -> u32 {
    ((v << 1) ^ (v >> 31)) as u32
}

/// zig-zag decoding
#[inline]
fn unzigzag(v: u32) -> i32 {
    ((v >> 1) as i32) ^ (-((v & 1) as i32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zigzag() {
        assert_eq!(zigzag(0), 0);
        assert_eq!(zigzag(-1), 1);
        assert_eq!(zigzag(1), 2);
        assert_eq!(zigzag(-2), 3);
        assert_eq!(zigzag(2), 4);
        assert_eq!(zigzag(4096), 8192);
        assert_eq!(zigzag(-4096), 8191);
    }

    #[test]
    fn test_linestring_with_two_vertices() {
        // Test that linestrings with exactly 2 vertices work correctly
        let mut encoder = GeometryEncoder::new();
        encoder.add_linestring([[0, 0], [10, 10]]);
        let geometry = encoder.into_vec();

        // Expected: MoveTo(1) + coords(2) + LineTo(1) + coords(2)
        assert_eq!(geometry.len(), 6);
        assert_eq!(geometry[0], GEOM_COMMAND_MOVE_TO_WITH_COUNT1); // MoveTo with count=1
        assert_eq!(geometry[1], zigzag(0)); // dx = 0
        assert_eq!(geometry[2], zigzag(0)); // dy = 0
        assert_eq!(geometry[3], GEOM_COMMAND_LINE_TO | (1 << 3)); // LineTo with count=1
        assert_eq!(geometry[4], zigzag(10)); // dx = 10
        assert_eq!(geometry[5], zigzag(10)); // dy = 10
    }

    #[test]
    fn test_linestring_with_duplicate_points_filtered() {
        // Test that duplicate consecutive points are filtered out
        // This simulates what happens at low zoom levels
        let mut encoder = GeometryEncoder::new();
        encoder.add_linestring([[0, 0], [0, 0], [0, 0]]);
        let geometry = encoder.into_vec();

        // Expected: MoveTo(1) + coords(2) + LineTo(1) + coords(2) [zero-length segment]
        assert_eq!(geometry.len(), 6);
        assert_eq!(geometry[0], GEOM_COMMAND_MOVE_TO_WITH_COUNT1);
        assert_eq!(geometry[3], GEOM_COMMAND_LINE_TO | (1 << 3)); // LineTo with count=1
        assert_eq!(geometry[4], 0); // dx = 0 (repeated point)
        assert_eq!(geometry[5], 0); // dy = 0 (repeated point)
    }

    #[test]
    fn test_unzigzag() {
        assert_eq!(unzigzag(0), 0);
        assert_eq!(unzigzag(1), -1);
        assert_eq!(unzigzag(2), 1);
        assert_eq!(unzigzag(3), -2);
        assert_eq!(unzigzag(4), 2);
        assert_eq!(unzigzag(8192), 4096);
        assert_eq!(unzigzag(8191), -4096);
    }

    #[test]
    fn test_zigzag_roundtrip() {
        for v in [-4096, -100, -1, 0, 1, 100, 4096] {
            assert_eq!(unzigzag(zigzag(v)), v);
        }
    }

    #[test]
    fn test_decode_points() {
        // Encode points
        let mut encoder = GeometryEncoder::new();
        let points = [[10, 20], [30, 40], [50, 60]];
        encoder.add_points(points);
        let geometry = encoder.into_vec();

        // Decode points
        let mut decoder = GeometryDecoder::new(&geometry);
        let decoded = decoder.decode_points().unwrap();

        assert_eq!(decoded, points);
    }

    #[test]
    fn test_decode_single_linestring() {
        // Encode a linestring
        let mut encoder = GeometryEncoder::new();
        let linestring = [[0, 0], [10, 10], [20, 20]];
        encoder.add_linestring(linestring);
        let geometry = encoder.into_vec();

        // Decode linestring
        let mut decoder = GeometryDecoder::new(&geometry);
        let decoded = decoder.decode_linestrings().unwrap();

        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], linestring);
    }

    #[test]
    fn test_decode_multiple_linestrings() {
        // Encode multiple linestrings
        let mut encoder = GeometryEncoder::new();
        let ls1 = [[0, 0], [10, 10]];
        let ls2 = [[100, 100], [110, 110], [120, 120]];
        encoder.add_linestring(ls1);
        encoder.add_linestring(ls2);
        let geometry = encoder.into_vec();

        // Decode linestrings
        let mut decoder = GeometryDecoder::new(&geometry);
        let decoded = decoder.decode_linestrings().unwrap();

        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0], ls1);
        assert_eq!(decoded[1], ls2);
    }

    #[test]
    fn test_decode_polygon_single_ring() {
        // Encode a simple polygon (exterior ring only)
        let mut encoder = GeometryEncoder::new();
        let ring = [[0, 0], [100, 0], [100, 100], [0, 100]];
        encoder.add_ring(ring);
        let geometry = encoder.into_vec();

        // Decode polygon
        let mut decoder = GeometryDecoder::new(&geometry);
        let decoded = decoder.decode_polygons().unwrap();

        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].len(), 1);
        assert_eq!(decoded[0][0], ring);
    }

    #[test]
    fn test_decode_polygon_with_holes() {
        // Encode a polygon with holes
        let mut encoder = GeometryEncoder::new();
        // Holes must be counter-clockwise (reverse order)
        let exterior = [[0, 0], [100, 0], [100, 100], [0, 100]]; // clockwise
        let hole1 = [[10, 10], [10, 20], [20, 20], [20, 10]]; // counter-clockwise
        let hole2 = [[30, 30], [30, 40], [40, 40], [40, 30]]; // counter-clockwise
        encoder.add_ring(exterior);
        encoder.add_ring(hole1);
        encoder.add_ring(hole2);
        let geometry = encoder.into_vec();

        // Decode polygon
        let mut decoder = GeometryDecoder::new(&geometry);
        let decoded = decoder.decode_polygons().unwrap();

        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].len(), 3);
        assert_eq!(decoded[0][0], exterior);
        assert_eq!(decoded[0][1], hole1);
        assert_eq!(decoded[0][2], hole2);
    }

    #[test]
    fn test_decode_multiple_polygons() {
        // Encode multiple polygons
        let mut encoder = GeometryEncoder::new();

        // First polygon with hole
        let poly1_ring1 = [[0, 0], [50, 0], [50, 50], [0, 50]]; // clockwise exterior
        let poly1_ring2 = [[10, 10], [10, 20], [20, 20], [20, 10]]; // counter-clockwise hole
        encoder.add_ring(poly1_ring1);
        encoder.add_ring(poly1_ring2);

        // Second polygon (simple)
        let poly2_ring1 = [[100, 100], [150, 100], [150, 150], [100, 150]]; // clockwise exterior
        encoder.add_ring(poly2_ring1);

        let geometry = encoder.into_vec();

        // Decode polygons
        let mut decoder = GeometryDecoder::new(&geometry);
        let decoded = decoder.decode_polygons().unwrap();

        // With proper signed area detection, we should get 2 separate polygons
        // poly1_ring1 is clockwise (exterior), poly1_ring2 is counter-clockwise (hole), poly2_ring1 is clockwise (exterior)
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].len(), 2); // First polygon with 1 exterior + 1 hole
        assert_eq!(decoded[0][0], poly1_ring1);
        assert_eq!(decoded[0][1], poly1_ring2);
        assert_eq!(decoded[1].len(), 1); // Second polygon with just 1 exterior
        assert_eq!(decoded[1][0], poly2_ring1);
    }
}
