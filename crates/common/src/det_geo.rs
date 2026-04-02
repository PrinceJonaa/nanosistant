//! Geographic deterministic functions — coordinates, distances, bounding boxes.

use serde::Serialize;

const EARTH_RADIUS_KM: f64 = 6371.0;

/// Geographic coordinate.
#[derive(Debug, Clone, Serialize)]
pub struct Coord {
    pub lat: f64,
    pub lon: f64,
}

/// Haversine distance between two coordinates in kilometers.
#[must_use]
pub fn haversine_km(a: &Coord, b: &Coord) -> f64 {
    let dlat = (b.lat - a.lat).to_radians();
    let dlon = (b.lon - a.lon).to_radians();
    let lat_a = a.lat.to_radians();
    let lat_b = b.lat.to_radians();
    let h = (dlat / 2.0).sin().powi(2)
        + lat_a.cos() * lat_b.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * h.sqrt().asin();
    (EARTH_RADIUS_KM * c * 100.0).round() / 100.0
}

/// Miles from km.
#[must_use]
pub fn km_to_miles(km: f64) -> f64 { (km * 0.621371 * 100.0).round() / 100.0 }

/// Midpoint between two coordinates.
#[must_use]
pub fn midpoint(a: &Coord, b: &Coord) -> Coord {
    Coord {
        lat: (a.lat + b.lat) / 2.0,
        lon: (a.lon + b.lon) / 2.0,
    }
}

/// Bounding box around a set of coordinates.
#[derive(Debug, Serialize)]
pub struct BoundingBox {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lon: f64,
    pub max_lon: f64,
}

#[must_use]
pub fn bounding_box(coords: &[Coord]) -> Option<BoundingBox> {
    if coords.is_empty() { return None; }
    Some(BoundingBox {
        min_lat: coords.iter().map(|c| c.lat).fold(f64::INFINITY, f64::min),
        max_lat: coords.iter().map(|c| c.lat).fold(f64::NEG_INFINITY, f64::max),
        min_lon: coords.iter().map(|c| c.lon).fold(f64::INFINITY, f64::min),
        max_lon: coords.iter().map(|c| c.lon).fold(f64::NEG_INFINITY, f64::max),
    })
}

/// Decimal degrees to Degrees/Minutes/Seconds string.
#[must_use]
pub fn dd_to_dms(decimal: f64, is_lat: bool) -> String {
    let abs = decimal.abs();
    let deg = abs.floor() as u32;
    let min_full = (abs - deg as f64) * 60.0;
    let min = min_full.floor() as u32;
    let sec = (min_full - min as f64) * 60.0;
    let dir = if is_lat { if decimal >= 0.0 { "N" } else { "S" } }
              else      { if decimal >= 0.0 { "E" } else { "W" } };
    format!("{deg}°{min}'{sec:.1}\"{dir}")
}

/// DMS string to decimal degrees.
pub fn dms_to_dd(dms: &str) -> Result<f64, String> {
    let s = dms.trim();
    let dir = s.chars().last().unwrap_or('N');
    let sign = if matches!(dir, 'S' | 'W') { -1.0 } else { 1.0 };
    let stripped: String = s.chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '°' || *c == '\'' || *c == '"')
        .collect();
    let parts: Vec<&str> = stripped.split(['°', '\'', '"'].as_ref()).collect();
    let deg: f64 = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0.0);
    let min: f64 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0.0);
    let sec: f64 = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0.0);
    Ok(sign * (deg + min / 60.0 + sec / 3600.0))
}

/// Initial bearing (forward azimuth) from a to b, in degrees (0-360).
#[must_use]
pub fn bearing(a: &Coord, b: &Coord) -> f64 {
    let lat_a = a.lat.to_radians();
    let lat_b = b.lat.to_radians();
    let dlon = (b.lon - a.lon).to_radians();
    let y = dlon.sin() * lat_b.cos();
    let x = lat_a.cos() * lat_b.sin() - lat_a.sin() * lat_b.cos() * dlon.cos();
    let bearing = y.atan2(x).to_degrees();
    (bearing + 360.0) % 360.0
}

/// Rough timezone offset (hours) from longitude.
#[must_use]
pub fn lon_to_utc_offset(lon: f64) -> f64 {
    (lon / 15.0 * 2.0).round() / 2.0  // nearest 0.5hr
}

/// Check if a coordinate is within a bounding box.
#[must_use]
pub fn point_in_bbox(coord: &Coord, bbox: &BoundingBox) -> bool {
    coord.lat >= bbox.min_lat && coord.lat <= bbox.max_lat
        && coord.lon >= bbox.min_lon && coord.lon <= bbox.max_lon
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn haversine_nyc_la() {
        let nyc = Coord { lat: 40.7128, lon: -74.0060 };
        let la  = Coord { lat: 34.0522, lon: -118.2437 };
        let d = haversine_km(&nyc, &la);
        assert!((d - 3940.0).abs() < 50.0, "distance: {d}");
    }
    #[test] fn midpoint_works() {
        let a = Coord { lat: 0.0, lon: 0.0 };
        let b = Coord { lat: 10.0, lon: 20.0 };
        let m = midpoint(&a, &b);
        assert!((m.lat - 5.0).abs() < 0.001);
        assert!((m.lon - 10.0).abs() < 0.001);
    }
    #[test] fn bounding_box_works() {
        let coords = vec![
            Coord { lat: 10.0, lon: 20.0 },
            Coord { lat: 30.0, lon: 40.0 },
        ];
        let bb = bounding_box(&coords).unwrap();
        assert_eq!(bb.min_lat, 10.0);
        assert_eq!(bb.max_lon, 40.0);
    }
    #[test] fn dd_to_dms_north() {
        let s = dd_to_dms(40.7128, true);
        assert!(s.contains('N'), "got: {s}");
    }
    #[test] fn km_to_miles_correct() {
        assert!((km_to_miles(1.609344) - 1.0).abs() < 0.01);
    }
}
