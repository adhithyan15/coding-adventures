//! Tests for BJT transistors (NPN and PNP).

use transistors::bjt::{NPN, PNP};
use transistors::types::{BJTParams, BJTRegion};

// =========================================================================
// NPN Tests
// =========================================================================

#[test]
fn npn_cutoff_region() {
    // Vbe below threshold -> no current
    let t = NPN::new(None);
    assert_eq!(t.region(0.0, 5.0), BJTRegion::Cutoff);
    assert_eq!(t.collector_current(0.0, 5.0), 0.0);
    assert!(!t.is_conducting(0.0));
}

#[test]
fn npn_active_region() {
    // Vbe at threshold, Vce > Vce_sat -> active (amplifier)
    let t = NPN::new(None);
    assert_eq!(t.region(0.7, 3.0), BJTRegion::Active);
    let ic = t.collector_current(0.7, 3.0);
    assert!(ic > 0.0);
}

#[test]
fn npn_saturation_region() {
    // Vbe at threshold, Vce <= Vce_sat -> saturated (switch ON)
    let t = NPN::new(None);
    assert_eq!(t.region(0.7, 0.1), BJTRegion::Saturation);
}

#[test]
fn npn_is_conducting() {
    // is_conducting should be true when Vbe >= Vbe_on
    let t = NPN::new(None);
    assert!(!t.is_conducting(0.5));
    assert!(t.is_conducting(0.7));
    assert!(t.is_conducting(1.0));
}

#[test]
fn npn_current_gain() {
    // In active region, Ic should be approximately beta * Ib
    let t = NPN::new(Some(BJTParams {
        beta: 100.0,
        ..BJTParams::default()
    }));
    let ic = t.collector_current(0.7, 3.0);
    let ib = t.base_current(0.7, 3.0);
    if ib > 0.0 {
        assert!((ic / ib - 100.0).abs() < 1.0);
    }
}

#[test]
fn npn_base_current_cutoff() {
    // Base current should be 0 in cutoff
    let t = NPN::new(None);
    assert_eq!(t.base_current(0.0, 5.0), 0.0);
}

#[test]
fn npn_transconductance_cutoff() {
    // gm should be 0 in cutoff
    let t = NPN::new(None);
    assert_eq!(t.transconductance(0.0, 5.0), 0.0);
}

#[test]
fn npn_transconductance_active() {
    // gm should be positive in active region
    let t = NPN::new(None);
    let gm = t.transconductance(0.7, 3.0);
    assert!(gm > 0.0);
}

#[test]
fn npn_custom_beta() {
    // Custom beta should affect current gain
    let t_low = NPN::new(Some(BJTParams {
        beta: 50.0,
        ..BJTParams::default()
    }));
    let t_high = NPN::new(Some(BJTParams {
        beta: 200.0,
        ..BJTParams::default()
    }));
    // Same Ic (determined by Is and Vbe), different Ib
    let ib_low = t_low.base_current(0.7, 3.0);
    let ib_high = t_high.base_current(0.7, 3.0);
    assert!(ib_low > ib_high); // Lower beta = more base current
}

#[test]
fn npn_saturation_boundary() {
    // At Vce = Vce_sat, transistor is in saturation
    let t = NPN::new(None);
    assert_eq!(t.region(0.7, 0.2), BJTRegion::Saturation);
}

#[test]
fn npn_active_boundary() {
    // Just above Vce_sat, transistor is in active
    let t = NPN::new(None);
    assert_eq!(t.region(0.7, 0.3), BJTRegion::Active);
}

// =========================================================================
// PNP Tests
// =========================================================================

#[test]
fn pnp_cutoff_region() {
    // PNP with small |Vbe| should be OFF
    let t = PNP::new(None);
    assert_eq!(t.region(0.0, 0.0), BJTRegion::Cutoff);
    assert_eq!(t.collector_current(0.0, 0.0), 0.0);
    assert!(!t.is_conducting(0.0));
}

#[test]
fn pnp_conducts_with_negative_vbe() {
    // PNP conducts when |Vbe| >= Vbe_on (Vbe typically negative)
    let t = PNP::new(None);
    assert!(t.is_conducting(-0.7));
    assert_eq!(t.region(-0.7, -3.0), BJTRegion::Active);
}

#[test]
fn pnp_saturation() {
    // PNP in saturation when |Vce| <= Vce_sat
    let t = PNP::new(None);
    assert_eq!(t.region(-0.7, -0.1), BJTRegion::Saturation);
}

#[test]
fn pnp_collector_current_positive() {
    // PNP collector current magnitude should be positive
    let t = PNP::new(None);
    let ic = t.collector_current(-0.7, -3.0);
    assert!(ic > 0.0);
}

#[test]
fn pnp_base_current() {
    // PNP should have non-zero base current when conducting
    let t = PNP::new(None);
    let ib = t.base_current(-0.7, -3.0);
    assert!(ib > 0.0);
}

#[test]
fn pnp_cutoff_no_base_current() {
    // PNP base current should be 0 in cutoff
    let t = PNP::new(None);
    assert_eq!(t.base_current(0.0, 0.0), 0.0);
}

#[test]
fn pnp_transconductance() {
    // PNP gm should be positive when conducting
    let t = PNP::new(None);
    let gm = t.transconductance(-0.7, -3.0);
    assert!(gm > 0.0);
}

#[test]
fn pnp_transconductance_cutoff() {
    // PNP gm should be 0 in cutoff
    let t = PNP::new(None);
    assert_eq!(t.transconductance(0.0, 0.0), 0.0);
}
