use ir_to_cil_bytecode::builder::{encode_ldc_i8, CILBytecodeBuilder};

#[test]
fn int64_encoding_uses_nine_bytes_even_for_small_values() {
    for (value, expected) in [
        (1, vec![0x21, 1, 0, 0, 0, 0, 0, 0, 0]),
        (i64::MIN, vec![0x21, 0, 0, 0, 0, 0, 0, 0, 0x80]),
        (i64::MAX, vec![0x21, 255, 255, 255, 255, 255, 255, 255, 127]),
        (4294967296, vec![0x21, 0, 0, 0, 0, 1, 0, 0, 0]),
    ] {
        assert_eq!(encode_ldc_i8(value), expected);
        let mut builder = CILBytecodeBuilder::new();
        builder.emit_ldc_i8(value);
        assert_eq!(builder.assemble().unwrap(), expected);
    }
}
