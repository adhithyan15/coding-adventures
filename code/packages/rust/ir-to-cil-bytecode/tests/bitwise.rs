use ir_to_cil_bytecode::builder::{CILBranchKind, CILBytecodeBuilder};
#[test]
fn bitwise_bytes_and_branch_offsets() {
    let mut b = CILBytecodeBuilder::new();
    b.emit_branch(CILBranchKind::Always, "end", false);
    b.emit_and();
    b.emit_or();
    b.mark("end");
    assert_eq!(b.assemble().unwrap(), vec![0x2b, 2, 0x5f, 0x60]);
}
