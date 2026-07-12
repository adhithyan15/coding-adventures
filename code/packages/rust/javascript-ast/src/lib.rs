//! Backend-agnostic JavaScript AST.
//!
//! Per [CLOC02](../../specs/CLOC02-javascript-ast.md) and
//! [CLOC09](../../specs/CLOC09-ast-taxonomy.md). The AST that the
//! JavaScript frontend emits and that every downstream consumer reads.
//!
//! # Consumers
//!
//! - The Closure-Compiler-clone's typechecker, optimization passes,
//!   emitter.
//! - The future V8-in-Rust clone's bytecode lowering pass.
//! - JSDoc and TypeScript types extractors (they walk the AST looking for
//!   comment anchors).
//! - IDE tooling — LSP, hover, completion, debug adapters.
//!
//! Because so many consumers depend on it, the AST is intentionally
//! **small** — defined just by the node types in this crate plus the
//! invariants below.
//!
//! # Backend-agnostic invariants (CLOC02 §"Backend-agnostic invariants")
//!
//! 1. **No backend types in AST nodes.** The crate imports only from
//!    `correlation-vector`, `javascript-tokens`, and `serde`. Never from
//!    `closure-*`, `type-sidecar`, IR/bytecode crates.
//! 2. **Per-node identity is an optional CV id, not a span.** Spans live
//!    in CV `Origin` records in a parallel log. The AST stores only the
//!    lightweight `Option<CvId>` — see [§ Per-node CvId is optional]
//!    below.
//! 3. **No mutation in the public surface.** Passes return new trees;
//!    they do not mutate inputs.
//! 4. **Version tag on `Program`, not on every node.** Per-node version
//!    tags would bloat every variant; the parser refuses to emit nodes
//!    that aren't legal at the requested version, so downstream readers
//!    can assume "every variant present is legal."
//! 5. **No type information in the AST.** Types live in
//!    `coding-adventures-type-sidecar`, keyed by `CvId`.
//! 6. **No optimization metadata in the AST.** "This is dead," "this was
//!    inlined" — all of that lives in CV contributions.
//!
//! # Per-node CvId is optional
//!
//! Every node carries `cv: Option<CvId>`. CV tracing is opt-in per
//! program, not per-pass and not per-build. Two equally-supported modes:
//!
//! 1. **Tracing enabled** — every node is constructed with
//!    `cv: Some(id)`. The frontend assigns ids during lex/parse, passes
//!    fork new ids when they rewrite a node, the emitter reads
//!    `cv.expect(...)` on each token and writes a source-map mapping.
//!    Full source-map support, full provenance queries.
//!
//! 2. **Tracing disabled** — every node is constructed with `cv: None`.
//!    The frontend skips id assignment, passes don't fork (there's
//!    nothing to fork from), the emitter writes no source map. Useful
//!    for synthetic test programs, code-transform tools that don't need
//!    source maps, and downstream consumers like the future
//!    V8-on-LANG-VM that produces its own debugger metadata.
//!
//! Modes are per-program; mixing within one program is allowed but
//! uncommon. The pipeline has one behavior across both modes — passes
//! match on `node.cv`: `Some(parent_id)` forks, `None` no-ops.
//!
//! # Wire format
//!
//! Every variant serializes to JSON with a `"type": "..."` tag matching
//! ESTree exactly. Internal field names are camelCase via
//! `#[serde(rename_all = "camelCase")]`. The `cv` field carries
//! `#[serde(skip_serializing_if = "Option::is_none", default)]` so
//! untraced ASTs match ESTree's wire format byte-for-byte (no `cv` key
//! in the output at all).
//!
//! # What v1 contains (CLOC09 Phase 1)
//!
//! - `Program` root with `body: Vec<ProgramItem>`.
//! - 10 [`Statement`] variants (see [`statement`]).
//! - 14 [`Expression`] variants (see [`expression`]).
//! - 2 [`Declaration`] variants (see [`declaration`]).
//!
//! Subsequent phases per CLOC09 add more variants additively without
//! changing the shape of existing ones.
//!
//! # Note on `CvId`
//!
//! CLOC02 specifies `cv: CvId` where `CvId` is a "copy-cheap newtype".
//! The current `correlation-vector` crate represents CV IDs as plain
//! `String` (see `correlation-vector` v0.x). To avoid coupling the two
//! crates' release cycles, this crate type-aliases [`CvId`] to `String`.
//! A future PR can promote it to a real newtype without churn — every
//! field is typed `CvId`, not `String`, even though they're the same
//! type today.

use coding_adventures_javascript_tokens::EsVersion;
use serde::{Deserialize, Serialize};

pub mod declaration;
pub mod expression;
pub mod statement;

/// Serde adapter that serializes [`EsVersion`] as a string via its
/// `Display` impl and deserializes via its `FromStr`.
///
/// The `javascript-tokens` crate intentionally has zero dependencies
/// (not even serde) so it can be reused by tools that don't want a
/// serialization dep. We bridge that here via the
/// [`#[serde(with = "...")]`] hook on the `version` field of
/// [`Program`].
mod es_version_serde {
    use coding_adventures_javascript_tokens::EsVersion;
    use serde::{Deserialize, Deserializer, Serializer};
    use std::str::FromStr;

    pub fn serialize<S>(v: &EsVersion, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(&v.to_string())
    }

    pub fn deserialize<'de, D>(d: D) -> Result<EsVersion, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        EsVersion::from_str(&s).map_err(serde::de::Error::custom)
    }
}

// Re-export the leaf types so downstream code does
// `use coding_adventures_javascript_ast::{Program, IfStatement,
// BinaryOperator}` without learning the module layout.
pub use declaration::{
    BindingTarget, ClassDeclaration, Declaration, FunctionDeclaration, FunctionParam, VarKind,
    VariableDeclaration, VariableDeclarator,
};
pub use expression::{
    ArrayExpression, ArrowBody, ArrowFunctionExpression, AssignmentExpression,
    AssignmentOperator, AssignmentTarget,
    BigIntLiteral, BinaryExpression, BinaryOperator, BooleanLiteral, CallExpression,
    ClassExpression, ClassMember, MethodDefinition, MethodKind, PropertyDefinition,
    ConditionalExpression, Expression, FunctionExpression, Identifier, LogicalExpression,
    ChainExpression, LogicalOperator, MemberExpression,
    NewExpression,
    NullLiteral, NumericLiteral, ObjectExpression, ObjectMember, OptionalCallExpression,
    OptionalMemberExpression, PrivateName, Property, PropertyKey, PropertyKind,
    RegExpLiteral,
    SequenceExpression, SpreadElement,
    StringLiteral, TaggedTemplateExpression, TemplateElement, TemplateLiteral, UnaryExpression, UnaryOperator,
    UndefinedLiteral, UpdateExpression, UpdateOperator,
    AwaitExpression, ImportExpression, ImportMeta, NewTarget, Super, ThisExpression, YieldExpression,
};
pub use statement::{
    BlockStatement, BreakStatement, CatchClause, ContinueStatement, DebuggerStatement,
    DoWhileStatement, EmptyStatement, ExpressionStatement, ForInStatement, ForInit, ForOfStatement,
    ForStatement, IfStatement, LabeledStatement, ReturnStatement, Statement, SwitchCase,
    SwitchStatement, ThrowStatement, TryStatement, WhileStatement, WithStatement,
};

/// A correlation-vector identifier. Aliased to `String` for v1 to match
/// the current `correlation-vector` crate's representation. See the
/// crate-level docs note on `CvId` for the migration plan.
pub type CvId = String;

/// The top of the AST. Every JavaScript compile produces exactly one
/// `Program`. Holds the ES version that produced the tree, the
/// script/module discriminator, an optional CV id, and the statement
/// + declaration body.
///
/// Per CLOC02 the `version` field is recorded *only here* — individual
/// nodes never carry a version. Downstream readers can assume any
/// variant they see is legal at `self.version`.
// `Eq` is intentionally NOT derived — NumericLiteral.value is f64,
// which has no Eq impl (NaN != NaN). PartialEq is enough for tests
// and for the pass equality checks that compare two program trees.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Program {
    /// Optional CV identifier for this program. See [§ Per-node CvId is
    /// optional](crate#per-node-cvid-is-optional).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,

    /// The ECMAScript edition the parser used. Serializes as the
    /// canonical version string ("Es2025", "Es5", etc.) via
    /// [`es_version_serde`].
    #[serde(with = "es_version_serde")]
    pub version: EsVersion,

    /// Whether this source was parsed as a script or a module. The
    /// parser picks based on caller hint, file extension, or shebang.
    /// Closure passes and the V8 clone both need this — module and
    /// script have different top-level scoping rules.
    pub source_type: SourceType,

    /// Top-level program items — statements and declarations.
    ///
    /// Defaults to empty so existing callers that constructed `Program`
    /// before CLOC09 Phase 1 still work without changes.
    #[serde(default)]
    pub body: Vec<ProgramItem>,
}

/// A top-level item in a `Program`'s body — either a statement or a
/// declaration. ESTree models top-level as `Statement | ModuleDeclaration`
/// and lifts declarations into the statement enum; we keep declarations
/// separate so passes that care about them specifically
/// (`closure-pass-rename`, `closure-pass-treeshake`,
/// `closure-pass-remove-unused-vars`) traverse `Vec<Declaration>`
/// directly. Phase 4 adds the `ModuleDeclaration` variant.
// The `Statement` variant is intentionally large (it embeds the full statement
// enum); boxing it would ripple through the public AST API and every consumer
// that pattern-matches these variants, so we accept the size difference here.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProgramItem {
    Statement(Statement),
    Declaration(Declaration),
}

/// Whether a [`Program`] was parsed as a script or a module.
///
/// The top-level scoping rules differ between the two: scripts have a
/// shared global scope and allow non-strict-mode features by default;
/// modules are always strict and have per-file scopes plus
/// `import`/`export` syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceType {
    Script,
    Module,
}

impl Program {
    /// Construct a traced `Program` — the cv is `Some(cv)`.
    ///
    /// This is the constructor existing callers use (parser, scaffold
    /// tests). It stays here so the CLOC09 Phase 1 PR doesn't churn 11
    /// downstream crates that already call `Program::new`.
    pub fn new(cv: CvId, version: EsVersion, source_type: SourceType) -> Self {
        Self {
            cv: Some(cv),
            version,
            source_type,
            body: Vec::new(),
        }
    }

    /// Construct an untraced `Program` — `cv` is `None`.
    ///
    /// Use this when CV tracing is disabled: synthetic test programs,
    /// code-transform tools that don't emit source maps, or downstream
    /// consumers like the future V8-on-LANG-VM frontend that produces
    /// its own debugger metadata.
    pub fn new_untraced(version: EsVersion, source_type: SourceType) -> Self {
        Self {
            cv: None,
            version,
            source_type,
            body: Vec::new(),
        }
    }

    /// Replace the body and return self (builder-style). Cheap and
    /// chainable for test-fixture construction.
    pub fn with_body(mut self, body: Vec<ProgramItem>) -> Self {
        self.body = body;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_new_is_traced() {
        let program = Program::new(
            "synthetic.1".to_string(),
            EsVersion::Es2025,
            SourceType::Module,
        );

        assert_eq!(program.cv.as_deref(), Some("synthetic.1"));
        assert_eq!(program.version, EsVersion::Es2025);
        assert_eq!(program.source_type, SourceType::Module);
        assert!(program.body.is_empty());
    }

    #[test]
    fn program_new_untraced_has_none_cv() {
        let program = Program::new_untraced(EsVersion::Es2025, SourceType::Script);
        assert_eq!(program.cv, None);
        assert_eq!(program.source_type, SourceType::Script);
        assert!(program.body.is_empty());
    }

    #[test]
    fn program_supports_both_source_types() {
        let script = Program::new(
            "s.1".to_string(),
            EsVersion::Es5,
            SourceType::Script,
        );
        let module = Program::new(
            "m.1".to_string(),
            EsVersion::Es2025,
            SourceType::Module,
        );

        assert_eq!(script.source_type, SourceType::Script);
        assert_eq!(module.source_type, SourceType::Module);
        assert_ne!(script.source_type, module.source_type);
    }

    #[test]
    fn program_clone_eq() {
        let p1 = Program::new(
            "c.1".to_string(),
            EsVersion::Es2020,
            SourceType::Script,
        );
        let p2 = p1.clone();
        assert_eq!(p1, p2);
    }

    #[test]
    fn source_type_is_copy() {
        // SourceType derives Copy so it's cheap to pass around. Compile-
        // time check via the `Copy` bound.
        fn assert_copy<T: Copy>() {}
        assert_copy::<SourceType>();
    }

    #[test]
    fn version_is_copy() {
        // EsVersion is also Copy.
        fn assert_copy<T: Copy>() {}
        assert_copy::<EsVersion>();
    }

    #[test]
    fn traced_program_serializes_cv_field() {
        let program = Program::new(
            "p.1".to_string(),
            EsVersion::Es2025,
            SourceType::Module,
        );
        let json = serde_json::to_string(&program).expect("serialize");
        assert!(
            json.contains("\"cv\":\"p.1\""),
            "traced Program should include cv field; got {}",
            json
        );
    }

    #[test]
    fn untraced_program_omits_cv_field() {
        let program = Program::new_untraced(EsVersion::Es2025, SourceType::Module);
        let json = serde_json::to_string(&program).expect("serialize");
        assert!(
            !json.contains("\"cv\""),
            "untraced Program must omit cv field; got {}",
            json
        );
    }

    #[test]
    fn program_round_trips_via_serde() {
        let p1 = Program::new(
            "round.1".to_string(),
            EsVersion::Es2025,
            SourceType::Module,
        );
        let json = serde_json::to_string(&p1).expect("serialize");
        let p2: Program = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(p1, p2);
    }

    #[test]
    fn untraced_program_round_trips_via_serde() {
        let p1 = Program::new_untraced(EsVersion::Es5, SourceType::Script);
        let json = serde_json::to_string(&p1).expect("serialize");
        let p2: Program = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(p1, p2);
    }
}
