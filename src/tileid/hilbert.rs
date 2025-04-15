//! Tile ID based on Hilbert curve (compliant with PMTiles)

pub fn hilbert_to_zxy(id: u64) -> (u8, u32, u32) {
    let z = (((u64::BITS - (3 * id + 1).leading_zeros()) - 1) / 2) as u8;
    let acc = ((1 << (z * 2)) - 1) / 3;
    let mut pos = id - acc;
    let (x, y) = (0..z).fold((0, 0), |(x, y), a| {
        let s = 1u32 << a;
        let rx = s & (pos as u32 >> 1);
        let ry = s & (pos as u32 ^ rx);
        let (x, y) = rotate(s, x, y, rx, ry);
        pos >>= 1;
        (x + rx, y + ry)
    });
    (z, x, y)
}

pub fn zxy_to_hilbert(z: u8, mut x: u32, mut y: u32) -> u64 {
    let acc = ((1 << (z * 2)) - 1) / 3;
    (0..z).rev().fold(acc, |acc, a| {
        let s = 1 << a;
        let rx = s & x;
        let ry = s & y;
        (x, y) = rotate(s, x, y, rx, ry);
        acc + ((((3 * rx) ^ ry) as u64) << a)
    })
}

const fn rotate(n: u32, mut x: u32, mut y: u32, rx: u32, ry: u32) -> (u32, u32) {
    if ry == 0 {
        if rx != 0 {
            x = (n - 1).wrapping_sub(x);
            y = (n - 1).wrapping_sub(y);
        }
        (x, y) = (y, x)
    }
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let fixture = vec![
            // ((x, y, z), expected_tile_id)
            //
            // z = 0
            ((0, 0, 0), 0),
            // z = 1
            ((1, 0, 0), 1),
            ((1, 0, 1), 2),
            ((1, 1, 1), 3),
            ((1, 1, 0), 4),
            // z = 2
            ((2, 0, 1), 8),
            ((2, 1, 1), 7),
            ((2, 2, 0), 19),
            ((2, 3, 3), 15),
            ((2, 3, 2), 16),
            // z= 3
            ((3, 0, 0), 21),
            ((3, 7, 0), 84),
            // z = 4
            ((4, 0, 0), 85),
            ((4, 15, 0), 340),
            // z = 18 (tileId exceeds u32)
            ((18, 1, 1), 22906492247),
            // z = 31
            ((31, 100, 100), 1537228672809139573),
        ];

        for ((x, y, z), expected_tile_id) in fixture {
            let tile_id = zxy_to_hilbert(x, y, z);
            assert_eq!(tile_id, expected_tile_id);
            assert_eq!(hilbert_to_zxy(tile_id), (x, y, z));
        }
    }
}
