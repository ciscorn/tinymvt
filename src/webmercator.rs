//! Web Mercator projection utilities.

use std::f64::consts::{FRAC_PI_2, TAU};

use crate::TileZXY;

const A: f64 = 6378137.;
const CIRCUMFERENCE: f64 = A * TAU;

/// Converts geographic coordinate (lng, lat) to Web Mercator coordinate (mx, my) normalized.
///
/// The range of (mx, my) is [0.0, 0.0]-[1.0, 1.0] (same as Mapbox/MapLibre API)
#[inline]
pub fn lnglat_to_web_mercator(lng: f64, lat: f64) -> (f64, f64) {
    let mx = (lng + 180.0) / 360.0;
    let my = ((90.0 + lat).to_radians() / 2.0).tan().ln().to_degrees();
    let my = (-my + 180.0) / 360.0;
    (mx, my)
}

/// Converts Web Mercator coordinate (mx, my) normalized to geographic coordinate (lng, lat).
///
/// The range of (mx, my) is [0.0, 0.0]-[1.0, 1.0] (same as Mapbox/MapLibre API)
#[inline]
pub fn web_mercator_to_lnglat(mx: f64, my: f64) -> (f64, f64) {
    let lng = mx * 360.0 - 180.0;
    let lat = my * 360.0 - 180.0;
    let lat = -(2.0 * (lat.to_radians().exp()).atan() - FRAC_PI_2).to_degrees();
    (lng, lat)
}

/// Converts geographic coordinate (lng, lat) to Web Mercator coordinate (mx, my) in meters.
///
/// The range of (mx, my) is [-20037508.342789244, -20037508.342789244, 20037508.342789244, 20037508.342789244]
#[inline]
pub fn lnglat_to_web_mercator_meters(lng: f64, lat: f64) -> (f64, f64) {
    let mx = lng / 360.0 * CIRCUMFERENCE;
    let my = ((90.0 + lat).to_radians() / 2.0).tan().ln() * A;
    (mx, my)
}

/// Converts Web Mercator coordinate (mx, my) in meters to geographic coordinate (lng, lat).
///
/// The range of (mx, my) is [-20037508.342789244, -20037508.342789244, 20037508.342789244, 20037508.342789244]
#[inline]
pub fn web_mercator_meters_to_lnglat(mx: f64, my: f64) -> (f64, f64) {
    let lng = mx / CIRCUMFERENCE * 360.0;
    let lat = (2.0 * (my / A).exp().atan()).to_degrees() - 90.0;
    (lng, lat)
}

/// Calculates the tile coordinates (z, x, y) from the zoom level and Web Mercator coordinates (mx, my).
///
/// The range of (mx, my) is [0.0, 0.0]-[1.0, 1.0] (same as Mapbox/MapLibre API)
#[inline]
pub fn web_mercator_to_zxy(z: u8, mx: f64, my: f64) -> TileZXY {
    let x = (mx * (1 << z) as f64) as u32;
    let y = (my * (1 << z) as f64) as u32;
    (z, x, y)
}

/// Calculates the tile coordinates (z, x, y) from the zoom level and geographic coordinates (lng, lat).
#[inline]
pub fn lnglat_to_zxy(z: u8, lng: f64, lat: f64) -> TileZXY {
    let (mx, my) = lnglat_to_web_mercator(lng, lat);
    web_mercator_to_zxy(z, mx, my)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_normalized() {
        {
            let (lng, lat) = (136.08, 37.39);
            let (mx, my) = lnglat_to_web_mercator(lng, lat);
            let (lng2, lat2) = web_mercator_to_lnglat(mx, my);
            assert!((lng - lng2).abs() < 1e-13);
            assert!((lat - lat2).abs() < 1e-13);
        }
        {
            let (lng, lat) = (0.3, 0.2);
            let (mx, my) = lnglat_to_web_mercator(lng, lat);
            let (lng2, lat2) = web_mercator_to_lnglat(mx, my);
            assert!((lng - lng2).abs() < 1e-13);
            assert!((lat - lat2).abs() < 1e-13);
        }
    }

    #[test]
    fn roundtrip_in_meters() {
        {
            let (lng, lat) = (136.08, 37.39);
            let (mx, my) = lnglat_to_web_mercator_meters(lng, lat);
            let (lng2, lat2) = web_mercator_meters_to_lnglat(mx, my);
            assert!((lng - lng2).abs() < 1e-9);
            assert!((lat - lat2).abs() < 1e-9);
        }
        {
            let (lng, lat) = (0.3, 0.2);
            let (mx, my) = lnglat_to_web_mercator_meters(lng, lat);
            let (lng2, lat2) = web_mercator_meters_to_lnglat(mx, my);
            assert!((lng - lng2).abs() < 1e-9);
            assert!((lat - lat2).abs() < 1e-9);
        }
    }

    #[test]
    fn null_island() {
        // https://en.wikipedia.org/wiki/Null_Island
        // (lng: 0, lat: 0) -> (mx: 0.5, my: 0.5)
        let (lng, lat) = (0., 0.);
        let (mx, my) = lnglat_to_web_mercator(lng, lat);
        assert!((mx - 0.5).abs() < 1e-10);
        assert!((my - 0.5).abs() < 1e-10);
    }

    #[test]
    fn null_island_in_meters() {
        // https://en.wikipedia.org/wiki/Null_Island
        // (lng: 0, lat: 0) -> (mx: 0.5, my: 0.5)
        let (lng, lat) = (0., 0.);
        let (mx, my) = lnglat_to_web_mercator_meters(lng, lat);
        println!("{}, {}", mx, my);
        assert!((mx - 0.0).abs() < 1e-9);
        assert!((my - 0.0).abs() < 1e-9);
    }

    #[test]
    fn bound_in_meters() {
        let (lng, lat) = (180., 85.0511287798066);
        let (mx, my) = lnglat_to_web_mercator_meters(lng, lat);
        println!("{}", CIRCUMFERENCE / 2.);
        assert!((mx - CIRCUMFERENCE / 2.).abs() < 1e-7);
        assert!((my - CIRCUMFERENCE / 2.).abs() < 1e-7);
    }

    #[test]
    fn test_lnglat_to_zxy() {
        assert_eq!(
            lnglat_to_zxy(13, 138.28421421786732, 37.153461188900344),
            (13, 7242, 3184)
        );
        assert_eq!(
            lnglat_to_zxy(13, 138.322514014752, 37.108119251859506),
            (13, 7243, 3185)
        );
        assert_eq!(web_mercator_to_zxy(13, 0., 0.), (13, 0, 0));
        assert_eq!(web_mercator_to_zxy(13, 0.5, 1.), (13, 1 << 12, 1 << 13));
        assert_eq!(web_mercator_to_zxy(13, 1., 0.5), (13, 1 << 13, 1 << 12));
    }
}
