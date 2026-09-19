use ir_to_cil_bytecode::builder::{CILBranchKind, CILBytecodeBuilder};
#[test]
fn shifts_are_single_byte_branch_operands() {
    let mut b = CILBytecodeBuilder::new();
    b.emit_branch(CILBranchKind::Always, "end", false);
    b.emit_shl();
    b.emit_shr();
    b.mark("end");
    assert_eq!(b.assemble().unwrap(), vec![0x2b, 2, 0x62, 0x63]);
}
