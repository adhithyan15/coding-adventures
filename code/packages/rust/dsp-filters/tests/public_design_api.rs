use dsp_filters::{
    design_band_pass, design_high_pass, design_low_pass, try_design_band_pass,
    try_design_high_pass, try_design_low_pass, FilterError, WindowType,
};

#[test]
fn checked_and_legacy_designers_are_available_from_the_public_crate() {
    let window = WindowType::Hamming;
    assert_eq!(
        design_low_pass(0.2, 3, window),
        try_design_low_pass(0.2, 3, window).unwrap()
    );
    assert_eq!(
        design_high_pass(0.2, 3, window),
        try_design_high_pass(0.2, 3, window).unwrap()
    );
    assert_eq!(
        design_band_pass(0.1, 0.3, 3, window),
        try_design_band_pass(0.1, 0.3, 3, window).unwrap()
    );

    assert!(matches!(
        try_design_low_pass(0.6, 3, window),
        Err(FilterError::InvalidCoefficient(_))
    ));
    assert!(matches!(
        try_design_high_pass(f32::NAN, 3, window),
        Err(FilterError::InvalidCoefficient(_))
    ));
    assert!(matches!(
        try_design_band_pass(0.3, 0.2, 3, window),
        Err(FilterError::InvalidCoefficient(_))
    ));
}
