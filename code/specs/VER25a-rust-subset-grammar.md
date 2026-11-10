# VER25a — Confined Rust-Subset Grammar & Parser (VER-P022)

**Parent:** [VER25](VER25-wir-core-interpreter.md), [VER26 §4–§5](VER26-ct-compare-pilot.md), [VER00 §7.5.5, §8.6.6](VER00-verified-shipping-program.md).

---

## 1. Purpose & Scope

This specification establishes the syntax, grammar, and AST for the **Confined Rust Subset** used to ingest security-critical cryptographic and systems primitives into the Warrant Intermediate Representation (WIR). It delivers **VER-P022**.

The primary motivation is to parse real-world crates such as `code/packages/rust/ct-compare` (and sibling primitives like `zeroize`, `poly1305`, and `chacha20`) without relying on bulky external compiler dependencies (`syn`, `proc-macro2`, `quote`) that violate the zero-third-party-dependency rule and introduce audit liabilities.

### 1.1 Key Principles

1. **Zero External Dependencies:** Built entirely with Rust standard library primitives (`core` and `alloc`/`std`).
2. **Deterministic & Bounded:** Hard resource quotas on source size (≤ 1 MiB), recursion/nesting depth (≤ 32), and total AST node counts (≤ 65,536).
3. **Explicit Volatile & Unsafe Tracking:** Unsafe blocks and volatile intrinsic accesses (`read_volatile`, `write_volatile`, `black_box`) are first-class nodes in the AST to expose the exact compiler-observability and safety assumptions (VER26 §3).
4. **Faithful Recovery of Real Crates:** Directly parses `ct-compare/src/lib.rs` and its internal `mod volatile_barrier`, const-generic functions (`ct_eq_fixed<const N: usize>`), bitwise folding expressions, and range-based loops.
5. **Fail-Closed Diagnostics:** Every syntax rejection produces a structured error indicating exact 1-based line and column, byte offset, and expected token/construct.

---

## 2. Subset Grammar Specification

The grammar covers the necessary Rust constructs for algorithmic, cryptographic, and memory-barrier code:

### 2.1 Lexical Elements
- **Whitespace & Comments:** `// ...`, `/* ... */`, outer doc comments `//! ...`, inner doc comments `/// ...`.
- **Keywords:** `fn`, `pub`, `mod`, `use`, `let`, `mut`, `if`, `else`, `for`, `in`, `return`, `unsafe`, `as`, `const`, `true`, `false`, `super`.
- **Identifiers:** Standard Rust identifiers `[a-zA-Z_][a-zA-Z0-9_]*`.
- **Literals:**
  - Decimals (`0`, `42`, `1000`).
  - Hexadecimals (`0x00`, `0xFF`, `0x42`).
  - Binaries (`0b0000_0000`, `0b1000_0000`).
  - Type-suffixed integers (`0u8`, `1u64`, `63usize`).
  - Byte strings (`b"abcdef"`, `b""`).
  - Byte arrays (`[0u8; 32]`, `[0xFFu8; 64]`).
  - Booleans (`true`, `false`).
- **Operators:**
  - Arithmetic / Bitwise: `+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`, `<<`, `>>`.
  - Logical: `&&`, `||`, `!`.
  - Comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`.
  - Assignment / Compound: `=`, `+=`, `-=`, `|=`, `&=`, `^=`.
- **Delimiters:** `(`, `)`, `{`, `}`, `[`, `]`, `::`, `,`, `;`, `:`, `->`, `..`.

### 2.2 Declarations & Items
- **Attributes:** Outer `#![...]` and inner `#[...]` attributes (e.g. `#[inline(always)]`, `#[allow(unsafe_code)]`, `#[cfg(test)]`).
- **Use Declarations:**
  - Simple paths: `use core::hint::black_box;`
  - Grouped paths: `use core::ptr::{read_volatile, write_volatile};`
  - Wildcard / Super: `use super::*;`
- **Modules:** `mod name { items... }`
- **Functions:**
  ```text
  [vis] fn name [< const N : usize >] ( [params] ) [ -> RetType ] { [stmts] }
  ```
  where `vis` can be `pub`, `pub(super)`, or private.

### 2.3 Types
- Primitive scalar types: `u8`, `u16`, `u32`, `u64`, `usize`, `i8`, `i16`, `i32`, `i64`, `isize`, `bool`.
- Reference types: `&T`, `&mut T`.
- Slice types: `&[T]`.
- Array types: `[T; N]`, `&[T; N]`.
- Dynamic vector: `Vec<T>`.
- Unit: `()`.

### 2.4 Statements & Expressions
- **Let Statement:** `let [mut] ident [: Type] = expr;`
- **Assignment:** `target = expr;`, `target op= expr;`
- **Return:** `return [expr];`
- **Control Flow:**
  - `if expr { block } [else { block | if ... }]`
  - `for ident in expr .. expr { block }`
  - `unsafe { block }`
- **Expressions:**
  - Binary operations (`^`, `|`, `&`, `>>`, `==`, `!=`, etc.)
  - Unary operations (`!`, `-`, `&`, `&mut`, `*`)
  - Method calls (`x.len()`, `x.wrapping_sub(y)`, `diff.wrapping_neg()`)
  - Function / Path calls (`read_volatile(x)`, `volatile_barrier::accumulate(...)`)
  - Array / Slice indexing (`a[i]`, `&a[i]`)
  - Type cast (`expr as Type`)
  - Macro invocations (`vec![...]`, `assert_eq!(...)`, `debug_assert_eq!(...)`, `assert!(...)`)

---

## 3. AST Structure

The AST is represented in pure, safe Rust:

```rust
pub struct SourceFile {
    pub inner_attrs: Vec<Attribute>,
    pub items: Vec<Item>,
}

pub enum Item {
    Use(UseDecl),
    Module(ModuleDecl),
    Function(FnDecl),
}

pub struct FnDecl {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub name: String,
    pub const_generics: Vec<ConstGenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<RustType>,
    pub body: Block,
}
```

---

## 4. Verification & Limits

1. **Quota Checks:** Maximum tokens ≤ 65,536; recursion depth ≤ 32; file size ≤ 1,048,576 bytes.
2. **Coverage Thresholds:** ≥ 90% Line Coverage AND ≥ 90% Branch Coverage across parser, lexer, and AST.
3. **Integration Verification:** Must parse the actual `code/packages/rust/ct-compare/src/lib.rs` without modifications.
