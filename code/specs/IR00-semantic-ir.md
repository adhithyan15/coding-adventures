# IR00 — Semantic Intermediate Representation

## Overview

This document specifies the **Semantic IR** — the universal, language-agnostic
intermediate representation that sits at the heart of every compiler, transpiler,
bundler, optimizer, and code transformation tool in this repository.

The Semantic IR is not a JavaScript AST. It is not tied to any source language or
output language. A TypeScript file, an XML enum definition, a Protocol Buffer schema,
and a Python module can all be lowered into the same Semantic IR and then raised back
out as JavaScript, TypeScript, or any other language the codegen understands.

Think of it the way LLVM IR relates to C, Rust, and Swift: each of those languages
has a frontend that lowers to LLVM IR; each CPU architecture has a backend that raises
from LLVM IR to native machine code. Neither the frontends nor the backends know about
each other. That is the design goal here — but operating at the high-level language
layer rather than the machine-code layer.

```
TypeScript frontend  ──┐
XML enum frontend    ──┤                    ┌── JavaScript codegen
Proto schema frontend──┼── Semantic IR ─────┼── TypeScript codegen
JSON Schema frontend ──┤                    ├── Type declaration codegen
Future frontend      ──┘                    └── Future codegen
```

Every node carries a **`cv_id`**: a stable identity string assigned at birth that
follows it through every transformation, merge, split, and deletion in the pipeline.
The full Correlation Vector system — ID format, CVLog, contribution model, lifecycle
rules, querying API, and polyglot implementations — is specified in
**CV00-correlation-vector.md**. This spec only describes how the IR uses it.

---

## Design Principles

### 1. Semantic, not syntactic

Syntactic representations mirror the grammar — they have separate node types for
`ArrowFunctionExpression`, `FunctionDeclaration`, and `FunctionExpression` because the
grammar has three different rules. A semantic representation collapses these into one
`FunctionDef` node with attributes that capture the distinctions that matter
(`arrow: true`, `async: true`, `generator: false`).

This matters for two reasons:

- **Passes are simpler.** A constant-folding pass does not need to handle three
  function node types. It handles one.
- **Frontends are decoupled from backends.** A codegen that emits arrow functions
  does not need to know whether the source was TypeScript or a hand-crafted IR node.

The rule for when to normalize vs preserve a distinction: **normalize when two
syntactic forms have identical semantics in all contexts; preserve when they differ
in at least one observable way.** Arrow functions and regular functions have different
`this` binding — that is a semantic difference, so we preserve it as an attribute.
`let` and `const` have different mutability semantics — preserved. `var` and `let`
have different scoping rules — preserved. All three collapse into `Binding` with a
`kind` attribute rather than three separate node types.

### 2. Language-agnostic core, extensible for any language

Every node kind in the universal catalog (Section 5) has been deliberately stripped
of language-specific assumptions. A `FunctionDef` is a function definition in any
language that has functions. A `Binding` is a named value binding in any language
that has variables.

Language-specific concepts live in the extended catalog (Section 6 covers JS/TS).
Future sections will cover Python, Ruby, Lua, etc. The pipeline does not need to know
which extended catalog a node comes from. Passes that do not recognize a node kind
must pass it through unmodified.

### 3. Polyglot implementation

The Semantic IR is a data model, not a library. This spec describes it in terms of
abstract data types. Every language in the repository (Elixir, Rust, TypeScript, Go,
Python, Ruby) can and should implement this spec identically. The JSON serialization
format defined in Section 8 is the interchange layer between implementations.

A pipeline built in Rust can receive a node tree serialized by an Elixir parser,
run its passes, and hand off the result to a TypeScript codegen. The CV system
survives serialization — CVs are stable strings that mean the same thing across
all implementations.

### 4. Correlation Vectors are first-class

Every node has a `cv_id` assigned at birth (see CV00 for ID format and generation
rules). Every pass that touches a node calls `CVLog.contribute` or
`CVLog.passthrough` so the full transformation history is recorded. The CVLog travels
with the node tree in the Context through every pass.

When tracing is disabled (`CVLog.enabled = false`), all CVLog writes are no-ops —
the `cv_id` fields still exist on nodes but no history is recorded. Zero branching
overhead in production paths.

---

## Part 1: The Node

The Node is the atomic unit of the Semantic IR. Every piece of a program —
statements, expressions, types, declarations, module structure — is a Node.

### 1.1 Structure

```
Node {
  kind:     NodeKind       -- What semantic concept this node represents
  attrs:    Attrs          -- Kind-specific attributes (key → AttrValue)
  children: Node[]         -- Ordered child nodes
  cv_id:    CvId           -- Correlation vector identity (stable, never changes)
}
```

That is the complete node. Four fields.

Notice what is **not** in the node:

- No source position. Source position is in the CVLog entry for this `cv_id`.
  Separating identity from location means synthesized nodes (created by passes with
  no source location) can still participate in the CV system.
- No pass metadata. Analysis results (inferred types, scope resolution, etc.) live
  in the Context alongside the node tree (Section 4), keyed by `cv_id`. This keeps
  nodes immutable and avoids node inflation as more passes run.
- No parent pointer. The tree is walked top-down. Passes that need parent context
  carry it in a traversal stack.

### 1.2 Field semantics

**`kind`** is an atom (or enum value in statically-typed languages). It names the
semantic concept the node represents. The kind determines what attributes are legal
and what children are expected. See Sections 5 and 6 for the kind catalogs.

**`attrs`** is a map from string keys to `AttrValue`. The legal keys and value types
for each `kind` are defined in the kind catalog. Unknown attribute keys on a known
kind are ignored by passes that do not use them — this allows forward compatibility
as new attributes are added to existing kinds.

**`children`** is an ordered list of child nodes. The positions of children (which
child is the condition, which is the body, etc.) are defined by the kind catalog.
Where order is semantically meaningful (the statements of a block), it is preserved.

**`cv_id`** is a stable, globally unique string identifier assigned when the node
is first created. Once assigned, it never changes for the lifetime of the pipeline
run. Format, generation, and querying are defined in CV00-correlation-vector.md.
See Part 2 for IR-specific CV conventions.

### 1.3 AttrValue types

Attributes carry structured data associated with a node. The following value types
are supported across all implementations:

| Type      | Description                          | Examples                      |
|-----------|--------------------------------------|-------------------------------|
| `string`  | UTF-8 string                         | `"foo"`, `"let"`, `"strict"`  |
| `number`  | IEEE 754 double or arbitrary integer | `42`, `3.14`, `-1`            |
| `boolean` | True or false                        | `true`, `false`               |
| `atom`    | Enumerated symbolic value            | `:let`, `:const`, `:async`    |
| `null`    | Explicit absence of a value          | `null`                        |
| `list`    | Ordered list of AttrValues           | `[":public", ":readonly"]`    |
| `map`     | String-keyed map of AttrValues       | `{min: 0, max: 255}`          |

In dynamically typed languages (Elixir, Python, Ruby), `AttrValue` is the natural
union type. In statically typed languages (Rust, TypeScript, Go), use a tagged union
or enum with variants for each type above.

---

## Part 2: Correlation Vectors in the IR

> The full CV specification — ID format, dot-extension scheme, CVLog structure,
> lifecycle rules (`create`, `contribute`, `derive`, `merge`, `delete`, `passthrough`),
> querying API, serialization, and polyglot implementations — lives in
> **CV00-correlation-vector.md**. Read that spec first. This section only describes
> the IR-specific conventions for how passes use the CV library.

### 2.1 How the parser assigns CVs

When the parser creates a node from a source token or grammar rule, it calls
`CVLog.create(log, origin: %{source: file, location: "line:col"})` and stores the
returned `cv_id` in the node. This is the node's permanent identity.

### 2.2 How passes contribute

The `source` field in a CV contribution is the pass name (e.g., `"variable_renamer"`).
The `tag` field is the compiler action. Recommended tag vocabulary for IR passes:

| Tag               | Meaning                                                    |
|-------------------|------------------------------------------------------------|
| `"created"`       | Pass created this node (parser, synthesizing pass)         |
| `"passed_through"`| Pass examined the node, made no changes                    |
| `"transformed"`   | Pass modified attrs or reordered children                  |
| `"renamed"`       | Pass changed a name (variable renamer, mangler)            |
| `"folded"`        | Pass replaced with a simpler equivalent                    |
| `"inlined"`       | Pass inlined content from another node                     |
| `"type_resolved"` | Type inference attached type information                   |
| `"scope_resolved"`| Scope analysis resolved this identifier to a binding      |
| `"deleted"`       | DCE or other pass removed this node                        |

These are conventions, not constraints. Passes may use any tag that is meaningful
to their domain.

### 2.3 Node lifecycle rules for IR passes

**Keep the same `cv_id`** when a pass transforms a node in place — renaming a
variable, folding a constant, attaching type info. Same logical concept, modified form.

**Call `CVLog.derive`** when splitting one node into multiple output nodes —
destructuring `const {a, b} = x` into two `Binding` nodes, expanding a decorator.

**Call `CVLog.merge`** when combining multiple nodes into one — inlining a function
call merges the call site CV and the function body CV into the inlined expression.

**Call `CVLog.delete`** when removing a node — dead code elimination, unreachable
branch removal. The CV entry persists in the log so the deletion is always auditable.

**Call `CVLog.create` with no origin** for synthesized nodes that have no source
correspondence — codegen wrapper IIFEs, bundler runtime boilerplate.

---

## Part 3: The Pass

A pass is any transformation or analysis that runs over the node tree. Passes are
the units of the pipeline. Composing a sequence of passes produces a tool.

### 3.1 The pass interface

Every pass implements one function:

```
run(node: Node, context: Context, opts: Opts) → (Node, Context)
```

- **Input**: The root node of the current tree, the current context, and pass-specific
  options.
- **Output**: A (possibly transformed) root node and an updated context.
- **Pure**: Given the same inputs, a pass produces the same outputs. Passes do not
  have side effects. They do not mutate their inputs.
- **Total**: A pass must return a result for any valid input. It signals errors by
  returning an error in the context (Section 4.3), not by throwing exceptions.

The pass function may recursively call itself on child nodes, or delegate to a
traversal helper (Section 3.2).

### 3.2 Traversal

Most passes follow a standard tree-walking pattern. Implementations should provide
a traversal helper that applies a visitor function at each node:

```
traverse(node, context, visitor) → (Node, Context)
```

The visitor is called at each node with `(node, context)` and returns
`(node, context)`. The traversal helper wires up the recursion. The visitor function
decides whether to recurse into children (pre-order), recurse first (post-order),
or both.

**Transparency rule**: A visitor that does not recognize a node's `kind` must return
the node unchanged (after recording a `:passed_through` contribution if tracing is
enabled). This is how foreign-frontend nodes (XML enums, proto messages, etc.) flow
through JS-specific passes without errors.

### 3.3 Pass naming

Passes are identified by a `PassName` atom or string. Standard names follow the
convention `language:concept` or just `concept` for language-agnostic passes:

```
:parser
:scope_analysis
:type_inference
:constant_folder
:dead_code_eliminator
:variable_renamer
:property_renamer
:function_inliner
:module_resolver
:bundler
:codegen
:source_map
```

Pass names appear in CV contributions and in pipeline configuration. They must be
unique within a pipeline run.

---

## Part 4: The Context

The Context travels alongside the node tree through the entire pipeline. It carries
the CVLog, analysis results keyed by `cv_id`, errors, and pipeline configuration.

### 4.1 Structure

```
Context {
  cv_log:       CVLog
  scope_map:    Map<CvId, ScopeInfo>      -- populated by scope_analysis pass
  type_map:     Map<CvId, TypeInfo>       -- populated by type_inference pass
  binding_map:  Map<CvId, BindingInfo>    -- populated by scope_analysis pass
  call_graph:   CallGraph?                -- populated by call_graph pass
  errors:       Error[]
  warnings:     Warning[]
  opts:         Opts                      -- pipeline-level configuration
}
```

Passes add to the context maps but do not remove from them. A pass that computes
scope information for a node adds to `scope_map`; later passes read from it. This
means the context is an accumulating record of everything the pipeline has learned
about the program.

### 4.2 Analysis result types

**ScopeInfo**: Attached to `Identifier` nodes by the scope analysis pass.
```
ScopeInfo {
  binding_cv_id: CvId        -- CV of the binding node this identifier resolves to
  scope_kind:    atom         -- :global, :module, :function, :block, :class
  is_free:       boolean      -- true if identifier refers to an outer scope
}
```

**TypeInfo**: Attached to any expression node by the type inference pass.
```
TypeInfo {
  inferred:  TypeExpr    -- best-effort inferred type
  declared:  TypeExpr?   -- explicitly annotated type (from source or JSDoc)
  certain:   boolean     -- false if inference was a best guess
}
```

**BindingInfo**: Attached to `Binding` nodes.
```
BindingInfo {
  kind:       atom        -- :let, :const, :var, :param, :import, :class, :function
  references: CvId[]      -- CV ids of all Identifier nodes that resolve here
  is_dead:    boolean     -- true if no references (set by DCE pass)
}
```

### 4.3 Errors and warnings

Passes signal problems by appending to `context.errors` or `context.warnings`.
The pipeline runner (Section 5) decides whether to halt on errors or continue
(configurable). Errors carry a `cv_id` pointing to the offending node so the
source location can be looked up in the CVLog.

```
Error {
  code:    string      -- e.g., "TYPE_MISMATCH", "UNDEFINED_VARIABLE"
  message: string
  cv_id:   CvId?
  pass:    PassName
}
```

---

## Part 5: The Pipeline Runner

The pipeline runner composes a sequence of passes into a tool.

### 5.1 Running a pipeline

```
run_pipeline(source, passes, opts) → PipelineResult
```

1. Invoke the parser pass to produce `(root_node, initial_context)`.
2. Thread `(root_node, context)` through each pass in sequence.
3. Return `PipelineResult` containing the final node tree, final context, and CVLog.

```
PipelineResult {
  node:     Node        -- root of the transformed tree
  context:  Context     -- accumulated analysis and errors
  cv_log:   CVLog       -- full provenance record (empty if tracing disabled)
}
```

### 5.2 Recipes

A recipe is a named, reusable pass list. Implementations should provide a recipe
registry so tools can be assembled by name:

```
Recipes {
  :babel_transform  → [parser, scope_analysis, ts_type_stripper,
                        arrow_fn_transform, class_transform, async_transform, codegen]

  :terser           → [parser, scope_analysis, constant_folder, peephole,
                        variable_renamer, codegen]

  :closure_simple   → [parser, scope_analysis, constant_folder, peephole,
                        variable_renamer, codegen, source_map]

  :closure_advanced → [parser, scope_analysis, type_inference, call_graph,
                        data_flow, dead_code_eliminator, function_inliner,
                        constant_folder, peephole, property_renamer,
                        variable_renamer, codegen, source_map]

  :webpack          → [parser, scope_analysis, module_resolver, dependency_graph,
                        tree_shaker, bundler, codegen, source_map]

  :whitespace_only  → [parser, codegen]
}
```

Recipes are starting points, not constraints. Users compose custom pass lists for
any tool by mixing and matching from the catalog.

### 5.3 Tracing wrapper

Any pass can be wrapped in a tracing decorator that captures a before/after snapshot
of every node it touches:

```
traced(pass) → pass
```

The traced wrapper calls the inner pass normally. After each node transformation, it
compares the before and after states and appends a detailed contribution to the CVLog.
This is more expensive than the normal CVLog appends (which are just pushes), but
is useful for debugging a specific pass.

Typical use: wrap only the pass under investigation, not the entire pipeline.

---

## Part 6: Universal Node Kind Catalog

These kinds are language-agnostic. Any frontend can emit them; any pass can handle
them. The JS/TS kind catalog in Part 7 extends this set.

### Program container

| Kind      | Attrs                                 | Children           | Meaning                            |
|-----------|---------------------------------------|--------------------|-------------------------------------|
| `program` | `source_type: :script\|:module`       | statements[]       | Top-level container for a file      |
| `block`   | —                                     | statements[]       | Sequence of statements in a scope   |

### Declarations and bindings

| Kind            | Attrs                                                      | Children          | Meaning                           |
|-----------------|------------------------------------------------------------|-------------------|------------------------------------|
| `binding`       | `name: string`, `kind: :let\|:const\|:var\|:param\|:field` | [initializer?]    | Named value binding                |
| `function_def`  | `name: string?`, `async: bool`, `generator: bool`, `arrow: bool` | [params, body] | Function or lambda definition   |
| `class_def`     | `name: string?`                                           | [superclass?, members[]] | Class definition              |
| `class_member`  | `name: string`, `static: bool`, `kind: :method\|:field\|:getter\|:setter` | [value] | Class member |
| `enum_def`      | `name: string`                                            | members[]         | Enumeration type definition        |
| `enum_member`   | `name: string`                                            | [value?]          | Member of an enumeration           |
| `param`         | `name: string`, `rest: bool`, `optional: bool`            | [default?, type_annotation?] | Function parameter         |

### Statements

| Kind         | Attrs                          | Children                          | Meaning                     |
|--------------|--------------------------------|-----------------------------------|-----------------------------|
| `return`     | —                              | [value?]                          | Return from function        |
| `throw`      | —                              | [value]                           | Throw exception             |
| `if`         | —                              | [test, consequent, alternate?]    | Conditional branch          |
| `while`      | `kind: :while\|:do_while`      | [test, body]                      | While loop                  |
| `for`        | —                              | [init?, test?, update?, body]     | C-style for loop            |
| `for_each`   | `kind: :of\|:in`               | [binding, iterable, body]         | Iteration loop              |
| `switch`     | —                              | [discriminant, cases[]]           | Switch statement            |
| `case`       | `default: bool`                | [test?, consequents[]]            | Switch case clause          |
| `try_catch`  | —                              | [body, catch_clause?, finally?]   | Exception handling          |
| `catch`      | —                              | [binding?, body]                  | Catch clause                |
| `break`      | `label: string?`               | —                                 | Break out of loop/switch    |
| `continue`   | `label: string?`               | —                                 | Continue to next iteration  |
| `label`      | `name: string`                 | [statement]                       | Labeled statement           |
| `expression_statement` | —                  | [expression]                      | Expression used as statement|

### Expressions

| Kind           | Attrs                                               | Children              | Meaning                         |
|----------------|-----------------------------------------------------|-----------------------|---------------------------------|
| `assignment`   | `op: :assign\|:plus_assign\|:minus_assign\|...`     | [target, value]       | Assignment expression           |
| `binary_op`    | `op: atom`                                          | [left, right]         | Binary operation (see ops table)|
| `unary_op`     | `op: atom`, `prefix: bool`                          | [operand]             | Unary operation                 |
| `call`         | `optional_chain: bool`                              | [callee, args[]]      | Function/method call            |
| `new`          | —                                                   | [constructor, args[]] | Constructor call                |
| `member_access`| `property: string`, `computed: bool`, `optional: bool` | [object]           | Property access (`obj.prop`)    |
| `index_access` | `optional: bool`                                    | [object, index]       | Index access (`arr[i]`)         |
| `conditional`  | —                                                   | [test, consequent, alternate] | Ternary operator       |
| `sequence`     | —                                                   | expressions[]         | Comma-separated expressions     |
| `spread`       | `rest: bool`                                        | [value]               | Spread or rest element          |

### Literals

| Kind             | Attrs                          | Children | Meaning                         |
|------------------|--------------------------------|----------|---------------------------------|
| `identifier`     | `name: string`                 | —        | Variable or name reference      |
| `literal`        | `kind: :string\|:number\|:boolean\|:null\|:undefined\|:regex\|:bigint`, `value: AttrValue`, `raw: string` | — | Literal value |
| `array_literal`  | —                              | elements[] | Array literal `[a, b, c]`   |
| `record_literal` | —                              | properties[] | Object literal `{a: 1}`   |
| `property`       | `key: string`, `computed: bool`, `shorthand: bool`, `kind: :init\|:get\|:set` | [key_node?, value] | Object property |

### Modules

| Kind       | Attrs                                      | Children              | Meaning                    |
|------------|--------------------------------------------|-----------------------|----------------------------|
| `import`   | `source: string`, `kind: :static\|:dynamic` | specifiers[]         | Import declaration         |
| `export`   | `default: bool`, `source: string?`         | [declaration\|specifiers[]] | Export declaration    |
| `specifier`| `local: string`, `imported: string?`, `exported: string?` | — | Import/export name mapping |

### Types (language-agnostic subset)

| Kind              | Attrs                        | Children                | Meaning                          |
|-------------------|------------------------------|-------------------------|----------------------------------|
| `type_annotation` | —                            | [type_expr]             | Type annotation on a node        |
| `named_type`      | `name: string`               | type_args[]             | Named type reference `Foo<T>`    |
| `union_type`      | —                            | members[]               | Union type `A \| B`              |
| `intersection_type` | —                          | members[]               | Intersection type `A & B`        |
| `array_type`      | —                            | [element_type]          | Array type `T[]`                 |
| `tuple_type`      | —                            | element_types[]         | Tuple type `[A, B]`              |
| `literal_type`    | `value: AttrValue`           | —                       | Literal type `"foo"` or `42`     |
| `function_type`   | `async: bool`                | [params[], return_type] | Function type `(a: T) => U`      |
| `record_type`     | —                            | members[]               | Object type `{ a: T; b: U }`     |

### Binary operators (`:op` attribute values)

```
Arithmetic:  :add, :sub, :mul, :div, :mod, :pow
Bitwise:     :bit_and, :bit_or, :bit_xor, :bit_not, :shl, :shr, :ushr
Comparison:  :eq, :neq, :strict_eq, :strict_neq, :lt, :lte, :gt, :gte
Logical:     :and, :or, :nullish
String:      (string concatenation uses :add)
Membership:  :in, :instanceof
```

---

## Part 7: JS/TS Node Kind Catalog

These kinds are specific to JavaScript and TypeScript. They flow through the pipeline
transparently in passes that do not know about them.

### JavaScript-specific expressions

| Kind                | Attrs                             | Children               | Meaning                              |
|---------------------|-----------------------------------|------------------------|--------------------------------------|
| `this`              | —                                 | —                      | `this` keyword                       |
| `super`             | —                                 | —                      | `super` reference                    |
| `await`             | —                                 | [value]                | `await` expression                   |
| `yield`             | `delegate: bool`                  | [value?]               | `yield` expression                   |
| `typeof`            | —                                 | [operand]              | `typeof` operator                    |
| `void`              | —                                 | [operand]              | `void` operator                      |
| `delete`            | —                                 | [operand]              | `delete` operator                    |
| `template_literal`  | —                                 | [quasis[], expressions[]] | Template literal `` `foo ${bar}` ``|
| `template_element`  | `raw: string`, `cooked: string`, `tail: bool` | —    | Literal chunk of a template literal  |
| `tagged_template`   | —                                 | [tag, quasi]           | Tagged template `` tag`...` ``       |
| `sequence`          | —                                 | expressions[]          | Comma operator                       |

### JavaScript-specific statements

| Kind           | Attrs                         | Children         | Meaning                             |
|----------------|-------------------------------|------------------|-------------------------------------|
| `debugger`     | —                             | —                | `debugger` statement                |
| `with`         | —                             | [object, body]   | `with` statement (legacy, avoid)    |

### Destructuring

| Kind                  | Attrs              | Children              | Meaning                              |
|-----------------------|--------------------|-----------------------|--------------------------------------|
| `array_pattern`       | —                  | elements[]            | Array destructuring pattern `[a, b]` |
| `object_pattern`      | —                  | properties[]          | Object destructuring `{a, b}`        |
| `assignment_pattern`  | —                  | [left, right]         | Default value in destructuring       |
| `rest_element`        | —                  | [argument]            | Rest in destructuring `...rest`      |

### TypeScript-specific

| Kind                  | Attrs                                        | Children                  | Meaning                                |
|-----------------------|----------------------------------------------|---------------------------|----------------------------------------|
| `ts_type_alias`       | `name: string`                               | [type_params[], type_expr] | `type Foo<T> = ...`                   |
| `ts_interface`        | `name: string`                               | [type_params[], extends[], members[]] | `interface Foo { ... }`      |
| `ts_enum`             | `name: string`, `const: bool`                | members[]                 | `enum Color { Red = 0 }`               |
| `ts_namespace`        | `name: string`                               | [body]                    | `namespace Foo { ... }`                |
| `ts_type_assertion`   | `style: :as\|:angle`                         | [expression, type_expr]   | `x as Foo` or `<Foo>x`                |
| `ts_non_null`         | —                                            | [expression]              | Non-null assertion `x!`                |
| `ts_satisfies`        | —                                            | [expression, type_expr]   | `x satisfies Foo`                      |
| `ts_declare`          | —                                            | [declaration]             | `declare` modifier                     |
| `ts_abstract`         | —                                            | [declaration]             | `abstract` modifier                    |
| `ts_access_modifier`  | `modifier: :public\|:private\|:protected\|:readonly` | [member]        | Access modifier on class member        |
| `ts_override`         | —                                            | [member]                  | `override` modifier                    |
| `ts_conditional_type` | —                                            | [check, extends, true_type, false_type] | `T extends U ? X : Y`   |
| `ts_mapped_type`      | `readonly: atom?`, `optional: atom?`         | [type_param, key_type, value_type] | `{ [K in keyof T]: V }`       |
| `ts_infer`            | `name: string`                               | —                         | `infer T` in conditional types         |
| `ts_keyof`            | —                                            | [type_expr]               | `keyof T`                              |
| `ts_typeof_type`      | —                                            | [expression]              | `typeof x` in type position            |
| `ts_indexed_access`   | —                                            | [object_type, index_type] | `T[K]`                                 |
| `ts_template_literal_type` | —                                      | [quasis[], types[]]       | `` `${string}` `` in type position     |
| `ts_type_predicate`   | `asserts: bool`                              | [param_name, type_expr?]  | `x is Foo` in return type              |
| `ts_decorator`        | —                                            | [expression]              | `@Decorator` on class/method/param     |
| `ts_type_param`       | `name: string`                               | [constraint?, default?]   | Generic type parameter `<T extends U>` |
| `ts_import_type`      | `qualifier: string?`                         | [argument, type_args?]    | `import("./foo").Bar`                  |

### Extension node

Any frontend or pass may introduce custom node kinds not in this catalog. By
convention, custom kinds are namespaced with the frontend or domain name:

```
xml:processing_instruction
proto:message_def
json_schema:object_schema
graphql:type_def
```

The transparency rule guarantees these nodes flow through unknown passes unchanged.
Any pass can be extended to handle custom kinds by adding a case to its visitor.

---

## Part 8: Serialization (Polyglot Interchange)

When a pipeline spans multiple language implementations (e.g., an Elixir parser
feeding a Rust optimizer feeding a TypeScript codegen), the node tree and CVLog
must be serialized between processes. The canonical format is JSON.

### 8.1 Node JSON encoding

```json
{
  "kind": "function_def",
  "attrs": {
    "name": "greet",
    "async": false,
    "generator": false,
    "arrow": false
  },
  "children": [
    { "kind": "param", "attrs": {"name": "name"}, "children": [], "cv_id": "3a7f1b2c.4" }
  ],
  "cv_id": "3a7f1b2c.3"
}
```

### 8.2 CVLog JSON encoding

```json
{
  "entries": {
    "3a7f1b2c.3": {
      "id": "3a7f1b2c.3",
      "parent_ids": [],
      "origin": { "file": "app.ts", "line": 5, "col": 0, "end_line": 8, "end_col": 1 },
      "contributions": [
        { "pass": "parser", "action": "created", "meta": {} },
        { "pass": "scope_analysis", "action": "passed_through", "meta": {} }
      ]
    }
  },
  "deletions": [],
  "pass_order": ["parser", "scope_analysis"]
}
```

### 8.3 Streaming

For large programs, implementations may stream the node tree as a sequence of
newline-delimited JSON objects (NDJSON) rather than one giant array. The CVLog is
always emitted last, after all node objects. Receivers buffer the CVLog until the
full stream is consumed.

---

## Part 9: Polyglot Implementation Notes

The following guidance helps implementors in each language in the repository produce
compatible IR implementations.

### Elixir

```elixir
defmodule IR.Node do
  defstruct [:kind, :attrs, :children, :cv_id]
  @type t :: %__MODULE__{
    kind:     atom(),
    attrs:    %{String.t() => term()},
    children: [t()],
    cv_id:    String.t()
  }
end

## IR.Context and IR.CVLog
# CVEntry and CVLog are defined in the CorrelationVector package (CV00).
# IR only defines the Context that carries them alongside the node tree.

defmodule IR.Context do
  defstruct [:cv_log, :scope_map, :type_map, :binding_map,
             :call_graph, :errors, :warnings, :opts]
end
```

### Rust

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub kind: String,
    pub attrs: HashMap<String, AttrValue>,
    pub children: Vec<Node>,
    pub cv_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttrValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
    List(Vec<AttrValue>),
    Map(HashMap<String, AttrValue>),
}

pub trait Pass {
    fn name(&self) -> &str;
    fn run(&self, node: Node, ctx: Context, opts: &Opts) -> (Node, Context);
}
```

### TypeScript

```typescript
interface Node {
  kind: string;
  attrs: Record<string, AttrValue>;
  children: Node[];
  cvId: string;
}

type AttrValue = string | number | boolean | null | AttrValue[] | Record<string, AttrValue>;

interface Pass {
  name: string;
  run(node: Node, ctx: Context, opts: Opts): [Node, Context];
}
```

### Go

```go
type Node struct {
    Kind     string
    Attrs    map[string]any
    Children []Node
    CvID     string
}

type Pass interface {
    Name() string
    Run(node Node, ctx Context, opts Opts) (Node, Context)
}
```

### Python

```python
from dataclasses import dataclass, field
from typing import Any

@dataclass
class Node:
    kind: str
    attrs: dict[str, Any] = field(default_factory=dict)
    children: list["Node"] = field(default_factory=list)
    cv_id: str = ""
```

### Ruby

```ruby
Node = Data.define(:kind, :attrs, :children, :cv_id)
```

---

## Part 10: Relationship to Other Specs and Packages

- **CV00-correlation-vector.md** — Defines the CVLog, CV ID format, dot-extension
  scheme, all six CV operations, querying API, and polyglot implementations. This
  spec is a consumer of that library. Every language implementation of the IR depends
  on the corresponding language implementation of the CV package.

- **`code/grammars/`** — The `.tokens` and `.grammar` files are the frontends. The
  grammar-driven parser produces a raw parse tree that is then lowered into the
  Semantic IR by a normalization pass. This decoupling means adding a new language
  version (e.g., ES2026) requires only new grammar files — the Semantic IR and all
  downstream passes are untouched.

- **`ecmascript_es*_lexer/parser`**, **`typescript_lexer/parser`** — These packages
  produce parse trees from the grammar system. A new `js_ir_normalizer` package (to
  be specified in IR01) will lower these parse trees into Semantic IR nodes.

- **`code/specs/04-bytecode-compiler.md`** — The bytecode compiler operates on a
  different IR (bytecode for the VM). The Semantic IR is a high-level, source-to-source
  IR. The two do not overlap but share the design philosophy of language-agnostic
  passes.

---

## Appendix A: Quick Reference Card

```
Every node:  {kind, attrs, children, cv_id}
cv_id:       stable forever, assigned at parse time, carried through all transforms

Transform 1:1  →  keep cv_id, append contribution to CVLog
Transform 1:N  →  new derived cv_ids with parent_id pointing back
Transform N:1  →  new merged cv_id with parent_ids listing all sources
Delete node    →  record in DeletionLog, append :deleted contribution
Create new     →  synthetic cv_id (00000000.N)

Pass contract: run(node, context, opts) → (node, context)   [pure, total]
Transparency:  unknown kind → pass through unchanged

Pipeline:      reduce over pass list, threading (node, context)
Recipes:       named pass lists for common tools (babel, terser, closure, webpack)
```
