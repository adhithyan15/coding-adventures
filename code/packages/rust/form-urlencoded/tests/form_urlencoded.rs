use form_urlencoded::{
    encode_form_urlencoded, validate_form_urlencoded, FormUrlEncodedError, FormUrlEncodedField,
    FormUrlEncodedLimits, MAX_FORM_URLENCODED_COMPONENT_BYTES, MAX_FORM_URLENCODED_FIELDS,
    MAX_FORM_URLENCODED_OUTPUT_BYTES,
};

fn limits() -> FormUrlEncodedLimits {
    FormUrlEncodedLimits::new(16, 1024, 4096).unwrap()
}

#[test]
fn encodes_oauth_shaped_fields_with_exact_form_profile() {
    let fields = [
        FormUrlEncodedField::new("grant_type", "authorization_code"),
        FormUrlEncodedField::new("code", "a b/c~"),
    ];
    let form = validate_form_urlencoded(&fields, limits()).unwrap();
    let mut output = [0xa5; 4096];
    let encoded = encode_form_urlencoded(&form, &mut output).unwrap();
    let encoded_length = encoded.len();

    assert_eq!(encoded, b"grant_type=authorization_code&code=a+b%2Fc%7E");
    assert_eq!(form.encoded_length(), encoded_length);
    assert_eq!(output[encoded_length], 0xa5);
}

#[test]
fn preserves_order_duplicates_and_empty_components() {
    let fields = [
        FormUrlEncodedField::new("", ""),
        FormUrlEncodedField::new("scope", "one"),
        FormUrlEncodedField::new("scope", "two"),
    ];
    let form = validate_form_urlencoded(&fields, limits()).unwrap();
    let mut output = [0; 64];
    assert_eq!(
        encode_form_urlencoded(&form, &mut output).unwrap(),
        b"=&scope=one&scope=two"
    );
    assert_eq!(form.fields()[1].name(), "scope");
    assert_eq!(form.fields()[2].value(), "two");
}

#[test]
fn encodes_utf8_bytes_reserved_bytes_and_safe_ascii() {
    let fields = [
        FormUrlEncodedField::new("safe", "AZaz09*-._"),
        FormUrlEncodedField::new("utf8", "b\u{00fc}cher"),
        FormUrlEncodedField::new("reserved", "+&=%"),
    ];
    let form = validate_form_urlencoded(&fields, limits()).unwrap();
    let mut output = [0; 128];
    assert_eq!(
        encode_form_urlencoded(&form, &mut output).unwrap(),
        b"safe=AZaz09*-._&utf8=b%C3%BCcher&reserved=%2B%26%3D%25"
    );
}

#[test]
fn empty_field_sequence_encodes_as_empty_output() {
    let fields = [];
    let form = validate_form_urlencoded(&fields, limits()).unwrap();
    let mut output = [];
    assert_eq!(encode_form_urlencoded(&form, &mut output).unwrap(), b"");
}

#[test]
fn validates_zero_and_exact_package_limit_ceilings() {
    let zero = FormUrlEncodedLimits::new(0, 0, 0).unwrap();
    assert!(validate_form_urlencoded(&[], zero).is_ok());

    let maximum = FormUrlEncodedLimits::new(
        MAX_FORM_URLENCODED_FIELDS,
        MAX_FORM_URLENCODED_COMPONENT_BYTES,
        MAX_FORM_URLENCODED_OUTPUT_BYTES,
    )
    .unwrap();
    assert_eq!(maximum.maximum_fields(), MAX_FORM_URLENCODED_FIELDS);
    assert_eq!(
        maximum.maximum_component_bytes(),
        MAX_FORM_URLENCODED_COMPONENT_BYTES
    );
    assert_eq!(
        maximum.maximum_output_bytes(),
        MAX_FORM_URLENCODED_OUTPUT_BYTES
    );

    for invalid in [
        FormUrlEncodedLimits::new(MAX_FORM_URLENCODED_FIELDS + 1, 0, 0),
        FormUrlEncodedLimits::new(0, MAX_FORM_URLENCODED_COMPONENT_BYTES + 1, 0),
        FormUrlEncodedLimits::new(0, 0, MAX_FORM_URLENCODED_OUTPUT_BYTES + 1),
    ] {
        assert_eq!(invalid.unwrap_err(), FormUrlEncodedError::InvalidLimits);
    }
}

#[test]
fn rejects_field_component_and_encoded_output_limit_violations() {
    let two = [
        FormUrlEncodedField::new("a", "b"),
        FormUrlEncodedField::new("c", "d"),
    ];
    let one_field = FormUrlEncodedLimits::new(1, 1024, 4096).unwrap();
    assert_eq!(
        validate_form_urlencoded(&two, one_field).unwrap_err(),
        FormUrlEncodedError::TooManyFields
    );

    let long = "x".repeat(1025);
    let name = [FormUrlEncodedField::new(&long, "")];
    assert_eq!(
        validate_form_urlencoded(&name, limits()).unwrap_err(),
        FormUrlEncodedError::NameTooLong
    );
    let value = [FormUrlEncodedField::new("", &long)];
    assert_eq!(
        validate_form_urlencoded(&value, limits()).unwrap_err(),
        FormUrlEncodedError::ValueTooLong
    );

    let escaped = [FormUrlEncodedField::new("x", "////")];
    let tight_output = FormUrlEncodedLimits::new(1, 1024, 8).unwrap();
    assert_eq!(
        validate_form_urlencoded(&escaped, tight_output).unwrap_err(),
        FormUrlEncodedError::OutputTooLong
    );
}

#[test]
fn leaves_undersized_output_unchanged() {
    let fields = [FormUrlEncodedField::new("secret", "value")];
    let form = validate_form_urlencoded(&fields, limits()).unwrap();
    let mut output = [0xa5; 8];
    assert_eq!(
        encode_form_urlencoded(&form, &mut output).unwrap_err(),
        FormUrlEncodedError::OutputTooSmall
    );
    assert_eq!(output, [0xa5; 8]);
}

#[test]
fn debug_and_errors_do_not_render_sensitive_fields() {
    let fields = [FormUrlEncodedField::new("refresh_token", "secret-value")];
    let form = validate_form_urlencoded(&fields, limits()).unwrap();
    for debug in [format!("{:?}", fields[0]), format!("{form:?}")] {
        assert!(!debug.contains("refresh_token"));
        assert!(!debug.contains("secret-value"));
        assert!(debug.contains("[REDACTED]"));
    }

    let tight = FormUrlEncodedLimits::new(1, 4, 32).unwrap();
    let error = validate_form_urlencoded(&fields, tight).unwrap_err();
    assert!(!error.to_string().contains("refresh_token"));
    assert!(!format!("{error:?}").contains("secret-value"));
}
