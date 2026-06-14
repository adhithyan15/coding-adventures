# mosmodel-compiler

Compiles `.mil` component interface files to an interface descriptor JSON
and target-language bindings.

## What is mosmodel?

`mosmodel` is the component interface language for the Mosaic UI stack.
A `.mil` file answers exactly one question:

> *What does the outside world need to know to use this component?*

It answers with two constructs:

- **slots** — named, typed data values the host pushes *in* to the component
- **emits** — named, typed events the component fires *out* to the host

Nothing else is permitted; the compiler rejects anything else.

### Example

```mosmodel
component Grid {
  // What to show
  slot column-headers  : list<text> ;
  slot total-rows      : number ;

  // What the host has scrolled to
  slot viewport-offset : number = 0 ;
  slot viewport-rows   : list<list<text>> ;

  // Selection state — host owns, pushes in
  slot selected-row    : number = 0 ;
  slot selected-col    : number = 0 ;

  // Navigation — fires when user presses arrow key
  emit onNavigate ( row : number , col : number ) ;

  // Edit lifecycle
  emit onEditStart  ( row : number , col : number ) ;
  emit onEditCommit ( value : text ) ;
  emit onEditCancel ;
}
```

## Where it fits

```
mosmodel (.mil)           ← this crate compiles these
     │  declares interface
     ▼
moslayout (.mll)         ← moslayout-compiler
     │  references slot/emit names, arranges primitives
     ▼
mosstyle (.msl)           ← mosstyle-compiler
     │  references part names, declares visual appearance
     ▼
backend emitter
     │  collapses all three into one output file
     ▼
Rust struct / Swift class / JSX component / Qt QObject / …
```

## Compile pipeline

```
.mil source
      │  tokenize()
      ▼
Vec<Token>        (mosmodel.tokens grammar via GrammarLexer)
      │  parse()
      ▼
GrammarASTNode    (mosmodel.grammar via GrammarParser)
      │  analyze()
      ▼
MosmodelComponent (typed IR: slots + emits)
      │  validate()
      ▼
ValidationResult  (uniqueness, type resolution, default compatibility)
      │  emit_json() / emit_rust_binding()
      ▼
String            (interface descriptor JSON or Rust struct source)
```

## Usage

```rust
use mosmodel_compiler::compile;

let src = r#"
  component Button {
    slot label    : text ;
    slot disabled : bool = false ;
    emit onClick ;
    emit onLongPress ;
  }
"#;

let result = compile(src).expect("compilation failed");

// Interface descriptor (consumed by moslayout + mosstyle compilers)
println!("{}", result.descriptor_json);

// Rust struct binding for the Metal / paint-vm backend
println!("{}", result.rust_binding);
```

### Generated Rust binding (Button)

```rust
#[derive(Default)]
pub struct Button {
    pub label:    String,
    pub disabled: bool,
    pub on_click:      Option<Box<dyn Fn()>>,
    pub on_long_press: Option<Box<dyn Fn()>>,
}

impl Button {
    pub fn new() -> Self { Self::default() }
    pub fn label(mut self, v: String) -> Self { ... }
    pub fn disabled(mut self, v: bool) -> Self { ... }
    pub fn on_click(mut self, f: impl Fn() + 'static) -> Self { ... }
    pub fn on_long_press(mut self, f: impl Fn() + 'static) -> Self { ... }
}
```

## Slot types

| Type | Description |
|---|---|
| `text` | UTF-8 string |
| `number` | 64-bit float |
| `bool` | boolean |
| `image` | opaque image reference (backend-specific) |
| `color` | RGBA color value |
| `node` | arbitrary composed component |
| `list<T>` | homogeneous ordered list |
| `ComponentName` | named component type |

## Emit payload types

Same as slot types, minus `image` and `node` (events carry data, not subtrees).

## Error handling

`compile()` returns `Err(Vec<CompileError>)` with one of seven structured error
kinds from the spec §6: `DuplicateName`, `NameConflict`, `UnknownType`,
`InvalidDefault`, `NoDefaultForType`, `UnknownConstruct`, `MissingComponent`.

## Design principles

1. **The interface is the only public API.** Layout and style are implementation
   details; `.mil` is the only stable, public contract.
2. **One direction per construct.** Slots carry data inward. Emits carry signals
   outward. No two-way binding.
3. **The compiler is the enforcer.** Grammar rules make invalid constructs
   impossible to express.
4. **Backend-agnostic by construction.** The same `.mil` file drives Rust,
   Swift, React, Qt, and Web Component code generation.

## Relationship to other specs

- **UI13-mosmodel.md** — the language specification this crate implements.
- **moslayout-compiler** — imports the interface descriptor JSON produced here.
- **mosstyle-compiler** — imports part names from moslayout (not directly from mosmodel).
