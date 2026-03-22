//! Tests for analog amplifier analysis.

use transistors::amplifier::{analyze_common_emitter_amp, analyze_common_source_amp};
use transistors::bjt::NPN;
use transistors::mosfet::NMOS;
use transistors::types::BJTParams;

// =========================================================================
// Common-Source Amplifier Tests
// =========================================================================

#[test]
fn common_source_inverting_gain() {
    // Common-source amplifier should have negative voltage gain
    let t = NMOS::new(None);
    let result = analyze_common_source_amp(&t, 1.5, 3.3, 10_000.0, None);
    assert!(result.voltage_gain < 0.0);
}

#[test]
fn common_source_high_input_impedance() {
    // MOSFET amplifiers should have very high input impedance
    let t = NMOS::new(None);
    let result = analyze_common_source_amp(&t, 1.5, 3.3, 10_000.0, None);
    assert!(result.input_impedance > 1e9);
}

#[test]
fn common_source_positive_transconductance() {
    // Transconductance should be positive
    let t = NMOS::new(None);
    let result = analyze_common_source_amp(&t, 1.5, 3.3, 10_000.0, None);
    assert!(result.transconductance > 0.0);
}

#[test]
fn common_source_positive_bandwidth() {
    // Bandwidth should be positive
    let t = NMOS::new(None);
    let result = analyze_common_source_amp(&t, 1.5, 3.3, 10_000.0, None);
    assert!(result.bandwidth > 0.0);
}

#[test]
fn common_source_operating_point() {
    // Operating point should contain required keys
    let t = NMOS::new(None);
    let result = analyze_common_source_amp(&t, 1.5, 3.3, 10_000.0, None);
    assert!(result.operating_point.contains_key("vgs"));
    assert!(result.operating_point.contains_key("vds"));
    assert!(result.operating_point.contains_key("ids"));
    assert!(result.operating_point.contains_key("gm"));
}

#[test]
fn common_source_higher_rd_more_gain() {
    // Higher drain resistance should give more voltage gain
    let t = NMOS::new(None);
    let r1 = analyze_common_source_amp(&t, 1.5, 3.3, 5_000.0, None);
    let r2 = analyze_common_source_amp(&t, 1.5, 3.3, 20_000.0, None);
    assert!(r2.voltage_gain.abs() > r1.voltage_gain.abs());
}

// =========================================================================
// Common-Emitter Amplifier Tests
// =========================================================================

#[test]
fn common_emitter_inverting_gain() {
    // Common-emitter amplifier should have negative voltage gain
    let t = NPN::new(None);
    let result = analyze_common_emitter_amp(&t, 0.7, 5.0, 4700.0, None);
    assert!(result.voltage_gain < 0.0);
}

#[test]
fn common_emitter_moderate_input_impedance() {
    // BJT amplifiers have moderate input impedance (r_pi)
    let t = NPN::new(None);
    let result = analyze_common_emitter_amp(&t, 0.7, 5.0, 4700.0, None);
    // r_pi should be in kOhm range for typical bias
    assert!(result.input_impedance > 100.0);
    assert!(result.input_impedance < 1e6);
}

#[test]
fn common_emitter_positive_transconductance() {
    // Transconductance should be positive
    let t = NPN::new(None);
    let result = analyze_common_emitter_amp(&t, 0.7, 5.0, 4700.0, None);
    assert!(result.transconductance > 0.0);
}

#[test]
fn common_emitter_higher_beta_higher_impedance() {
    // Higher beta should give higher input impedance
    let t_low = NPN::new(Some(BJTParams {
        beta: 50.0,
        ..BJTParams::default()
    }));
    let t_high = NPN::new(Some(BJTParams {
        beta: 200.0,
        ..BJTParams::default()
    }));
    let r1 = analyze_common_emitter_amp(&t_low, 0.7, 5.0, 4700.0, None);
    let r2 = analyze_common_emitter_amp(&t_high, 0.7, 5.0, 4700.0, None);
    assert!(r2.input_impedance > r1.input_impedance);
}

#[test]
fn common_emitter_operating_point() {
    // Operating point should contain required keys
    let t = NPN::new(None);
    let result = analyze_common_emitter_amp(&t, 0.7, 5.0, 4700.0, None);
    assert!(result.operating_point.contains_key("vbe"));
    assert!(result.operating_point.contains_key("vce"));
    assert!(result.operating_point.contains_key("ic"));
    assert!(result.operating_point.contains_key("ib"));
}
