use compiler_ir::{IrOp, IrProgram};

#[derive(Default)]
pub struct IrOptimizer;

impl IrOptimizer {
    pub fn optimize(&self, program: &IrProgram) -> IrProgram {
        let mut optimized = program.clone();
        optimized
            .instructions
            .retain(|instruction| instruction.opcode != IrOp::Nop);
        optimized
    }
}

pub fn optimize_program(program: &IrProgram) -> IrProgram {
    IrOptimizer.optimize(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use compiler_ir::IrInstruction;

    #[test]
    fn strips_nops() {
        let mut program = IrProgram::new("_start");
        program.add_instruction(IrInstruction::new(IrOp::Nop, vec![], 0));
        let optimized = optimize_program(&program);
        assert!(optimized.instructions.is_empty());
    }
}
