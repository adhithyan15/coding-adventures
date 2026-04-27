//! `LanguageBridge` trait -- the single point of integration for language servers.
//!
//! # Design Philosophy: One Trait with Optional Methods
//!
//! Rust's trait system does not support Go-style runtime interface detection
//! (type assertions like `bridge.(HoverProvider)`). Instead, we use a single
//! trait with required + optional methods:
//!
//! - **Required methods** (`tokenize`, `parse`): every language MUST implement these.
//! - **Optional provider methods** (`hover`, `definition`, etc.): default to `None`,
//!   meaning "not supported." A bridge overrides only the methods it supports.
//! - **Capability flags** (`supports_hover`, etc.): default to `false`. The bridge
//!   overrides to `true` for methods it implements. The server uses these flags
//!   to build the capabilities response during `initialize`.
//!
//! # The `Option<Result<...>>` Pattern
//!
//! Optional methods return `Option<Result<T, String>>`:
//! - `None` -> "this feature is not supported" (don't advertise capability)
//! - `Some(Ok(value))` -> supported and succeeded
//! - `Some(Err(msg))` -> supported but failed
//!
//! This three-state return lets the server distinguish "not implemented" from
//! "implemented but errored" without needing separate `supports_*` calls at
//! runtime. However, we still provide `supports_*` methods for capability
//! advertisement because calling methods with dummy values is fragile.
//!
//! # ASTNode Type
//!
//! We use `Box<dyn Any + Send + Sync>` as the AST type. Each language's parser
//! returns its own concrete AST type boxed as `dyn Any`. The bridge downcasts
//! it back to the concrete type inside its handler methods using
//! `ast.downcast_ref::<MyAst>()`.

use crate::types::*;
use std::any::Any;

/// The required and optional interface every language bridge must implement.
///
/// ## Required Methods
///
/// - `tokenize`: lex the source into tokens (for semantic highlighting).
/// - `parse`: parse the source into an AST + diagnostics (for error display).
///
/// ## Optional Methods
///
/// All optional methods have default implementations returning `None` (not
/// supported). Override only the ones your language supports.
///
/// ## Capability Flags
///
/// The `supports_*` methods default to `false`. Override them to `true` for
/// each optional method your bridge implements. The server reads these during
/// `initialize` to build the capabilities response.
pub trait LanguageBridge: Send + Sync {
    // -----------------------------------------------------------------------
    // Required methods
    // -----------------------------------------------------------------------

    /// Lex the source string and return the token stream.
    ///
    /// The tokens are used for semantic highlighting. Each `Token` carries a
    /// `token_type` string (e.g. `"KEYWORD"`, `"IDENTIFIER"`), its `value`,
    /// and its 1-based `line` and `column` position.
    fn tokenize(&self, source: &str) -> Result<Vec<Token>, String>;

    /// Parse the source string and return:
    /// - `ast`: the parsed abstract syntax tree (may be partial on error)
    /// - `diagnostics`: parse errors and warnings as LSP `Diagnostic` objects
    ///
    /// Even when there are syntax errors, `parse` should return a partial AST.
    /// This allows hover, folding, and symbol features to continue working on
    /// the valid portions of the file.
    fn parse(
        &self,
        source: &str,
    ) -> Result<(Box<dyn Any + Send + Sync>, Vec<Diagnostic>), String>;

    // -----------------------------------------------------------------------
    // Optional provider methods
    // -----------------------------------------------------------------------

    /// Return hover information for the AST node at the given position.
    ///
    /// Returns `None` if not supported, `Some(Ok(None))` if supported but
    /// nothing to show at this position, `Some(Ok(Some(result)))` for content.
    fn hover(
        &self,
        _ast: &dyn Any,
        _pos: Position,
    ) -> Option<Result<Option<HoverResult>, String>> {
        None
    }

    /// Return the location where the symbol at `pos` was declared.
    fn definition(
        &self,
        _ast: &dyn Any,
        _pos: Position,
        _uri: &str,
    ) -> Option<Result<Option<Location>, String>> {
        None
    }

    /// Return all uses of the symbol at `pos`.
    fn references(
        &self,
        _ast: &dyn Any,
        _pos: Position,
        _uri: &str,
        _include_decl: bool,
    ) -> Option<Result<Vec<Location>, String>> {
        None
    }

    /// Return autocomplete suggestions valid at `pos`.
    fn completion(
        &self,
        _ast: &dyn Any,
        _pos: Position,
    ) -> Option<Result<Vec<CompletionItem>, String>> {
        None
    }

    /// Return the set of text edits needed to rename the symbol at `pos`.
    fn rename(
        &self,
        _ast: &dyn Any,
        _pos: Position,
        _new_name: &str,
    ) -> Option<Result<Option<WorkspaceEdit>, String>> {
        None
    }

    /// Return semantic token data for the whole document.
    fn semantic_tokens(
        &self,
        _source: &str,
        _tokens: &[Token],
    ) -> Option<Result<Vec<SemanticToken>, String>> {
        None
    }

    /// Return the outline tree for the given AST.
    fn document_symbols(
        &self,
        _ast: &dyn Any,
    ) -> Option<Result<Vec<DocumentSymbol>, String>> {
        None
    }

    /// Return collapsible regions derived from the AST structure.
    fn folding_ranges(
        &self,
        _ast: &dyn Any,
    ) -> Option<Result<Vec<FoldingRange>, String>> {
        None
    }

    /// Return signature hint information for the call at `pos`.
    fn signature_help(
        &self,
        _ast: &dyn Any,
        _pos: Position,
    ) -> Option<Result<Option<SignatureHelpResult>, String>> {
        None
    }

    /// Return the text edits needed to format the document.
    fn format(&self, _source: &str) -> Option<Result<Vec<TextEdit>, String>> {
        None
    }

    // -----------------------------------------------------------------------
    // Capability flags
    // -----------------------------------------------------------------------
    //
    // These are used by `build_capabilities()` to determine which LSP
    // capabilities to advertise. Override to `true` for each optional method
    // your bridge implements.

    /// Does this bridge support hover tooltips?
    fn supports_hover(&self) -> bool {
        false
    }
    /// Does this bridge support "Go to Definition"?
    fn supports_definition(&self) -> bool {
        false
    }
    /// Does this bridge support "Find All References"?
    fn supports_references(&self) -> bool {
        false
    }
    /// Does this bridge support autocomplete?
    fn supports_completion(&self) -> bool {
        false
    }
    /// Does this bridge support symbol rename?
    fn supports_rename(&self) -> bool {
        false
    }
    /// Does this bridge support semantic token highlighting?
    fn supports_semantic_tokens(&self) -> bool {
        false
    }
    /// Does this bridge support the document outline?
    fn supports_document_symbols(&self) -> bool {
        false
    }
    /// Does this bridge support code folding?
    fn supports_folding_ranges(&self) -> bool {
        false
    }
    /// Does this bridge support signature help?
    fn supports_signature_help(&self) -> bool {
        false
    }
    /// Does this bridge support document formatting?
    fn supports_format(&self) -> bool {
        false
    }
}
