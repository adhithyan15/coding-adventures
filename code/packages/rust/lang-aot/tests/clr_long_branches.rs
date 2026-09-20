use clr_simulator::{CLRSimulator, Value};
use ir_to_cil_bytecode::builder::{CILBranchKind, CILBytecodeBuilder};
#[test]
fn promoted_branches_execute_literal_targets() {
    for (kind, opcode) in [
        (CILBranchKind::Always, 0x38),
        (CILBranchKind::False, 0x39),
        (CILBranchKind::True, 0x3a),
    ] {
        for cond in [0, 1] {
            let mut b = CILBytecodeBuilder::new();
            if opcode != 0x38 {
                b.emit_ldc_i4(cond);
            }
            b.emit_branch(kind, "far", false);
            b.emit_ldc_i4(1);
            b.emit_ret();
            for _ in 0..140 {
                b.emit_raw(vec![0]);
            }
            b.mark("far");
            b.emit_ldc_i4(2);
            b.emit_ret();
            let bytes = b.assemble().unwrap();
            let at = usize::from(opcode != 0x38);
            assert_eq!(bytes[at], opcode);
            assert_eq!(&bytes[at + 1..at + 5], &142_i32.to_le_bytes());
            let mut s = CLRSimulator::new();
            s.load(&bytes, 0);
            s.run(10);
            assert!(s.halted);
            let take = opcode == 0x38 || if opcode == 0x39 { cond == 0 } else { cond != 0 };
            assert_eq!(s.stack, vec![Some(Value::Int(if take { 2 } else { 1 }))]);
        }
    }
}
