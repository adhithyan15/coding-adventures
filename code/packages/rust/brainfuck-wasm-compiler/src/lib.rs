use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use brainfuck::parser::parse_brainfuck;
use brainfuck_ir_compiler::{compile, release_config, BuildConfig};
use compiler_ir::IrProgram;
use ir_optimizer::optimize_program;
use ir_to_wasm_compiler::{FunctionSignature, IrToWasmCompiler};
use ir_to_wasm_validator::validate as validate_ir_to_wasm;
use parser::grammar_parser::GrammarASTNode;
use wasm_module_encoder::encode_module;
use wasm_types::WasmModule;
use wasm_validator::{validate, ValidatedModule};

#[derive(Debug)]
pub struct PackageResult {
    pub source: String,
    pub filename: String,
    pub ast: GrammarASTNode,
    pub raw_ir: IrProgram,
    pub optimized_ir: IrProgram,
    pub module: WasmModule,
    pub validated_module: ValidatedModule,
    pub binary: Vec<u8>,
    pub wasm_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageError {
    pub stage: String,
    pub message: String,
}

impl PackageError {
    fn new(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.stage, self.message)
    }
}

impl std::error::Error for PackageError {}

#[derive(Debug, Clone)]
pub struct BrainfuckWasmCompiler {
    pub filename: String,
    pub build_config: Option<BuildConfig>,
    pub optimize_ir: bool,
}

impl Default for BrainfuckWasmCompiler {
    fn default() -> Self {
        Self {
            filename: "program.bf".to_string(),
            build_config: None,
            optimize_ir: true,
        }
    }
}

impl BrainfuckWasmCompiler {
    pub fn compile_source(&self, source: &str) -> Result<PackageResult, PackageError> {
        let filename = self.filename.clone();
        let config = self.build_config.clone().unwrap_or_else(release_config);
        let ast = parse_brainfuck(source).map_err(|err| PackageError::new("parse", err))?;
        let ir_result =
            compile(&ast, &filename, config).map_err(|err| PackageError::new("ir-compile", err))?;

        let raw_ir = ir_result.program;
        let optimized_ir = if self.optimize_ir {
            optimize_program(&raw_ir)
        } else {
            raw_ir.clone()
        };

        let signatures = vec![FunctionSignature {
            label: "_start".to_string(),
            param_count: 0,
            export_name: Some("_start".to_string()),
        }];
        let lowering_errors = validate_ir_to_wasm(&optimized_ir, &signatures);
        if let Some(err) = lowering_errors.first() {
            return Err(PackageError::new("validate-ir", err.message.clone()));
        }

        let module = IrToWasmCompiler::default()
            .compile(&optimized_ir, &signatures)
            .map_err(|err| PackageError::new("lower", err.to_string()))?;
        let validated_module =
            validate(&module).map_err(|err| PackageError::new("validate-wasm", err.to_string()))?;
        let binary =
            encode_module(&module).map_err(|err| PackageError::new("encode", err.to_string()))?;

        Ok(PackageResult {
            source: source.to_string(),
            filename,
            ast,
            raw_ir,
            optimized_ir,
            module,
            validated_module,
            binary,
            wasm_path: None,
        })
    }

    pub fn write_wasm_file(
        &self,
        source: &str,
        output_path: impl AsRef<Path>,
    ) -> Result<PackageResult, PackageError> {
        let mut result = self.compile_source(source)?;
        let path = output_path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| PackageError::new("write", err.to_string()))?;
        }
        fs::write(&path, &result.binary)
            .map_err(|err| PackageError::new("write", err.to_string()))?;
        result.wasm_path = Some(path);
        Ok(result)
    }
}

pub fn compile_source(source: &str) -> Result<PackageResult, PackageError> {
    BrainfuckWasmCompiler::default().compile_source(source)
}

pub fn pack_source(source: &str) -> Result<PackageResult, PackageError> {
    compile_source(source)
}

pub fn write_wasm_file(
    source: &str,
    output_path: impl AsRef<Path>,
) -> Result<PackageResult, PackageError> {
    BrainfuckWasmCompiler::default().write_wasm_file(source, output_path)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use wasm_runtime::{WasiConfig, WasiEnv, WasmRuntime};

    use super::*;

    #[test]
    fn compiles_brainfuck_to_valid_wasm_bytes() {
        let result = compile_source(",.").unwrap();

        assert!(!result.raw_ir.instructions.is_empty());
        assert!(!result.binary.is_empty());
        assert!(result
            .module
            .exports
            .iter()
            .any(|entry| entry.name == "_start"));
    }

    #[test]
    fn compiled_binary_runs_in_runtime() {
        let result = compile_source(",.").unwrap();
        let output = Arc::new(Mutex::new(String::new()));
        let output_ref = Arc::clone(&output);
        let wasi = WasiEnv::new(WasiConfig {
            stdout_callback: Some(Box::new(move |text| {
                output_ref.lock().unwrap().push_str(text);
            })),
            stdin_callback: Some(Box::new(|requested| {
                let mut bytes = b"R".to_vec();
                bytes.truncate(requested);
                bytes
            })),
            ..Default::default()
        });
        let runtime = WasmRuntime::with_host(Box::new(wasi));

        let values = runtime.load_and_run(&result.binary, "_start", &[]).unwrap();

        assert_eq!(values, vec![0]);
        assert_eq!(output.lock().unwrap().as_str(), "R");
    }

    #[test]
    fn writes_wasm_file() {
        let target = std::env::temp_dir().join("brainfuck-wasm-compiler-test.wasm");
        let result = write_wasm_file("+.", &target).unwrap();

        assert_eq!(result.wasm_path.as_deref(), Some(target.as_path()));
        assert!(target.exists());

        let _ = std::fs::remove_file(target);
    }
}
