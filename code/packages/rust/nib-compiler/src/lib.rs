use compiler_ir::IrProgram;
use intel_4004_assembler::assemble;
use intel_4004_packager::{decode_hex, encode_hex};
use ir_optimizer::optimize_program;
use ir_to_intel_4004_compiler::IrToIntel4004Compiler;
use nib_ir_compiler::{compile_nib, release_config};
use nib_type_checker::{check_source, TypedAst};

#[derive(Debug, Clone)]
pub struct CompileArtifacts {
    pub typed_ast: TypedAst,
    pub raw_ir: IrProgram,
    pub optimized_ir: IrProgram,
    pub assembly: String,
    pub binary: Vec<u8>,
    pub hex_text: String,
}

pub fn compile_source(source: &str) -> Result<CompileArtifacts, String> {
    let checked = check_source(source);
    if !checked.ok {
        return Err(checked
            .errors
            .iter()
            .map(|error| format!("{}:{}: {}", error.line, error.column, error.message))
            .collect::<Vec<_>>()
            .join("\n"));
    }

    let typed_ast = checked.typed_ast;
    let raw_ir = compile_nib(typed_ast.clone(), release_config()).program;
    let optimized_ir = optimize_program(&raw_ir);
    let assembly = IrToIntel4004Compiler::default()
        .compile(&optimized_ir)
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|error| error.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        })?;
    let binary = assemble(&assembly).map_err(|error| error.to_string())?;
    let hex_text = encode_hex(&binary, 0)?;

    Ok(CompileArtifacts {
        typed_ast,
        raw_ir,
        optimized_ir,
        assembly,
        binary,
        hex_text,
    })
}

pub fn decode_compiled_hex(text: &str) -> Result<Vec<u8>, String> {
    Ok(decode_hex(text)?.binary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use intel4004_simulator::Intel4004Simulator;

    #[test]
    fn compile_source_returns_artifacts() {
        let result = compile_source("fn main() { let x: u4 = 5; }").unwrap();
        assert!(!result.raw_ir.instructions.is_empty());
        assert!(!result.binary.is_empty());
        assert!(
            result.assembly.contains("LDM 5"),
            "assembly was:\n{}",
            result.assembly
        );
    }

    #[test]
    fn compiled_program_runs_in_simulator() {
        let result = compile_source("fn main() { let x: u4 = 5; }").unwrap();
        let decoded = decode_hex(&result.hex_text).unwrap();
        let mut sim = Intel4004Simulator::new(4096);
        let traces = sim.run(&decoded.binary, 100);
        assert!(!traces.is_empty());
        assert!(sim.halted);
        assert_eq!(sim.registers[2], 5, "assembly was:\n{}", result.assembly);
    }
}
