use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use compiler_ir::IrProgram;
use ir_optimizer::optimize_program;
use ir_to_wasm_compiler::{FunctionSignature, IrToWasmCompiler};
use ir_to_wasm_validator::validate as validate_ir_to_wasm;
use nib_ir_compiler::{compile_nib, release_config, BuildConfig};
use nib_type_checker::{check, TypedAst};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use wasm_module_encoder::encode_module;
use wasm_types::WasmModule;
use wasm_validator::{validate, ValidatedModule};

#[derive(Debug)]
pub struct PackageResult {
    pub source: String,
    pub ast: GrammarASTNode,
    pub typed_ast: TypedAst,
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
pub struct NibWasmCompiler {
    pub build_config: Option<BuildConfig>,
    pub optimize_ir: bool,
}

impl Default for NibWasmCompiler {
    fn default() -> Self {
        Self {
            build_config: None,
            optimize_ir: true,
        }
    }
}

impl NibWasmCompiler {
    pub fn compile_source(&self, source: &str) -> Result<PackageResult, PackageError> {
        let ast = coding_adventures_nib_parser::parse_nib(source)
            .map_err(|err| PackageError::new("parse", err.to_string()))?;
        let ast_for_result = ast.clone();
        let type_result = check(ast);
        if !type_result.ok {
            let diagnostics = type_result
                .errors
                .iter()
                .map(|error| {
                    format!(
                        "Line {}, Col {}: {}",
                        error.line, error.column, error.message
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            return Err(PackageError::new("type-check", diagnostics));
        }

        let typed_ast = type_result.typed_ast;
        let config = self.build_config.unwrap_or_else(release_config);
        let raw_ir = compile_nib(typed_ast.clone(), config).program;
        let optimized_ir = if self.optimize_ir {
            optimize_program(&raw_ir)
        } else {
            raw_ir.clone()
        };
        let signatures = extract_signatures(&typed_ast.root);

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
            ast: ast_for_result,
            typed_ast,
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
    NibWasmCompiler::default().compile_source(source)
}

pub fn pack_source(source: &str) -> Result<PackageResult, PackageError> {
    compile_source(source)
}

pub fn write_wasm_file(
    source: &str,
    output_path: impl AsRef<Path>,
) -> Result<PackageResult, PackageError> {
    NibWasmCompiler::default().write_wasm_file(source, output_path)
}

pub fn extract_signatures(ast: &GrammarASTNode) -> Vec<FunctionSignature> {
    let mut signatures = vec![FunctionSignature {
        label: "_start".to_string(),
        param_count: 0,
        export_name: Some("_start".to_string()),
    }];

    for decl in function_nodes(ast) {
        if let Some(name) = first_name(decl) {
            signatures.push(FunctionSignature {
                label: format!("_fn_{name}"),
                param_count: count_params(decl),
                export_name: Some(name),
            });
        }
    }

    signatures
}

fn function_nodes(root: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    child_nodes(root)
        .into_iter()
        .filter_map(|node| {
            if node.rule_name == "fn_decl" {
                Some(node)
            } else if node.rule_name == "top_decl" {
                child_nodes(node)
                    .into_iter()
                    .find(|inner| inner.rule_name == "fn_decl")
            } else {
                None
            }
        })
        .collect()
}

fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|child| match child {
            ASTNodeOrToken::Node(inner) => Some(inner),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

fn first_name(node: &GrammarASTNode) -> Option<String> {
    node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Token(token) if token.effective_type_name() == "NAME" => {
            Some(token.value.clone())
        }
        ASTNodeOrToken::Node(inner) => first_name(inner),
        _ => None,
    })
}

fn count_params(fn_decl: &GrammarASTNode) -> usize {
    child_nodes(fn_decl)
        .into_iter()
        .find(|node| node.rule_name == "param_list")
        .map(|param_list| {
            child_nodes(param_list)
                .into_iter()
                .filter(|node| node.rule_name == "param")
                .count()
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use wasm_runtime::WasmRuntime;

    use super::*;

    fn run(binary: &[u8], entry: &str, args: &[i64]) -> Vec<i64> {
        WasmRuntime::new()
            .load_and_run(binary, entry, args)
            .expect("runtime should execute compiled Nib")
    }

    #[test]
    fn compile_source_returns_pipeline_artifacts() {
        let result = compile_source("fn answer() -> u4 { return 7; }").unwrap();

        assert!(!result.raw_ir.instructions.is_empty());
        assert!(!result.optimized_ir.instructions.is_empty());
        assert!(!result.binary.is_empty());
        assert!(result
            .module
            .exports
            .iter()
            .any(|entry| entry.name == "answer"));
    }

    #[test]
    fn pack_source_aliases_compile_source() {
        let compiled = compile_source("fn answer() -> u4 { return 7; }").unwrap();
        let packed = pack_source("fn answer() -> u4 { return 7; }").unwrap();

        assert_eq!(packed.binary, compiled.binary);
    }

    #[test]
    fn write_wasm_file_persists_bytes() {
        let target = std::env::temp_dir().join("nib-wasm-compiler-test.wasm");
        let result = write_wasm_file("fn answer() -> u4 { return 7; }", &target).unwrap();

        assert_eq!(result.wasm_path.as_deref(), Some(target.as_path()));
        assert_eq!(std::fs::read(&target).unwrap(), result.binary);

        let _ = std::fs::remove_file(target);
    }

    #[test]
    fn compiled_function_runs_in_runtime() {
        let result = compile_source("fn answer() -> u4 { return 7; }").unwrap();

        assert_eq!(run(&result.binary, "answer", &[]), vec![7]);
    }

    #[test]
    fn compiled_entrypoint_runs_in_runtime() {
        let source = "fn main() -> u4 { return 7; }";
        let result = compile_source(source).unwrap();

        assert_eq!(run(&result.binary, "_start", &[]), vec![7]);
    }

    #[test]
    fn type_errors_report_stage() {
        let err = compile_source("fn main() { let flag: bool = 1; }").unwrap_err();

        assert_eq!(err.stage, "type-check");
    }
}
