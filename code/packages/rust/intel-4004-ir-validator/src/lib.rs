use std::fmt;

use compiler_ir::{IrOp, IrProgram};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationDiagnostic {
    pub rule: String,
    pub message: String,
}

impl fmt::Display for ValidationDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.rule, self.message)
    }
}

pub struct IrValidator;

impl IrValidator {
    pub fn validate(&self, program: &IrProgram) -> Vec<ValidationDiagnostic> {
        let mut errors = Vec::new();
        for instruction in &program.instructions {
            match instruction.opcode {
                IrOp::LoadImm
                | IrOp::AddImm
                | IrOp::Add
                | IrOp::Label
                | IrOp::Jump
                | IrOp::BranchZ
                | IrOp::BranchNz
                | IrOp::Call
                | IrOp::Ret
                | IrOp::Halt
                | IrOp::Nop
                | IrOp::Comment => {}
                _ => errors.push(ValidationDiagnostic {
                    rule: "supported-opcodes".to_string(),
                    message: format!(
                        "opcode {:?} is not yet supported on Intel 4004",
                        instruction.opcode
                    ),
                }),
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compiler_ir::{IrInstruction, IrOperand};

    #[test]
    fn accepts_simple_program() {
        let mut program = IrProgram::new("_start");
        program.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(2), IrOperand::Immediate(5)],
            0,
        ));
        assert!(IrValidator.validate(&program).is_empty());
    }
}
