//! MacroNib source to IIR, composed from the shared preprocessor and Nib.
//!
//! Directives are consumed before Nib's existing guarded parser sees tokens.
//! The compiler retains Nib's type rules and IIR lowering unchanged.

#![warn(missing_docs)]

pub mod dialect;

pub use dialect::MacroNibDialect;

use coding_adventures_macronib_lexer::try_tokenize_macronib;
use coding_adventures_source_preprocessor::{
    preprocess, Bounds, FileId, MemoryFs, PpError, SourceFs, SourceMap,
};
use interpreter_ir::IIRModule;
use lexer::token::Token;
use nib_iir_compiler::CompileError;

/// A lexing, preprocessing, or Nib compilation error.
#[derive(Debug)]
pub enum MacroNibError {
    /// MacroNib's lexer rejected the source.
    Lex(String),
    /// The shared preprocessor rejected a directive or resource use.
    Preprocess(PpError),
    /// Nib's parser, type checker or compiler rejected the expanded source.
    Nib(CompileError),
}

impl std::fmt::Display for MacroNibError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lex(message) => write!(f, "MacroNib lex: {message}"),
            Self::Preprocess(error) => write!(f, "MacroNib preprocess: {error}"),
            Self::Nib(error) => write!(f, "MacroNib Nib stage: {error}"),
        }
    }
}

impl std::error::Error for MacroNibError {}

/// Directive-free Nib tokens and provenance for every token except EOF.
pub struct PreprocessedUnit {
    /// The tokens handed to Nib's parser, including its final EOF sentinel.
    pub tokens: Vec<Token>,
    /// Provenance for every non-EOF token.
    pub map: SourceMap,
}

/// Preprocess one MacroNib translation unit with a caller-supplied filesystem.
pub fn preprocess_source(
    source: &str,
    file: FileId,
    fs: &mut dyn SourceFs,
    bounds: Bounds,
) -> Result<PreprocessedUnit, MacroNibError> {
    let mut tokens = try_tokenize_macronib(source).map_err(MacroNibError::Lex)?;
    let sentinel = tokens.last().cloned();
    dialect::strip_eof(&mut tokens);
    let result = preprocess(tokens, file, &MacroNibDialect, fs, bounds)
        .map_err(MacroNibError::Preprocess)?;
    let mut tokens = result.tokens;
    if let Some(sentinel) = sentinel {
        tokens.push(sentinel);
    }
    Ok(PreprocessedUnit {
        tokens,
        map: result.map,
    })
}

/// Compile MacroNib source with no additional include files.
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, MacroNibError> {
    compile_source_with_includes(source, module_name, &[])
}

/// Compile MacroNib source with an in-memory include set.
pub fn compile_source_with_includes(
    source: &str,
    module_name: &str,
    includes: &[(&str, &str)],
) -> Result<IIRModule, MacroNibError> {
    let mut fs = MemoryFs::new();
    for (name, text) in includes {
        fs.insert(*name, *text);
    }
    let file = fs.insert("<main>", source);
    let unit = preprocess_source(source, file, &mut fs, Bounds::default())?;
    let ast = coding_adventures_nib_parser::parse_nib_tokens(unit.tokens)
        .map_err(|error| MacroNibError::Nib(CompileError::Parse(error.to_string())))?;
    nib_iir_compiler::compile_ast(ast, module_name).map_err(MacroNibError::Nib)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_macro_and_ifdef_select_nib_program() {
        let source = ".set ANSWER 7\n.ifdef ANSWER\nfn main() -> u4 { return ANSWER; }\n.else\nfn main() -> u4 { return 1; }\n.endif\n";
        let actual = compile_source(source, "m").unwrap();
        let expected =
            nib_iir_compiler::compile_source("fn main() -> u4 { return 7; }", "m").unwrap();
        assert_eq!(actual.entry_point, expected.entry_point);
        assert_eq!(actual.language, expected.language);
        assert_eq!(
            format!("{:?}", actual.functions[0].instructions),
            format!("{:?}", expected.functions[0].instructions)
        );
    }

    #[test]
    fn include_and_undefined_ifdef_compose() {
        let source = ".include \"helper.mnib\"\n.ifdef MISSING\nfn main() -> u4 { return 1; }\n.else\nfn main() -> u4 { return helper(); }\n.endif\n";
        let actual = compile_source_with_includes(
            source,
            "m",
            &[("helper.mnib", "fn helper() -> u4 { return 9; }\n")],
        )
        .unwrap();
        let expected = nib_iir_compiler::compile_source(
            "fn helper() -> u4 { return 9; }\nfn main() -> u4 { return helper(); }\n",
            "m",
        )
        .unwrap();
        assert_eq!(actual.functions.len(), expected.functions.len());
        for (left, right) in actual.functions.iter().zip(expected.functions.iter()) {
            assert_eq!(
                format!("{:?}", left.instructions),
                format!("{:?}", right.instructions)
            );
        }
    }

    #[test]
    fn malformed_directives_and_unresolvable_include_are_diagnostics() {
        for source in [
            ".set 1 2\n",
            ".ifdef\n",
            ".else extra\n",
            ".include x\n",
            ".include \"missing.mnib\"\n",
        ] {
            assert!(compile_source(source, "m").is_err(), "{source}");
        }
    }

    #[test]
    fn directives_never_reach_nib_parser() {
        let source = ".set N 5\nfn main() -> u4 { return N; }\n";
        let mut fs = MemoryFs::new();
        let file = fs.insert("<main>", source);
        let unit = preprocess_source(source, file, &mut fs, Bounds::default()).unwrap();
        assert_eq!(unit.map.len(), unit.tokens.len() - 1);
        assert!(!unit
            .tokens
            .iter()
            .any(|token| token.value == ".set" || token.value == "N"));
    }

    #[test]
    fn aligned_sources_have_identical_iir_including_provenance() {
        let macro_source = ".set N 5\nfn main() -> u4 { return N; }\n";
        let nib_source = "\nfn main() -> u4 { return 5; }\n";
        let actual = compile_source(macro_source, "m").unwrap();
        let expected = nib_iir_compiler::compile_source(nib_source, "m").unwrap();
        assert_eq!(format!("{actual:#?}"), format!("{expected:#?}"));
    }

    #[test]
    fn include_cycle_and_unclosed_conditional_are_located_errors() {
        let cycle = compile_source_with_includes(
            ".include \"a.mnib\"\n",
            "m",
            &[("a.mnib", ".include \"a.mnib\"\n")],
        )
        .unwrap_err();
        assert!(matches!(cycle, MacroNibError::Preprocess(_)));
        assert!(cycle.to_string().contains("cycle"), "{cycle}");
        let unclosed = compile_source(".ifdef X\nfn main() {}\n", "m").unwrap_err();
        assert!(matches!(unclosed, MacroNibError::Preprocess(_)));
    }

    #[test]
    fn macro_expansion_obeys_generic_token_budget() {
        let source = ".set MANY 1 + 1 + 1 + 1\nfn main() -> u4 { return MANY; }\n";
        let mut fs = MemoryFs::new();
        let file = fs.insert("<main>", source);
        let bounds = Bounds {
            tokens_produced: 4,
            ..Bounds::default()
        };
        let error = match preprocess_source(source, file, &mut fs, bounds) {
            Err(MacroNibError::Preprocess(error)) => error,
            _ => panic!("expanded token budget must reject source"),
        };
        assert!(error.to_string().contains("token"), "{error}");
    }
}
