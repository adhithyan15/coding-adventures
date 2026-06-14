# LANG48 — Typed Syntax (TW05-A)

## Overview

LANG48 implements **TW05-A**: the first bootstrap stage of the self-hosted Twig
compiler.  It extends the parser, AST, and IR compiler to accept the typed
syntax described in `TW05-self-hosted-refined-compiler.md`, then lowers typed
programs to dynamic IIR by erasing annotations — so annotated programs execute
on the existing VM without any type-system support.

Type *checking* and refinement *verification* come in TW05-B (base type checker)
and TW05-C (refinement bridge).  LANG48 only concerns itself with **parse ↔
AST ↔ erasure**.

---

## Acceptance criteria (TW05-A)

1. Typed syntax round-trips through parse/format.
2. Annotation-free Twig still parses unchanged.
3. Annotated programs (records, unions, match) run after erasure.
4. Golden parser tests cover every new production.

---

## New syntax

### `(typed …)` module clause

```scheme
(module compiler/lexer
  (typed strict)
  (export lex)
  (import compiler/token compiler/span))
```

Modes: `strict`, `lenient`, `off`.  The module clause is stored in
`Program::module_info` and passed through to the type checker in TW05-B.
In TW05-A it is **erased** — it emits no IIR.

---

### Type expressions (`type_annotation` grammar extended)

The former LANG23 `type_annotation` rule only accepted bare kind names
(`int`, `any`, `bool`) and two parenthesised forms (`(Int lo hi)`,
`(Member int (…))`).  LANG48 replaces it with a fully recursive
**s-expression grammar**:

```
type_annotation = LPAREN { type_annotation } RPAREN | NAME | INTEGER ;
```

This accepts any s-expression made of names and integers, covering:

| TW05 syntax              | Example                        |
|--------------------------|--------------------------------|
| Bare alias               | `Nat`, `String`, `TokenKind`   |
| Range                    | `(Int 0 128)`                  |
| Membership               | `(Member int (1 2 3))`         |
| Parametric               | `(Index source-len)`           |
| Generic                  | `(List Expr)`, `(Option Char)` |
| Dependent fn type        | `(fn (n) (Int 0 n))`           |
| Where predicate          | `(where Int (and ...))`        |
| Wildcard bound           | `(Int 1 _)` (`_` is a NAME)   |

The existing LANG23 `TypeAnnotation` variants are still recognised by the
extractor and converted to `RefinedType` for the refinement checker.
Unknown forms are stored as `TypeAnnotation::Opaque(TypeExpr)` for
TW05-B to interpret.

---

### Type aliases

```scheme
(type Nat       (Int 0 _))
(type Byte      (Int 0 256))
(type CharCode  (Int 0 1114112))
(type RegId     (fn (frame-size) (Int 0 frame-size)))
(type Index     (fn (len) (Int 0 len)))
```

A top-level form.  Stored as `Form::TypeAlias` in the AST.  **Erased in
TW05-A** — the type checker in TW05-B will expand them.

---

### Records

```scheme
(record Span
  (source-id : SourceId)
  (start     : (Index source-len))
  (end       : (Index source-len)))

(record Token
  (kind   : TokenKind)
  (lexeme : String)
  (span   : Span))
```

A top-level form.  In TW05-A, a `(record Name f1 f2 … fn)` declaration
is **erased into a set of IIR functions**:

| Generated function                  | IIR description                                      |
|-------------------------------------|------------------------------------------------------|
| `(Name f1 f2 … fn)`                 | Constructor: `(cons f1 (cons f2 (… (cons fn nil))))`|
| `(name-f1 r)`, …, `(name-fn r)`     | Positional accessors: `(car r)`, `(car (cdr r))`, … |
| `(name? v)`                         | Type predicate: `(pair? v)` (crude; refined in B)   |

Field names in accessor functions are lower-cased and hyphenated with the
record name: `Span` + field `source-id` → accessor `span-source-id`.

---

### Tagged unions

```scheme
(union Expr
  (IntLit    (value : Int)  (span : Span))
  (BoolLit   (value : Bool) (span : Span))
  (NameRef   (name : Symbol)(span : Span))
  (IfExpr    (cond : Expr)  (then : Expr) (else : Expr) (span : Span))
  (CallExpr  (callee : Expr)(args : (List Expr))        (span : Span)))
```

A top-level form.  In TW05-A, each variant with declaration index `i`
is erased into IIR functions using **integer tags**:

| Generated function                      | IIR description                                                     |
|-----------------------------------------|---------------------------------------------------------------------|
| `(VarName f1 … fn)`                    | Constructor: `(cons i (cons f1 (… (cons fn nil))))`                |
| `(VarName? v)`                          | Predicate: `(= (car v) i)`                                          |
| `(varname-f1 v)`, …, `(varname-fn v)`  | Accessors: `(car (cdr v))`, `(car (cdr (cdr v)))`, …               |

The integer tag `i` is the zero-based index of the variant in the
`(union …)` declaration.  The tag is stable within a compilation unit.

---

### Match expressions

```scheme
(match expr
  ((IntLit value span) ...)
  ((NameRef name span) ...)
  (_ ...))
```

An expression form added to the `compound` grammar rule.  In TW05-A,
a match is erased into a chain of `if` / `let` expressions that:

1. Evaluate the scrutinee once into a fresh register.
2. For each variant arm `(VarName b1 b2 … bn)`:
   - Check `(= (car scrutinee) variant-tag)`.
   - On true: bind fields with `(car (cdr …))` calls and evaluate the body.
3. For a bare-name arm (wildcard binding): bind the scrutinee to the name.
4. For a `_` arm: evaluate the body with no extra binding.

The compiler tracks variant tags in a table populated when `(union …)`
declarations are processed.  An arm whose pattern name is unknown in the
variant table is treated as a wildcard binding, not an error (for
forward-compatible typing).

---

## Grammar changes

### `twig.tokens` — new keywords

```
typed    ; (typed strict/lenient/off)
type     ; (type Name …)
record   ; (record Name …)
union    ; (union Name …)
match    ; (match expr …)
```

These names become reserved in typed Twig modules.  Existing unannotated
code is unchanged since none of these names were legal bindings before.

### `twig.grammar` — new productions

```
; Module clause extended with typed mode
module_clause  = export_clause | import_clause | typed_clause ;
typed_clause   = LPAREN "typed" NAME RPAREN ;

; Form extended with new top-level declarations
form = define | type_alias | record_def | union_def | expr ;

; Type alias
type_alias = LPAREN "type" NAME type_annotation RPAREN ;

; Record definition
record_def   = LPAREN "record" NAME { record_field } RPAREN ;
record_field = LPAREN NAME COLON type_annotation RPAREN ;

; Union definition
union_def     = LPAREN "union" NAME { union_variant } RPAREN ;
union_variant = LPAREN NAME { record_field } RPAREN ;

; Type annotation (general recursive form, replaces LANG23's restrictive grammar)
type_annotation = LPAREN { type_annotation } RPAREN | NAME | INTEGER ;

; Match expression added to compound
match_form  = LPAREN "match" expr { match_arm } RPAREN ;
match_arm   = LPAREN match_pat expr { expr } RPAREN ;
match_pat   = LPAREN NAME { NAME } RPAREN | NAME ;

compound = if_form | let_form | begin_form | lambda_form | quote_form | match_form | apply ;
```

---

## AST additions (`twig-parser`)

### New types in `ast_nodes.rs`

```rust
// General s-expression type representation
pub enum TypeExpr { Name(String), Int(i64), List(Vec<TypeExpr>) }

// Extend TypeAnnotation with an opaque fallback for TW05-B
pub enum TypeAnnotation {
    // … existing LANG23 variants …
    Opaque(TypeExpr),           // TW05-A: unknown type expr, stored for TW05-B
}

// Module metadata
pub enum TypedMode { Strict, Lenient, Off }
pub struct ModuleInfo { pub name: String, pub typed_mode: Option<TypedMode>,
                        pub exports: Vec<String>, pub imports: Vec<String> }

// Top-level declarations
pub struct TypeAlias  { pub name: String, pub expr: TypeExpr, pub line: usize, pub column: usize }
pub struct RecordField{ pub name: String, pub type_annotation: TypeAnnotation }
pub struct RecordDef  { pub name: String, pub fields: Vec<RecordField>, pub line: usize, pub column: usize }
pub struct UnionVariant { pub name: String, pub fields: Vec<RecordField> }
pub struct UnionDef   { pub name: String, pub variants: Vec<UnionVariant>, pub line: usize, pub column: usize }

// Match expression
pub enum MatchPat {
    Variant { name: String, bindings: Vec<String> },
    Wildcard,
    Binding(String),
}
pub struct MatchArm { pub pat: MatchPat, pub body: Vec<Expr> }
pub struct Match    { pub scrutinee: Box<Expr>, pub arms: Vec<MatchArm>, pub line: usize, pub column: usize }
```

### Extend existing types

```rust
pub enum Expr {
    // … existing variants …
    Match(Match),               // TW05-A
}

pub enum Form {
    Define(Define),
    Expr(Expr),
    TypeAlias(TypeAlias),       // TW05-A
    RecordDef(RecordDef),       // TW05-A
    UnionDef(UnionDef),         // TW05-A
}

pub struct Program {
    pub forms: Vec<Form>,
    pub module_info: Option<ModuleInfo>,    // TW05-A
}
```

---

## IR compiler changes (`twig-ir-compiler`)

### `Form::TypeAlias` — erased

```rust
Form::TypeAlias(_) => {}    // compile-time only; no IIR emitted in TW05-A
```

### `Form::RecordDef` — generate constructor + accessors

For `(record Span (source-id : SourceId) (start : Nat) (end : Nat))`,
emit three IIR functions:

1. `Span(source-id, start, end)` → builds `(cons source-id (cons start (cons end nil)))`
2. `span-source-id(r)` → `(car r)`
3. `span-start(r)` → `(car (cdr r))`
4. `span-end(r)` → `(car (cdr (cdr r)))`
5. `span?(v)` → `(pair? v)`

### `Form::UnionDef` — generate constructors + predicates + accessors

For variant `(IntLit (value : Int) (span : Span))` at index 0 in `(union Expr …)`:

1. `IntLit(value, span)` → `(cons 0 (cons value (cons span nil)))`
2. `IntLit?(v)` → `(= (car v) 0)`
3. `intlit-value(v)` → `(car (cdr v))`
4. `intlit-span(v)` → `(car (cdr (cdr v)))`

A `variant_tags: HashMap<String, usize>` table in the compiler maps
`"IntLit" → 0`, `"BoolLit" → 1`, etc.  The table is consulted when
lowering `match` arms.

### `Expr::Match` — lowered to if/let chain

```scheme
; (match scrutinee ((IntLit value span) body-int) (_ default))
; lowers to:
(let ((#matched scrutinee))
  (if (= (car #matched) 0)
    (let ((value (car (cdr #matched)))
          (span  (car (cdr (cdr #matched)))))
      body-int)
    default))
```

Arms are processed left-to-right.  After the last arm, if no wildcard
was present, `nil` is the fallthrough value.

---

## LSP package changes

The `twig-formatter`, `twig-semantic-tokens`, `twig-folding-ranges`, and
`twig-hover` crates have exhaustive `match` arms on `Form` and `Expr`.
LANG48 adds stubs for the new variants so the workspace compiles:

- `Form::TypeAlias | Form::RecordDef | Form::UnionDef` — formatted as
  their s-expression surface syntax; no tokens / no folding in LSP tools.
- `Expr::Match` — formatter: `(match …)` surface; semantic-tokens: `match`
  keyword + sub-expressions; folding: multi-line match folds; hover:
  recurse into scrutinee + arm bodies.

---

## Affected crates / version bumps

| Crate               | Old version | New version |
|---------------------|-------------|-------------|
| `twig-parser`       | 0.2.0       | 0.3.0       |
| `twig-ir-compiler`  | (current)   | +patch      |
| `twig-formatter`    | (current)   | +patch      |
| `twig-semantic-tokens` | (current) | +patch     |
| `twig-folding-ranges`  | (current) | +patch     |
| `twig-hover`        | (current)   | +patch      |

---

## Test plan

### `twig-parser`

```
parse_typed_module_clause            ; (module foo (typed strict))
parse_typed_lenient_clause           ; (module foo (typed lenient))
parse_type_alias                     ; (type Nat (Int 0 _))
parse_record_def_simple              ; (record Span (start : Nat) (end : Nat))
parse_union_def                      ; (union Expr (IntLit (v : Int)) (NameRef (n : Symbol)))
parse_match_variant_arm              ; (match e ((IntLit v s) ...))
parse_match_wildcard_arm             ; (match e (_ default))
parse_match_binding_arm              ; (match e (x x))
parse_type_annotation_parametric     ; (Index source-len) round-trips
parse_type_annotation_generic        ; (List Expr) round-trips
unannotated_code_unchanged           ; regression: all existing tests pass
```

### `twig-ir-compiler`

```
record_def_emits_constructor         ; Span(a,b,c) builds cons list
record_def_emits_accessors           ; span-start(v) returns (car (cdr v))
union_def_emits_constructors         ; IntLit(v,s) tag=0
union_def_emits_predicates           ; IntLit?(v) = (= (car v) 0)
match_variant_arm_lowers             ; (match e ((IntLit v s) v)) extracts field
match_wildcard_lowers                ; (match e (_ 42)) → 42
match_binding_lowers                 ; (match e (x x)) → scrutinee bound to x
type_alias_erased                    ; (type Nat (Int 0 _)) emits no IIR
```

### `twig-vm` (end-to-end)

```
e2e_record_construct_and_access      ; construct Span, access fields
e2e_union_match_two_variants         ; match on IntLit vs NameRef
e2e_match_wildcard_fallthrough       ; wildcard arm fires correctly
```

---

## What this PR does NOT ship

- Type checking: no `twig-type-checker` crate (TW05-B).
- Refinement checker wiring for new annotation forms (TW05-C).
- Actual compilation of self-hosted compiler modules (TW05-D onwards).
- `(typed strict)` enforcement — the mode is recorded but not yet enforced.
- `unsafe-assume` form (TW05-H).
