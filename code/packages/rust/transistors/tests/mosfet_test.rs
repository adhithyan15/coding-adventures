//! Tests for MOSFET transistors (NMOS and PMOS).

use transistors::mosfet::{NMOS, PMOS};
use transistors::types::{MOSFETParams, MOSFETRegion};

// =========================================================================
// NMOS Tests
// =========================================================================

#[test]
fn nmos_cutoff_region() {
    // Vgs below threshold -> no current, switch OFF
    let t = NMOS::new(None);
    assert_eq!(t.region(0.0, 1.0), MOSFETRegion::Cutoff);
    assert_eq!(t.drain_current(0.0, 1.0), 0.0);
    assert!(!t.is_conducting(0.0));
}

#[test]
fn nmos_cutoff_negative_vgs() {
    // Negative Vgs should also be cutoff
    let t = NMOS::new(None);
    assert_eq!(t.region(-1.0, 0.0), MOSFETRegion::Cutoff);
    assert_eq!(t.drain_current(-1.0, 0.0), 0.0);
}

#[test]
fn nmos_linear_region() {
    // Vgs above threshold, low Vds -> linear region
    let t = NMOS::new(None);
    assert_eq!(t.region(1.5, 0.1), MOSFETRegion::Linear);
    let ids = t.drain_current(1.5, 0.1);
    assert!(ids > 0.0);
}

#[test]
fn nmos_saturation_region() {
    // Vgs above threshold, high Vds -> saturation
    let t = NMOS::new(None);
    assert_eq!(t.region(1.0, 3.0), MOSFETRegion::Saturation);
    let ids = t.drain_current(1.0, 3.0);
    assert!(ids > 0.0);
}

#[test]
fn nmos_saturation_current_independent_of_vds() {
    // In saturation, current depends only on Vgs, not Vds
    let t = NMOS::new(None);
    let ids_1 = t.drain_current(1.5, 3.0);
    let ids_2 = t.drain_current(1.5, 5.0);
    assert!((ids_1 - ids_2).abs() < 1e-10);
}

#[test]
fn nmos_linear_current_increases_with_vds() {
    // In linear region, current increases with Vds
    let t = NMOS::new(None);
    let ids_low = t.drain_current(3.0, 0.1);
    let ids_high = t.drain_current(3.0, 0.5);
    assert!(ids_high > ids_low);
}

#[test]
fn nmos_is_conducting() {
    // is_conducting should be true when Vgs >= Vth
    let t = NMOS::new(None);
    assert!(!t.is_conducting(0.3)); // Below default Vth=0.4
    assert!(t.is_conducting(0.4));  // At Vth
    assert!(t.is_conducting(1.0));  // Above Vth
}

#[test]
fn nmos_output_voltage_on() {
    // When ON, output should be pulled to GND
    let t = NMOS::new(None);
    assert_eq!(t.output_voltage(3.3, 3.3), 0.0);
}

#[test]
fn nmos_output_voltage_off() {
    // When OFF, output should be at Vdd
    let t = NMOS::new(None);
    assert_eq!(t.output_voltage(0.0, 3.3), 3.3);
}

#[test]
fn nmos_custom_params() {
    // Custom parameters should be respected
    let params = MOSFETParams {
        vth: 0.7,
        k: 0.002,
        ..MOSFETParams::default()
    };
    let t = NMOS::new(Some(params));
    assert!(!t.is_conducting(0.5)); // Below custom Vth
    assert!(t.is_conducting(0.7));  // At custom Vth
}

#[test]
fn nmos_transconductance_cutoff() {
    // gm should be 0 in cutoff
    let t = NMOS::new(None);
    assert_eq!(t.transconductance(0.0, 1.0), 0.0);
}

#[test]
fn nmos_transconductance_saturation() {
    // gm should be positive in saturation
    let t = NMOS::new(None);
    let gm = t.transconductance(1.5, 3.0);
    assert!(gm > 0.0);
}

#[test]
fn nmos_boundary_cutoff_linear() {
    // Just above Vth with small Vds -> linear
    let t = NMOS::new(None);
    assert_eq!(t.region(0.5, 0.01), MOSFETRegion::Linear);
}

#[test]
fn nmos_boundary_linear_saturation() {
    // At Vds = Vgs - Vth, transistor enters saturation
    let t = NMOS::new(None);
    let vgs = 1.0;
    let vth = 0.4;
    let vds = vgs - vth; // Exactly at boundary
    assert_eq!(t.region(vgs, vds), MOSFETRegion::Saturation);
}

// =========================================================================
// PMOS Tests
// =========================================================================

#[test]
fn pmos_cutoff_when_vgs_zero() {
    // PMOS with Vgs=0 (gate at source level) should be OFF
    let t = PMOS::new(None);
    assert_eq!(t.region(0.0, 0.0), MOSFETRegion::Cutoff);
    assert!(!t.is_conducting(0.0));
}

#[test]
fn pmos_conducts_when_vgs_negative() {
    // PMOS conducts when Vgs is sufficiently negative
    let t = PMOS::new(None);
    assert!(t.is_conducting(-1.5));
    assert_eq!(t.region(-1.5, -3.0), MOSFETRegion::Saturation);
}

#[test]
fn pmos_linear_region() {
    // PMOS in linear region with small |Vds|
    let t = PMOS::new(None);
    assert_eq!(t.region(-1.5, -0.1), MOSFETRegion::Linear);
}

#[test]
fn pmos_drain_current_positive() {
    // PMOS drain current magnitude should be positive
    let t = PMOS::new(None);
    let ids = t.drain_current(-1.5, -3.0);
    assert!(ids > 0.0);
}

#[test]
fn pmos_cutoff_no_current() {
    // PMOS in cutoff should have zero current
    let t = PMOS::new(None);
    assert_eq!(t.drain_current(0.0, -1.0), 0.0);
}

#[test]
fn pmos_output_voltage_on() {
    // When ON, PMOS pulls output to Vdd
    let t = PMOS::new(None);
    assert_eq!(t.output_voltage(-3.3, 3.3), 3.3);
}

#[test]
fn pmos_output_voltage_off() {
    // When OFF, PMOS output is at GND
    let t = PMOS::new(None);
    assert_eq!(t.output_voltage(0.0, 3.3), 0.0);
}

#[test]
fn pmos_complementary_to_nmos() {
    // PMOS should be ON when NMOS is OFF and vice versa
    let nmos = NMOS::new(None);
    let pmos = PMOS::new(None);
    let vdd = 3.3;

    // Input HIGH: NMOS ON, PMOS OFF
    assert!(nmos.is_conducting(vdd));
    assert!(!pmos.is_conducting(0.0));

    // Input LOW: NMOS OFF, PMOS ON
    assert!(!nmos.is_conducting(0.0));
    assert!(pmos.is_conducting(-vdd));
}

#[test]
fn pmos_transconductance_cutoff() {
    // gm should be 0 in cutoff
    let t = PMOS::new(None);
    assert_eq!(t.transconductance(0.0, 0.0), 0.0);
}

#[test]
fn pmos_transconductance_on() {
    // gm should be positive when conducting
    let t = PMOS::new(None);
    let gm = t.transconductance(-1.5, -3.0);
    assert!(gm > 0.0);
}
