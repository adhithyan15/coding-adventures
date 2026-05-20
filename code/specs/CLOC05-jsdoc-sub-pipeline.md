# CLOC05 — JSDoc Sub-Pipeline: Grammar, Crates, Tag-to-Sidecar Mapping

## Why JSDoc gets its own sub-pipeline

JSDoc is a second language living inside JavaScript comments. It has its own
lexer rules (`@tag` identifiers, `{TypeExpression}` brackets, free-form
description prose), its own grammar (tag-specific syntax for `@param`,
`@template`, `@typedef`, `@implements`, …), and its own type expression mini-
language inherited from the Closure type annotations of the late 2000s.

Trying to mix JSDoc into the JS lexer/parser would break the CLOC02 contract
("JS AST is type-blind"). Putting JSDoc behind a separate sub-pipeline lets:

1. The JS frontend stay JS-shaped — it just emits comment tokens like it does
   for whitespace.
2. JSDoc evolve at its own pace — adding `@satisfies` or future Closure tags
   touches only the JSDoc crates, never the JS frontend.
3. The same JSDoc parser feed other consumers: documentation generators, IDE
   hover providers, and future type-checkers that aren't this Closure clone.

The output is uniform with CLOC04: a `type-sidecar::Sidecar` whose records are
keyed by JS-side `CvId`s.

## Pipeline overview

```text
javascript-ast::Program            (from the JS frontend, CLOC02)
        │
        ▼
jsdoc-comment-extractor            walks the AST + CV log; finds block
        │                          comments anchored to declarations
        ▼
Vec<JsdocComment>                  { anchor_cv: CvId, text: String, cv: CvId }
        │
        ▼
jsdoc-lexer                        wraps GrammarLexer over jsdoc.tokens
        │
        ▼
Vec<jsdoc::Token>                  every token has cv: CvId
        │
        ▼
jsdoc-parser                       wraps GrammarParser over jsdoc.grammar
        │
        ▼
jsdoc-ast::Document                a tree of tags + type expressions
        │
        ▼
jsdoc-types-extractor              walks the JSDoc AST → emits Sidecar
        │
        ▼
type-sidecar::Sidecar              keyed by the JS anchor_cv of each comment
```

Each arrow is an `&mut CorrelationLog` boundary per CLOC03 — every stage
appends contributions.

## Grammar files

New directory:

```text
code/grammars/jsdoc/
  jsdoc.tokens
  jsdoc.grammar
```

We do **not** version JSDoc the way ECMAScript is versioned. JSDoc has no
formal version specification; the Closure compiler, TypeScript's
`checkJs`, ESLint, and JSDoc.app each interpret the dialect slightly
differently. We define one canonical grammar that subsumes the union of the
common tags. Unknown tags don't fail the parser — they're captured as
`UnknownTag { name: String, raw_text: String }` and silently passed through.

### `jsdoc.tokens` outline

```text
# JSDoc tokens — block comment payload after /** is stripped

skip:
  WHITESPACE = /[ \t]+/             # but NOT newlines — they matter for tags
  LEADING_STAR = /\n[ \t]*\*[ \t]?/ # "* " at start of each line is skipped

errors:
  UNTERMINATED_TYPE = /\{[^}]*$/    # type expr running off the end

tokens:
  AT_TAG       = /@[a-zA-Z_$][a-zA-Z0-9_$-]*/
  LBRACE       = "{"
  RBRACE       = "}"
  LBRACKET     = "["
  RBRACKET     = "]"
  LPAREN       = "("
  RPAREN       = ")"
  PIPE         = "|"
  AMP          = "&"
  COMMA        = ","
  COLON        = ":"
  EQUALS       = "="
  ELLIPSIS     = "..."
  QUESTION     = "?"
  BANG         = "!"
  STAR         = "*"
  ANGLE_OPEN   = "<"
  ANGLE_CLOSE  = ">"
  ARROW        = "=>"
  DOT          = "."
  NEWLINE      = /\n/                # tag boundaries

  IDENTIFIER   = /[a-zA-Z_$][a-zA-Z0-9_$]*/
  NUMBER       = /-?[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?/
  STRING       = /"([^"\\]|\\.)*"/
  STRING       = /'([^'\\]|\\.)*'/

  # Free-form description chunks: anything until the next @tag or end.
  # Captured as DESCRIPTION when in description-context (set by the parser
  # via Extension F04-style group switching).

groups:
  description:
    DESCRIPTION_TEXT = /[^\n@][^\n]*/
```

Detailed lexing rules are nontrivial; the full file is implementation work,
not spec work. The point of this section is the token set the grammar will
consume.

### `jsdoc.grammar` outline

```text
document = { tag } ;

tag = AT_TAG [ type_expr ] [ name_path ] [ description ] NEWLINE
    | AT_TAG description NEWLINE  ;

# --- Type expressions ---

type_expr = LBRACE type RBRACE ;

type = union_type ;
union_type = intersection_type { PIPE intersection_type } ;
intersection_type = primary_type { AMP primary_type } ;

primary_type = "null" | "undefined" | "void" | "any" | "unknown"
             | "string" | "number" | "boolean" | "bigint" | "symbol"
             | literal_type
             | named_type
             | function_type
             | record_type
             | array_type
             | tuple_type
             | nullable_type
             | non_nullable_type
             | optional_type
             | variadic_type
             | LPAREN type RPAREN ;

literal_type = STRING | NUMBER | "true" | "false" ;

named_type = IDENTIFIER { DOT IDENTIFIER } [ type_arguments ] ;
type_arguments = ANGLE_OPEN type { COMMA type } ANGLE_CLOSE ;

function_type = "function" LPAREN [ param_list ] RPAREN [ COLON type ] ;
function_type = LPAREN [ param_list ] RPAREN ARROW type ;
param_list = type { COMMA type } ;

record_type = LBRACE [ record_field { COMMA record_field } ] RBRACE ;
record_field = IDENTIFIER [ QUESTION ] COLON type ;

array_type = primary_type LBRACKET RBRACKET ;
tuple_type = LBRACKET [ type { COMMA type } ] RBRACKET ;

nullable_type = QUESTION primary_type ;        # ?Foo means Foo | null
non_nullable_type = BANG primary_type ;        # !Foo means non-null Foo
optional_type = primary_type EQUALS ;          # Foo= means optional Foo
variadic_type = ELLIPSIS primary_type ;        # ...Foo means rest of Foo

# --- Name paths (for @param, @property, etc.) ---

name_path = IDENTIFIER { DOT IDENTIFIER }
          | LBRACKET IDENTIFIER [ EQUALS default_expr ] RBRACKET ;
default_expr = STRING | NUMBER | "true" | "false" | "null" | IDENTIFIER ;
```

Same caveat as the tokens file: the full grammar is implementation work. This
sketch covers the productions the type-extractor depends on.

## Crates

```text
code/packages/rust/
  jsdoc-tokens/          # TokenKind enum shared between lexer + parser + ast
  jsdoc-ast/             # Typed AST: Document, Tag, TypeExpr, NamePath, ...
  jsdoc-lexer/           # Wraps GrammarLexer over code/grammars/jsdoc/jsdoc.tokens
  jsdoc-parser/          # Wraps GrammarParser over code/grammars/jsdoc/jsdoc.grammar
  jsdoc-comment-extractor/  # JS AST → Vec<JsdocComment>
  jsdoc-types-extractor/    # jsdoc-ast::Document → type-sidecar::Sidecar
```

Crate-name conventions follow the repo: `coding-adventures-jsdoc-*`.

### `jsdoc-ast` shape (sketch)

```rust
pub struct Document {
    pub cv: CvId,                      // CV of the comment as a whole
    pub anchor_cv: CvId,               // the JS node this comment annotates
    pub tags: Vec<Tag>,
    pub free_description: Option<String>,  // text before any @tag
}

pub enum Tag {
    Type(TypeTag),                     // @type {T}
    Param(ParamTag),                   // @param {T} name description
    Returns(ReturnsTag),               // @returns {T} description
    Throws(ThrowsTag),                 // @throws {T} description
    Template(TemplateTag),             // @template T, @template {Constraint} T
    Typedef(TypedefTag),               // @typedef {T} Name
    Callback(CallbackTag),             // @callback Name with @param/@returns
    Property(PropertyTag),             // @property {T} name
    Implements(ImplementsTag),         // @implements {Iface}
    ExtendsTag(ExtendsTag),            // @extends {Base}
    Constructor(ConstructorTag),       // @constructor
    Class(ClassTag),                   // @class
    Enum(EnumTag),                     // @enum {T}
    This(ThisTag),                     // @this {T}
    Override(OverrideTag),             // @override
    Abstract(AbstractTag),             // @abstract
    Final(FinalTag),                   // @final
    Readonly(ReadonlyTag),             // @readonly
    Const(ConstTag),                   // @const {T?}
    Public(PublicTag), Protected(ProtectedTag), Private(PrivateTag),
    Deprecated(DeprecatedTag),         // @deprecated message
    Pure(PureTag),                     // @pure (Closure-style)
    NoSideEffects(NoSideEffectsTag),   // @nosideeffects
    Suppress(SuppressTag),             // @suppress {warning1|warning2}
    Description(DescriptionTag),       // explicit @description
    Example(ExampleTag),               // @example block
    See(SeeTag),                       // @see ref
    Author(AuthorTag), Version(VersionTag), License(LicenseTag),
    Unknown(UnknownTag),               // any tag we don't recognize
}

pub enum TypeExpr {
    Primitive(PrimitiveType),          // null, undefined, void, string, number, ...
    Literal(Literal),
    Named { path: Vec<String>, args: Vec<TypeExpr>, cv: CvId },
    Function { params: Vec<TypeExpr>, returns: Box<TypeExpr>, cv: CvId },
    Record { fields: Vec<RecordField>, cv: CvId },
    Array(Box<TypeExpr>, CvId),
    Tuple(Vec<TypeExpr>, CvId),
    Union(Vec<TypeExpr>, CvId),
    Intersection(Vec<TypeExpr>, CvId),
    Nullable(Box<TypeExpr>, CvId),     // ?Foo
    NonNullable(Box<TypeExpr>, CvId),  // !Foo
    Optional(Box<TypeExpr>, CvId),     // Foo=
    Variadic(Box<TypeExpr>, CvId),     // ...Foo
    Any(CvId), Unknown(CvId), Never(CvId),
    Opaque(String, CvId),              // unparseable / future syntax
}
```

Every node has `cv: CvId` per the CLOC02 invariant (the same rule that applies
to `javascript-ast` applies here).

## `jsdoc-comment-extractor` — finding the comments

JSDoc's anchoring rule: a JSDoc block comment (`/** ... */`) is attached to the
**next declaration or expression** that follows it, after any intervening
trivia. The extractor:

1. Walks the source string for `/** ... */` ranges (the JS lexer already
   skipped these, but the CV log has Origin records for them).
2. For each block comment, finds the next "anchorable" AST node — a
   declaration, expression statement, class member, etc.
3. Emits `JsdocComment { cv, anchor_cv, raw_text, comment_origin }`.

Edge cases the extractor handles explicitly (each gets a unit test):

- Multiple `/** */` comments stacked above one declaration → all anchor to it.
- Comment with no following anchorable node → anchored to a synthetic
  "orphan" CvId, surfaced as a warning.
- File-level comment at the top of the file (before any node) → anchored to
  `Program.cv`.
- Comment inside an expression (e.g., `foo(/** @type {string} */ bar)`) →
  anchored to the *next sibling* in expression position, not the next
  statement.
- License/copyright headers (often `/*!` or `/**!`) → skipped by convention.

CV contributions:

```rust
cv.contribute(comment.cv, Contribution {
    source: "jsdoc-comment-extractor",
    tag:    "anchored",
    meta:   json!({ "anchor_cv": anchor.cv }),
});
```

## `jsdoc-types-extractor` — emitting the sidecar

For each `Document` produced by `jsdoc-parser`, the extractor walks its tags
and lowers them into a single `type-sidecar::Record` keyed by the document's
`anchor_cv`. Tag-by-tag table:

| JSDoc tag | Sidecar mapping |
| --- | --- |
| `@type {T}` | `record.ty = lower(T)` |
| `@param {T} name` | append to `record.ty` if it's a `Function`: extends `params` with `FunctionParam{ name, ty: lower(T), optional: T is Optional, rest: T is Variadic }` |
| `@returns {T}` / `@return {T}` | sets `Function.returns = lower(T)` |
| `@throws {T}` | added under `attributes.extension["throws"]` (no first-class field) |
| `@template T` | adds `TypeParam{ name: "T", constraint: None }` to `Function.type_params` (or `Class.type_params` for class tags) |
| `@template {C} T` | same, with `constraint = Some(lower(C))` |
| `@typedef {T} Name` | emits a *separate* Record for the typedef site whose `ty = lower(T)` and whose name is reachable via `NamedRef.defined_at = self.cv`. The current document's anchor gets no `ty` (the typedef is a sibling declaration). |
| `@callback Name` | same as `@typedef` but `lower(T)` is built from the sibling `@param`/`@returns` tags rather than an inline type expression |
| `@property {T} name` | inside an `@typedef`/`@enum`, adds to the record type's `ObjectType.fields` |
| `@implements {Iface}` | adds to `attributes.extension["implements"]` as a `Vec<NamedRef>` (no first-class field; the typechecker resolves implementation conformance) |
| `@extends {Base}` | sets `ClassType.heritage` to `[NamedRef]` |
| `@constructor` / `@class` | promotes the record to `Class { ... }` from `Function { ... }` |
| `@enum {T}` | promotes to an enum-typed record; `T` is the element type, members come from sibling `@property` tags |
| `@this {T}` | sets `FunctionType.this_ty = Some(lower(T))` |
| `@override` | `attributes.override = TriState::True` |
| `@abstract` | `attributes.abstract_ = TriState::True` |
| `@final` | `attributes.extension["final"] = true` (no first-class slot) |
| `@readonly` | `attributes.readonly = TriState::True` |
| `@const {T?}` | `attributes.readonly = TriState::True`; if `T` is present, `record.ty = lower(T)` |
| `@public` / `@protected` / `@private` | `attributes.visibility = ...` |
| `@deprecated [msg]` | `attributes.deprecated = Some(msg.unwrap_or_default())` |
| `@pure` | `attributes.pure = TriState::True` |
| `@nosideeffects` | `attributes.no_side_effects = TriState::True` |
| `@suppress {...}` | `attributes.extension["suppress"] = list` |
| `@description` / free description | `attributes.extension["description"] = text` (passes ignore; doc generators consume) |
| `@example`, `@see`, `@author`, `@version`, `@license` | `attributes.extension["<tag>"] = ...` |
| Unknown tag | `attributes.extension["unknown_tags"]` list of `{ name, text }` |

### Nullability inference

JSDoc's nullability is a maze. The extractor follows this rule:

1. If the type expression is `?Foo` → `record.ty = lower(Foo)`,
   `record.attributes.nullable = TriState::True`.
2. If the type expression is `!Foo` → `record.ty = lower(Foo)`,
   `record.attributes.nullable = TriState::False`.
3. If the type expression is `Foo` and `Foo` is a primitive — Closure's rule
   is that primitives are non-nullable by default. So
   `record.attributes.nullable = TriState::False`.
4. If the type expression is `Foo` and `Foo` is a named type — Closure's rule
   is that named types are nullable by default. So
   `record.attributes.nullable = TriState::True`.

We document this rule prominently in the crate README; it's the most common
source of confusion for JSDoc users.

### Variadic and optional

`Foo=` (optional) and `...Foo` (variadic) only legal inside `@param`. The
extractor flags them on `FunctionParam{optional, rest}` rather than emitting
an `Optional` / `Variadic` `Type`. The Closure typechecker reads the flags
from `FunctionParam` directly.

## CV plumbing per CLOC03

Every stage above takes `&mut CorrelationLog` and appends contributions:

| Stage | Creates IDs | Tags |
| --- | --- | --- |
| `jsdoc-comment-extractor` | (none — IDs come from JS log) | `anchored`, `orphan` |
| `jsdoc-lexer` | one per token | (none — creation only) |
| `jsdoc-parser` | one per AST node via `merge(parents)` | `constructed` with `meta: {"rule": "<grammar_rule>"}` |
| `jsdoc-types-extractor` | (none — operates on JS-side IDs) | `sidecar-emitted` with meta `{"tags_read": [...]}` |

A debugger can then trace: source byte → JS comment token → JSDoc AST node →
sidecar record → typechecker judgment → optimization decision, all keyed by
the same correlation chain.

## Interaction with `jsdoc-comment-extractor`'s "orphan" comments

When a comment doesn't have a downstream anchor (e.g., a file-level comment
followed by nothing parsable), the extractor emits it with `anchor_cv =
Program.cv` and `attributes.extension["orphan"] = true` on the resulting
sidecar record. Tools that want to surface unanchored documentation can
filter on this flag.

## Testing strategy

| Layer | Tests |
| --- | --- |
| `jsdoc-tokens` & `jsdoc-ast` | Construction + serde round-trip per variant |
| `jsdoc-lexer` | Per-token golden fixture; ensures `* ` line prefixes are stripped |
| `jsdoc-parser` | Per-tag fixture: text in → `Document` out; covers each tag in the table above |
| `jsdoc-comment-extractor` | Anchoring edge cases (stacking, orphans, inline expression position, top-of-file) |
| `jsdoc-types-extractor` | Tag-mapping table is exhaustive: every row has at least one fixture |
| Cross-crate | End-to-end: JS source with full JSDoc annotations → merged sidecar matches a golden JSON file |
| Conformance | Run against Closure Compiler's own test corpus (the relevant subset that's not Java-specific) and compare resulting sidecars to a baseline |

Coverage target: 95%+ for the library crates, 90%+ for the extractors.

## Open questions

1. **Markdown in descriptions.** JSDoc.app renders Markdown; Closure's
   compiler doesn't. Our position: the extractor stores raw text in
   `attributes.extension["description"]`; downstream documentation tools are
   free to interpret it as Markdown. The typechecker ignores it. No parser
   support for Markdown.
2. **Inline `@inheritdoc` resolution.** This requires walking up class
   hierarchies. Deferred to the typechecker, not the extractor — the
   extractor just records `@inheritdoc` as an attribute flag.
3. **TypeScript-style JSDoc.** TS has extended JSDoc with `@satisfies`,
   template literal types in JSDoc, etc. MVP supports `@satisfies` (maps to
   `attributes.extension["satisfies"] = lower(T)`); template literal types
   are an Opaque escape until we add them to the JSDoc grammar.
4. **Closure-specific tags we explicitly choose not to support.** Some
   Closure-Compiler-specific tags (`@struct`, `@dict`, `@unrestricted`,
   `@modifies`, `@idGenerator`) affect deep optimizer behavior. MVP captures
   them as attribute extension keys; downstream pass support is per-pass
   scope.
5. **Performance.** A large file with hundreds of JSDoc-annotated symbols
   produces correspondingly many sidecar records. Sidecar size scaling is
   already flagged in CLOC04; no JSDoc-specific concern beyond that.

## What this spec does **not** cover

- The full character-for-character `jsdoc.tokens` and `jsdoc.grammar` files —
  these are implementation deliverables (the grammar files will land
  alongside the lexer crate).
- The Closure typechecker's algorithm for *using* JSDoc-emitted sidecars —
  belongs to the `closure-typechecker` crate.
- The TypeScript types-extractor — symmetric in design but separate spec when
  it becomes scheduled.
- IDE-side display of JSDoc (hover info, completion) — out of scope; the
  parsed `Document` is sufficient for any IDE consumer to build on.
