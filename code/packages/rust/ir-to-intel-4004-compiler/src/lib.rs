use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
use intel_4004_ir_validator::{IrValidator, ValidationDiagnostic};

pub struct IrToIntel4004Compiler {
    validator: IrValidator,
}

impl Default for IrToIntel4004Compiler {
    fn default() -> Self {
        Self {
            validator: IrValidator,
        }
    }
}

impl IrToIntel4004Compiler {
    pub fn compile(&self, program: &IrProgram) -> Result<String, Vec<ValidationDiagnostic>> {
        let errors = self.validator.validate(program);
        if !errors.is_empty() {
            return Err(errors);
        }

        let mut lines = vec!["    ORG 0x000".to_string()];
        for instruction in &program.instructions {
            lines.extend(emit_instruction(instruction));
        }
        Ok(lines.join("\n") + "\n")
    }
}

fn emit_instruction(instruction: &IrInstruction) -> Vec<String> {
    match instruction.opcode {
        IrOp::Label => match &instruction.operands[0] {
            IrOperand::Label(label) => vec![format!("{label}:")],
            _ => vec!["    ; invalid label".to_string()],
        },
        IrOp::LoadImm => {
            let dest = register_index(&instruction.operands[0]).unwrap_or(2);
            let value = immediate_value(&instruction.operands[1]).unwrap_or(0);
            if value <= 15 {
                vec![format!("    LDM {value}"), format!("    XCH R{dest}")]
            } else {
                vec![format!("    FIM P{}, {value}", dest / 2)]
            }
        }
        IrOp::AddImm => {
            let dest = register_index(&instruction.operands[0]).unwrap_or(2);
            let src = register_index(&instruction.operands[1]).unwrap_or(dest);
            let imm = immediate_value(&instruction.operands[2]).unwrap_or(0);
            if imm == 0 {
                vec![format!("    LD R{src}"), format!("    XCH R{dest}")]
            } else {
                let scratch = if src == 1 { 14 } else { 1 };
                vec![
                    format!("    LDM {imm}"),
                    format!("    XCH R{scratch}"),
                    format!("    LD R{src}"),
                    format!("    ADD R{scratch}"),
                    format!("    XCH R{dest}"),
                ]
            }
        }
        IrOp::Add => {
            let dest = register_index(&instruction.operands[0]).unwrap_or(2);
            let left = register_index(&instruction.operands[1]).unwrap_or(dest);
            let right = register_index(&instruction.operands[2]).unwrap_or(dest);
            vec![
                format!("    LD R{left}"),
                format!("    ADD R{right}"),
                format!("    XCH R{dest}"),
            ]
        }
        IrOp::Jump => match &instruction.operands[0] {
            IrOperand::Label(label) => vec![format!("    JUN {label}")],
            _ => vec!["    ; invalid jump".to_string()],
        },
        IrOp::Call => match &instruction.operands[0] {
            IrOperand::Label(label) => vec![format!("    JMS {label}")],
            _ => vec!["    ; invalid call".to_string()],
        },
        IrOp::Ret => vec!["    BBL 0".to_string()],
        IrOp::Halt => vec!["    HLT".to_string()],
        _ => vec![format!("    ; unsupported opcode {:?}", instruction.opcode)],
    }
}

fn register_index(operand: &IrOperand) -> Option<usize> {
    match operand {
        IrOperand::Register(index) => Some(*index),
        _ => None,
    }
}

fn immediate_value(operand: &IrOperand) -> Option<i64> {
    match operand {
        IrOperand::Immediate(value) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_load_imm_program() {
        let mut program = IrProgram::new("_start");
        program.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(2), IrOperand::Immediate(5)],
            0,
        ));
        let asm = IrToIntel4004Compiler::default().compile(&program).unwrap();
        assert!(asm.contains("LDM 5"));
    }
}
