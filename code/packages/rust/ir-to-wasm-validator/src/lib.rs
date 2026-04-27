use compiler_ir::IrProgram;
use ir_to_wasm_compiler::{FunctionSignature, IrToWasmCompiler};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub rule: String,
    pub message: String,
}

#[derive(Default)]
pub struct WasmIrValidator;

impl WasmIrValidator {
    pub fn validate(
        &self,
        program: &IrProgram,
        function_signatures: &[FunctionSignature],
    ) -> Vec<ValidationError> {
        match IrToWasmCompiler::default().compile(program, function_signatures) {
            Ok(_) => Vec::new(),
            Err(err) => vec![ValidationError {
                rule: "lowering".to_string(),
                message: err.to_string(),
            }],
        }
    }
}

pub fn validate(
    program: &IrProgram,
    function_signatures: &[FunctionSignature],
) -> Vec<ValidationError> {
    WasmIrValidator::default().validate(program, function_signatures)
}

#[cfg(test)]
mod tests {
    use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};

    use super::*;

    #[test]
    fn reports_lowering_errors() {
        let program = IrProgram {
            instructions: vec![
                IrInstruction::new(IrOp::Label, vec![IrOperand::Label("_start".into())], -1),
                IrInstruction::new(IrOp::Syscall, vec![IrOperand::Immediate(99)], 0),
            ],
            data: vec![],
            entry_label: "_start".into(),
            version: 1,
        };

        let errors = validate(&program, &[]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule, "lowering");
        assert!(errors[0].message.contains("unsupported SYSCALL"));
    }
}
