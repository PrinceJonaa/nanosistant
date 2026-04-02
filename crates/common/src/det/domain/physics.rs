//! Physics domain deterministic functions — SI units throughout.
//!
//! No units library dependency. All inputs assumed to be SI unless noted.

// ═══════════════════════════════════════
// Constants
// ═══════════════════════════════════════

/// Speed of light in vacuum (m/s).
#[must_use]
pub fn speed_of_light() -> f64 { 299_792_458.0 }

/// Elementary charge (C).
#[must_use]
pub fn electron_charge() -> f64 { 1.602_176_634e-19 }

/// Standard gravity (m/s²).
const G_STD: f64 = 9.806_65;
/// Gravitational constant (m³/(kg·s²)).
const G_NEWTON: f64 = 6.674_30e-11;

// ═══════════════════════════════════════
// Mechanics
// ═══════════════════════════════════════

/// Kinetic energy KE = ½mv².
#[must_use]
pub fn kinetic_energy(mass_kg: f64, velocity_ms: f64) -> f64 {
    0.5 * mass_kg * velocity_ms * velocity_ms
}

/// Gravitational potential energy PE = mgh.
/// Uses standard gravity if `g` ≤ 0.
#[must_use]
pub fn potential_energy(mass_kg: f64, height_m: f64, g: f64) -> f64 {
    let grav = if g <= 0.0 { G_STD } else { g };
    mass_kg * grav * height_m
}

/// Work W = F·d·cos(θ), where θ is in degrees.
#[must_use]
pub fn work_joules(force_n: f64, distance_m: f64, angle_deg: f64) -> f64 {
    force_n * distance_m * angle_deg.to_radians().cos()
}

/// Power P = E / t.
#[must_use]
pub fn power_watts(energy_j: f64, time_s: f64) -> f64 {
    if time_s == 0.0 { return 0.0; }
    energy_j / time_s
}

/// Linear momentum p = mv.
#[must_use]
pub fn momentum(mass_kg: f64, velocity_ms: f64) -> f64 {
    mass_kg * velocity_ms
}

// ═══════════════════════════════════════
// Wave / EM
// ═══════════════════════════════════════

/// Wavelength λ = speed / frequency.
#[must_use]
pub fn frequency_to_wavelength(freq_hz: f64, speed: f64) -> f64 {
    if freq_hz == 0.0 { return 0.0; }
    speed / freq_hz
}

/// Frequency f = speed / wavelength.
#[must_use]
pub fn wavelength_to_frequency(wavelength_m: f64, speed: f64) -> f64 {
    if wavelength_m == 0.0 { return 0.0; }
    speed / wavelength_m
}

/// Newton's law of universal gravitation F = G*m1*m2/r².
#[must_use]
pub fn gravitational_force(m1: f64, m2: f64, r: f64) -> f64 {
    if r == 0.0 { return 0.0; }
    G_NEWTON * m1 * m2 / (r * r)
}

// ═══════════════════════════════════════
// Decibels
// ═══════════════════════════════════════

/// Convert dB to linear power ratio: 10^(dB/10).
#[must_use]
pub fn db_to_linear(db: f64) -> f64 { 10.0_f64.powf(db / 10.0) }

/// Convert linear power ratio to dB: 10 * log10(linear).
#[must_use]
pub fn linear_to_db(linear: f64) -> f64 {
    if linear <= 0.0 { return f64::NEG_INFINITY; }
    10.0 * linear.log10()
}

// ═══════════════════════════════════════
// Temperature
// ═══════════════════════════════════════

/// °C to °F.
#[must_use]
pub fn celsius_to_fahrenheit(c: f64) -> f64 { c * 9.0 / 5.0 + 32.0 }

/// °F to °C.
#[must_use]
pub fn fahrenheit_to_celsius(f: f64) -> f64 { (f - 32.0) * 5.0 / 9.0 }

/// °C to K.
#[must_use]
pub fn celsius_to_kelvin(c: f64) -> f64 { c + 273.15 }

// ═══════════════════════════════════════
// Electricity
// ═══════════════════════════════════════

/// Ohm's law: V = I * R.
#[must_use]
pub fn ohms_law_voltage(current_a: f64, resistance_ohm: f64) -> f64 {
    current_a * resistance_ohm
}

/// Ohm's law: I = V / R.
#[must_use]
pub fn ohms_law_current(voltage_v: f64, resistance_ohm: f64) -> f64 {
    if resistance_ohm == 0.0 { return 0.0; }
    voltage_v / resistance_ohm
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_kinetic_energy() {
        // ½ * 2 * 3² = 9
        assert!((kinetic_energy(2.0, 3.0) - 9.0).abs() < 1e-10);
    }

    #[test] fn test_potential_energy() {
        // 10 * 9.80665 * 5 = 490.3325
        let pe = potential_energy(10.0, 5.0, 0.0);
        assert!((pe - 490.3325).abs() < 0.001, "got {pe}");
    }

    #[test] fn test_work_zero_angle() {
        // cos(0) = 1
        assert!((work_joules(10.0, 5.0, 0.0) - 50.0).abs() < 1e-10);
    }

    #[test] fn test_work_90_deg() {
        // cos(90°) ≈ 0
        let w = work_joules(10.0, 5.0, 90.0);
        assert!(w.abs() < 1e-10, "got {w}");
    }

    #[test] fn test_power() {
        assert!((power_watts(100.0, 5.0) - 20.0).abs() < 1e-10);
        assert_eq!(power_watts(100.0, 0.0), 0.0);
    }

    #[test] fn test_momentum() {
        assert!((momentum(5.0, 3.0) - 15.0).abs() < 1e-10);
    }

    #[test] fn test_freq_wavelength() {
        let c = speed_of_light();
        let f = 1e9; // 1 GHz
        let wl = frequency_to_wavelength(f, c);
        assert!((wl - 0.299_792_458).abs() < 1e-6, "got {wl}");
        let f2 = wavelength_to_frequency(wl, c);
        assert!((f2 - f).abs() < 1.0, "got {f2}");
    }

    #[test] fn test_gravitational_force() {
        // Earth pulling 1 kg from surface ≈ 9.82 N (rough, r=6.371e6)
        let f = gravitational_force(5.972e24, 1.0, 6.371e6);
        assert!(f > 9.5 && f < 10.5, "got {f}");
    }

    #[test] fn test_db_linear() {
        assert!((db_to_linear(10.0) - 10.0).abs() < 1e-10);
        assert!((db_to_linear(0.0) - 1.0).abs() < 1e-10);
        assert!((linear_to_db(10.0) - 10.0).abs() < 1e-10);
        assert!(linear_to_db(0.0).is_infinite());
    }

    #[test] fn test_temperature() {
        assert!((celsius_to_fahrenheit(0.0) - 32.0).abs() < 1e-10);
        assert!((celsius_to_fahrenheit(100.0) - 212.0).abs() < 1e-10);
        assert!((fahrenheit_to_celsius(32.0) - 0.0).abs() < 1e-10);
        assert!((celsius_to_kelvin(0.0) - 273.15).abs() < 1e-10);
        assert!((celsius_to_kelvin(-273.15) - 0.0).abs() < 1e-10);
    }

    #[test] fn test_ohms_law() {
        assert!((ohms_law_voltage(2.0, 5.0) - 10.0).abs() < 1e-10);
        assert!((ohms_law_current(10.0, 2.0) - 5.0).abs() < 1e-10);
        assert_eq!(ohms_law_current(10.0, 0.0), 0.0);
    }

    #[test] fn test_constants() {
        assert_eq!(speed_of_light(), 299_792_458.0);
        assert!((electron_charge() - 1.602_176_634e-19).abs() < 1e-30);
    }
}
