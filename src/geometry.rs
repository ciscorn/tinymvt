//! Geometry encoder for MVT.

const GEOM_COMMAND_MOVE_TO: u32 = 1;
const GEOM_COMMAND_LINE_TO: u32 = 2;
const GEOM_COMMAND_CLOSE_PATH: u32 = 7;

const GEOM_COMMAND_MOVE_TO_WITH_COUNT1: u32 = 1 << 3 | GEOM_COMMAND_MOVE_TO;
const GEOM_COMMAND_CLOSE_PATH_WITH_COUNT1: u32 = 1 << 3 | GEOM_COMMAND_CLOSE_PATH;

/// Utility for encoding MVT geometries.
pub struct GeometryEncoder {
    buf: Vec<u32>,
    prev_x: i16,
    prev_y: i16,
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
    pub fn add_points(&mut self, iterable: impl IntoIterator<Item = [i16; 2]>) {
        let mut iter = iterable.into_iter();
        let Some([first_x, first_y]) = iter.next() else {
            return;
        };
        let dx = (first_x - self.prev_x) as i32;
        let dy = (first_y - self.prev_y) as i32;
        (self.prev_x, self.prev_y) = (first_x, first_y);

        // move to
        let moveto_cmd_pos = self.buf.len();
        self.buf
            .extend([GEOM_COMMAND_MOVE_TO_WITH_COUNT1, zigzag(dx), zigzag(dy)]);

        let mut count = 1;
        for [x, y] in iter {
            let dx = (x - self.prev_x) as i32;
            let dy = (y - self.prev_y) as i32;
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
    pub fn add_linestring(&mut self, iterable: impl IntoIterator<Item = [i16; 2]>) {
        self.add_path(iterable, false)
    }

    /// Adds a polygon ring.
    ///
    /// A polygon consists of one exterior ring (clockwise) and optionally one or more interior rings (counter-clockwise).
    pub fn add_ring(&mut self, iterable: impl IntoIterator<Item = [i16; 2]>) {
        self.add_path(iterable, true)
    }

    /// Adds a path (line string or polygon ring).
    fn add_path(&mut self, iterable: impl IntoIterator<Item = [i16; 2]>, close: bool) {
        let mut iter = iterable.into_iter();
        let Some([first_x, first_y]) = iter.next() else {
            return;
        };
        let dx = (first_x - self.prev_x) as i32;
        let dy = (first_y - self.prev_y) as i32;
        (self.prev_x, self.prev_y) = (first_x, first_y);

        // move to
        self.buf
            .extend([GEOM_COMMAND_MOVE_TO_WITH_COUNT1, zigzag(dx), zigzag(dy)]);

        // line to
        let lineto_cmd_pos = self.buf.len();
        self.buf.push(GEOM_COMMAND_LINE_TO); // length will be set later
        let mut count = 0;
        for [x, y] in iter {
            let dx = (x - self.prev_x) as i32;
            let dy = (y - self.prev_y) as i32;
            (self.prev_x, self.prev_y) = (x, y);
            if dx != 0 || dy != 0 {
                self.buf.extend([zigzag(dx), zigzag(dy)]);
                count += 1;
            }
        }
        debug_assert!(count >= 2);

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

/// zig-zag encoding
///
/// See: https://protobuf.dev/programming-guides/encoding/#signed-ints
#[inline]
fn zigzag(v: i32) -> u32 {
    ((v << 1) ^ (v >> 31)) as u32
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
}
