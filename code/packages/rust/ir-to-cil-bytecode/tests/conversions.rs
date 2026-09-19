use ir_to_cil_bytecode::builder::{CILBranchKind, CILBytecodeBuilder};
#[test]
fn conversions_encode_single_bytes_and_count_toward_branches() {
    let mut b = CILBytecodeBuilder::new();
    b.emit_branch(CILBranchKind::Always, "end", false);
    b.emit_conv_i4();
    b.emit_conv_i8();
    b.mark("end");
    assert_eq!(b.assemble().unwrap(), vec![0x2b, 2, 0x69, 0x6a]);
}
