use std::fmt;
use std::path::{Path, PathBuf};

use coding_adventures_nib_parser::parse_nib;
use compiler_ir::IrProgram;
use ir_optimizer::optimize_program;
use ir_to_jvm_class_file::{
    lower_ir_to_jvm_class_file, write_class_file as backend_write_class_file, JvmBackendConfig,
    JvmClassArtifact,
};
use jvm_class_file::{parse_class_file, JvmClassFile};
use nib_ir_compiler::{compile_nib, release_config, BuildConfig};
use nib_type_checker::{check, TypedAst};
use parser::grammar_parser::GrammarASTNode;

#[derive(Debug, Clone)]
pub struct PackageResult {
    pub source: String,
    pub class_name: String,
    pub ast: GrammarASTNode,
    pub typed_ast: TypedAst,
    pub raw_ir: IrProgram,
    pub optimized_ir: IrProgram,
    pub artifact: JvmClassArtifact,
    pub parsed_class: JvmClassFile,
    pub class_bytes: Vec<u8>,
    pub class_file_path: Option<PathBuf>,
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
pub struct NibJvmCompiler {
    pub class_name: String,
    pub build_config: Option<BuildConfig>,
    pub optimize_ir: bool,
    pub emit_main_wrapper: bool,
}

impl Default for NibJvmCompiler {
    fn default() -> Self {
        Self {
            class_name: "NibProgram".to_string(),
            build_config: None,
            optimize_ir: true,
            emit_main_wrapper: true,
        }
    }
}

impl NibJvmCompiler {
    pub fn compile_source(&self, source: &str) -> Result<PackageResult, PackageError> {
        let ast = parse_nib(source).map_err(|err| PackageError::new("parse", err.to_string()))?;
        let type_result = check(ast.clone());
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

        let config = self.build_config.unwrap_or_else(release_config);
        let ir_result = compile_nib(type_result.typed_ast.clone(), config);
        let raw_ir = ir_result.program;
        let optimized_ir = if self.optimize_ir {
            optimize_program(&raw_ir)
        } else {
            raw_ir.clone()
        };
        let artifact = lower_ir_to_jvm_class_file(
            &optimized_ir,
            JvmBackendConfig {
                class_name: self.class_name.clone(),
                emit_main_wrapper: self.emit_main_wrapper,
                ..JvmBackendConfig::new(self.class_name.clone())
            },
        )
        .map_err(|err| PackageError::new("lower-jvm", err.to_string()))?;
        let parsed_class = parse_class_file(&artifact.class_bytes)
            .map_err(|err| PackageError::new("validate-class", err.to_string()))?;

        Ok(PackageResult {
            source: source.to_string(),
            class_name: self.class_name.clone(),
            ast,
            typed_ast: type_result.typed_ast,
            raw_ir,
            optimized_ir,
            class_bytes: artifact.class_bytes.clone(),
            artifact,
            parsed_class,
            class_file_path: None,
        })
    }

    pub fn write_class_file(
        &self,
        source: &str,
        output_dir: impl AsRef<Path>,
    ) -> Result<PackageResult, PackageError> {
        let mut result = self.compile_source(source)?;
        let path = backend_write_class_file(&result.artifact, output_dir)
            .map_err(|err| PackageError::new("write", err.to_string()))?;
        result.class_file_path = Some(path);
        Ok(result)
    }
}

pub fn compile_source(source: &str) -> Result<PackageResult, PackageError> {
    NibJvmCompiler::default().compile_source(source)
}

pub fn pack_source(source: &str) -> Result<PackageResult, PackageError> {
    compile_source(source)
}

pub fn write_class_file(
    source: &str,
    output_dir: impl AsRef<Path>,
) -> Result<PackageResult, PackageError> {
    NibJvmCompiler::default().write_class_file(source, output_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn compiles_nib_to_parseable_class() {
        let result = compile_source("fn main() { let x: u4 = 7; }").unwrap();
        assert!(!result.raw_ir.instructions.is_empty());
        assert!(result
            .parsed_class
            .find_method("_start", Some("()I"))
            .is_some());
    }

    #[test]
    fn pack_source_alias_matches_compile_source() {
        let compiled = compile_source("fn main() { let x: u4 = 7; }").unwrap();
        let packed = pack_source("fn main() { let x: u4 = 7; }").unwrap();
        assert_eq!(compiled.class_bytes, packed.class_bytes);
    }

    #[test]
    fn write_helper_persists_class_file() {
        let output_root = unique_temp_dir("nib-jvm-write");
        fs::create_dir_all(&output_root).unwrap();
        let result = write_class_file("fn main() { let x: u4 = 7; }", &output_root).unwrap();
        let expected = fs::canonicalize(&output_root)
            .unwrap()
            .join("NibProgram.class");
        assert_eq!(result.class_file_path.as_deref(), Some(expected.as_path()));
        let _ = fs::remove_dir_all(output_root);
    }

    #[test]
    fn parse_errors_are_stage_labeled() {
        let err = compile_source("fn").unwrap_err();
        assert_eq!(err.stage, "parse");
    }

    #[test]
    fn type_errors_are_stage_labeled() {
        let err = compile_source("fn main() { let x: bool = 1 +% 2; }").unwrap_err();
        assert_eq!(err.stage, "type-check");
    }

    #[test]
    fn generated_class_runs_on_graalvm_java_via_driver_when_available() {
        let graalvm_home = match std::env::var("GRAALVM_HOME") {
            Ok(value) => PathBuf::from(value),
            Err(_) => return,
        };

        let output_root = unique_temp_dir("nib-jvm-java");
        fs::create_dir_all(&output_root).unwrap();
        write_class_file("fn main() { let x: u4 = 7; }", &output_root).unwrap();
        let driver_source = output_root.join("InvokeNib.java");
        fs::write(
            &driver_source,
            [
                "public final class InvokeNib {",
                "    public static void main(String[] args) {",
                "        System.out.print(NibProgram._start());",
                "    }",
                "}",
            ]
            .join("\n"),
        )
        .unwrap();

        let javac = graalvm_home.join("bin/javac");
        let java = graalvm_home.join("bin/java");
        let javac_output = Command::new(javac)
            .arg("-cp")
            .arg(&output_root)
            .arg(&driver_source)
            .current_dir(&output_root)
            .output()
            .unwrap();
        assert!(javac_output.status.success(), "{javac_output:?}");

        let java_output = Command::new(java)
            .arg("-cp")
            .arg(&output_root)
            .arg("InvokeNib")
            .current_dir(&output_root)
            .output()
            .unwrap();
        assert!(java_output.status.success(), "{java_output:?}");
        let _ = fs::remove_dir_all(output_root);
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nonce}"))
    }
}
