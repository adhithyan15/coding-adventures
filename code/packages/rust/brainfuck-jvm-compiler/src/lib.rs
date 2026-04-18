use std::fmt;
use std::path::{Path, PathBuf};

use brainfuck::parser::parse_brainfuck;
use brainfuck_ir_compiler::{compile, release_config, BuildConfig};
use compiler_ir::IrProgram;
use ir_optimizer::optimize_program;
use ir_to_jvm_class_file::{
    lower_ir_to_jvm_class_file, write_class_file as backend_write_class_file, JvmBackendConfig,
    JvmClassArtifact,
};
use jvm_class_file::{parse_class_file, JvmClassFile};
use parser::grammar_parser::GrammarASTNode;

#[derive(Debug, Clone)]
pub struct PackageResult {
    pub source: String,
    pub filename: String,
    pub class_name: String,
    pub ast: GrammarASTNode,
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
pub struct BrainfuckJvmCompiler {
    pub filename: String,
    pub class_name: String,
    pub build_config: Option<BuildConfig>,
    pub optimize_ir: bool,
    pub emit_main_wrapper: bool,
}

impl Default for BrainfuckJvmCompiler {
    fn default() -> Self {
        Self {
            filename: "program.bf".to_string(),
            class_name: "BrainfuckProgram".to_string(),
            build_config: None,
            optimize_ir: true,
            emit_main_wrapper: true,
        }
    }
}

impl BrainfuckJvmCompiler {
    pub fn compile_source(&self, source: &str) -> Result<PackageResult, PackageError> {
        let ast =
            parse_brainfuck(source).map_err(|err| PackageError::new("parse", err.to_string()))?;
        let config = self.build_config.clone().unwrap_or_else(release_config);
        let ir_result = compile(&ast, &self.filename, config)
            .map_err(|err| PackageError::new("ir-compile", err))?;
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
            filename: self.filename.clone(),
            class_name: self.class_name.clone(),
            ast,
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
    BrainfuckJvmCompiler::default().compile_source(source)
}

pub fn pack_source(source: &str) -> Result<PackageResult, PackageError> {
    compile_source(source)
}

pub fn write_class_file(
    source: &str,
    output_dir: impl AsRef<Path>,
) -> Result<PackageResult, PackageError> {
    BrainfuckJvmCompiler::default().write_class_file(source, output_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn compiles_brainfuck_to_parseable_class() {
        let result = compile_source("+.").unwrap();
        assert!(!result.raw_ir.instructions.is_empty());
        assert!(!result.class_bytes.is_empty());
        assert!(result
            .parsed_class
            .find_method("_start", Some("()I"))
            .is_some());
    }

    #[test]
    fn pack_source_alias_matches_compile_source() {
        let compiled = compile_source("+.").unwrap();
        let packed = pack_source("+.").unwrap();
        assert_eq!(compiled.class_bytes, packed.class_bytes);
    }

    #[test]
    fn write_helper_persists_class_file() {
        let output_root = unique_temp_dir("brainfuck-jvm-write");
        fs::create_dir_all(&output_root).unwrap();
        let result = write_class_file("+.", &output_root).unwrap();
        let expected = fs::canonicalize(&output_root)
            .unwrap()
            .join("BrainfuckProgram.class");
        assert_eq!(result.class_file_path.as_deref(), Some(expected.as_path()));
        assert!(expected.exists());
        let _ = fs::remove_dir_all(output_root);
    }

    #[test]
    fn parse_errors_are_stage_labeled() {
        let err = compile_source("[").unwrap_err();
        assert_eq!(err.stage, "parse");
    }

    #[test]
    fn generated_class_runs_on_graalvm_java_when_available() {
        let graalvm_home = match std::env::var("GRAALVM_HOME") {
            Ok(value) => PathBuf::from(value),
            Err(_) => return,
        };

        let output_root = unique_temp_dir("brainfuck-jvm-java");
        fs::create_dir_all(&output_root).unwrap();
        write_class_file(&("+".repeat(65) + "."), &output_root).unwrap();

        let output = Command::new(graalvm_home.join("bin/java"))
            .arg("-cp")
            .arg(&output_root)
            .arg("BrainfuckProgram")
            .output()
            .unwrap();

        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"A");
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
