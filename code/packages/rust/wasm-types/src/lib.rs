//! # wasm-types
//!
//! Pure type definitions for the WebAssembly 1.0 (MVP) type system.
//!
//! This crate contains no parsing logic — it only defines the data structures
//! that represent a decoded WASM module's type information. Higher-level crates
//! like `wasm-opcodes` and `wasm-module-parser` depend on these definitions.
//!
//! ## Where types live in the WASM binary format
//!
//! A `.wasm` file is a sequence of **sections**. Each section has an ID byte,
//! a byte-length, then contents.  The types in this crate mirror the decoded
//! contents of those sections:
//!
//! ```text
//! .wasm file layout
//! ┌──────────────────────────────────────────────────────────┐
//! │ Magic: 0x00 0x61 0x73 0x6D  ("asm")                     │
//! │ Version: 0x01 0x00 0x00 0x00                             │
//! ├──────┬────────────────────────────────────────────────── │
//! │ §  1 │ Type section   → Vec<FuncType>                    │
//! │ §  2 │ Import section → Vec<Import>                      │
//! │ §  3 │ Function section → Vec<u32> (type indices)        │
//! │ §  4 │ Table section  → Vec<TableType>                   │
//! │ §  5 │ Memory section → Vec<MemoryType>                  │
//! │ §  6 │ Global section → Vec<Global>                      │
//! │ §  7 │ Export section → Vec<Export>                      │
//! │ §  8 │ Start section  → Option<u32>                      │
//! │ §  9 │ Element section → Vec<Element>                    │
//! │ § 10 │ Code section   → Vec<FunctionBody>                │
//! │ § 11 │ Data section   → Vec<DataSegment>                 │
//! │ §  0 │ Custom sections (name = "name", debug info, etc.) │
//! └──────┴────────────────────────────────────────────────────
//! ```
//!
//! ## Numeric types and LEB128
//!
//! All integers in WASM binaries are encoded as
//! [LEB128](https://en.wikipedia.org/wiki/LEB128) variable-length integers.
//! The `wasm-leb128` crate handles that encoding; this crate uses plain Rust
//! integers in its structs because we represent the *decoded* form.
//!
//! ## This crate is part of coding-adventures
//!
//! A ground-up implementation of the computing stack from transistors to
//! operating systems, written in multiple languages for learning purposes.

// W34 first slice (`code/specs/W34-wasm-gc-canonical-type-equivalence.md`):
// `CanonicalGroup`'s own `Rc`-backed sharing (see that type's doc comment
// for why `Rc`, not an owned/boxed copy, at every embed site).
use std::collections::HashSet;
use std::rc::Rc;

// ──────────────────────────────────────────────────────────────────────────────
// ValueType
// ──────────────────────────────────────────────────────────────────────────────

/// The value types that WASM supports, extended for WasmGC (2023).
///
/// WASM 1.0 has four numeric types.  The WasmGC proposal (standardised 2023,
/// shipping in V8 ≥ 119 / Chrome ≥ 119 and Firefox ≥ 120) adds reference
/// types: `anyref`, `i31ref`, and concrete struct/array references.
///
/// ## Numeric types (WASM 1.0)
///
/// ```text
/// Byte encoding in WASM binary
/// ┌────────┬──────┬────────────────────────────────────────────┐
/// │  Type  │ Byte │ Description                                │
/// ├────────┼──────┼────────────────────────────────────────────┤
/// │  i32   │ 0x7F │ 32-bit integer (signed or unsigned)        │
/// │  i64   │ 0x7E │ 64-bit integer (signed or unsigned)        │
/// │  f32   │ 0x7D │ 32-bit IEEE 754 float                      │
/// │  f64   │ 0x7C │ 64-bit IEEE 754 float                      │
/// └────────┴──────┴────────────────────────────────────────────┘
/// ```
///
/// Note: WASM 1.0 has no boolean type. Boolean results (e.g., from `i32.eq`)
/// are represented as `i32`, where 0 means false and any non-zero means true.
///
/// Note: The byte values count *down* from 0x7F. This is because the WASM
/// binary format uses *signed* LEB128 for type bytes.  0x7F is -1 in signed
/// LEB128, 0x7E is -2, etc.  Newer WASM proposals continue the pattern.
///
/// ## WasmGC reference types
///
/// WasmGC extends the type system with *managed references* — values that
/// point into the GC heap, not into linear memory.  Think of them like
/// Java/JVM object references: the runtime manages their lifetime.
///
/// ```text
/// ┌────────────────────────┬──────────────────────┬──────────────────────────┐
/// │  Type                  │ Byte encoding        │ Description              │
/// ├────────────────────────┼──────────────────────┼──────────────────────────┤
/// │  anyref (null any)     │ 0x6E                 │ Nullable "top" ref type  │
/// │  i31ref (non-null i31) │ 0x6C                 │ Boxed 31-bit integer     │
/// │  (ref null $T)         │ 0x63 + LEB128(idx)   │ Nullable concrete struct │
/// └────────────────────────┴──────────────────────┴──────────────────────────┘
/// ```
///
/// `anyref` (= `(ref null any)`) is the supertype of all GC-managed values.
/// In WasmGC, a `LispyPair` struct field can be typed `anyref` to hold *any*
/// GC value — a cons cell, an integer box, or null.  This mirrors how
/// dynamic Lisp stores atoms and pairs in the same slot.
///
/// `i31ref` is a special type that boxes a 31-bit signed integer *without*
/// allocating heap memory.  The runtime stores the value in the reference
/// pointer bits (like V8's small-integer tagging trick).  Unboxing is done
/// with `i31.get_s`.
///
/// `StructRef(idx)` is a *nullable concrete reference* to a specific named
/// struct type at type-section index `idx`.  The binary encoding is a 2-byte
/// sequence `0x63 <LEB128(idx)>`.
///
/// ## Encoding note
///
/// Because `Anyref`, `I31ref`, and `StructRef` have variable-length binary
/// encodings (unlike the single-byte numeric types), `ValueType` can no
/// longer use `#[repr(u8)]`.  The encoder calls [`ValueType::encode`] to
/// emit the correct byte sequence.
// All variants are trivially Copy: the only data-bearing variant is
// StructRef(u32), and u32: Copy.  Adding Copy back lets the wasm-execution
// interpreter call default_for(*local_type) without a clone().
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType {
    /// 32-bit integer. Used for booleans, pointers (in linear memory), chars.
    I32,
    /// 64-bit integer. Used for 64-bit arithmetic and 64-bit pointers.
    I64,
    /// 32-bit IEEE 754 single-precision float.
    F32,
    /// 64-bit IEEE 754 double-precision float.
    F64,

    /// `(ref null any)` — nullable any-reference (WasmGC supertype).
    ///
    /// Encoded as a single byte `0x6E`.  Any GC-managed value (struct
    /// reference, i31ref, null) is assignment-compatible with `anyref`.
    /// This is the natural type for a dynamically-typed Lisp value slot.
    Anyref,

    /// `(ref i31)` — boxed 31-bit signed integer (WasmGC).
    ///
    /// Encoded as `0x6C`.  Unlike heap structs, i31ref values do not live
    /// on the GC heap; the runtime encodes the integer directly in the
    /// pointer bits.  This makes integer boxing/unboxing essentially free.
    ///
    /// ```text
    /// i31.new   : [i32] → [i31ref]   — box an i32 (top bit dropped)
    /// i31.get_s : [i31ref] → [i32]   — unbox with sign extension
    /// ```
    I31ref,

    /// `(ref null $T)` — nullable reference to a concrete struct type (WasmGC).
    ///
    /// The `u32` payload is the index into the type section that names struct
    /// type `$T`.  Encoded as `0x63` followed by LEB128(index).
    ///
    /// Example: if `$LispyPair` is defined at type-section index 1, then a
    /// local variable holding a nullable `$LispyPair` reference has type
    /// `StructRef(1)` and is encoded as `[0x63, 0x01]`.
    StructRef(u32),

    /// `(ref null $t)` where `$t` names a concrete **function** type
    /// (function-references proposal) -- the analogous nullable concrete
    /// reference to [`ValueType::StructRef`], but into the function-type
    /// index space instead of the struct-type one.
    ///
    /// Needed for exactly one narrow real-corpus construct (WASM11-B, see
    /// `code/specs/W11-wasm-tail-calls.md`'s addendum): a helper function
    /// declared `(func $f (result (ref null $t)) (ref.null $t))`, whose
    /// result is then used where `funcref` is expected via `return_call`/
    /// `return_call_indirect` (valid, since a nullable ref to a SPECIFIC
    /// function type is a subtype of the general `funcref`) or where the
    /// reverse is attempted (invalid -- `funcref` is NOT a subtype of a
    /// specific concrete function type). This crate does not otherwise
    /// track per-function-type identity anywhere else; this variant exists
    /// only to make that one subtyping direction checkable.
    ///
    /// The `u32` payload indexes [`WasmModule::types`] directly (0..N-1)
    /// -- the SAME index space a plain `(type $t)` reference elsewhere
    /// already resolves to, unlike `StructRef`'s `+ types.len()`-offset
    /// struct-type index. Encoded the same 2-byte shape as `StructRef`:
    /// `0x63` followed by `LEB128(idx)` -- see `StructRef`'s own doc
    /// comment for why the two never collide despite sharing a tag byte.
    ConcreteFuncRef(u32),

    /// `funcref` — nullable reference to a function (reference-types
    /// proposal; also WASM 1.0's implicit, hardcoded table element type).
    ///
    /// Encoded as a single byte `0x70`. Like `Anyref`, the wrapped handle is
    /// an opaque `u32` (a function index) carried at the value level by
    /// `wasm-execution::WasmValue::Ref` — only this static type distinguishes
    /// a `funcref` from any other reference kind (see `code/specs/
    /// W08-wasm-funcref-externref.md`).
    Funcref,

    /// `externref` — nullable, opaque reference to a host-supplied value
    /// (reference-types proposal).
    ///
    /// Encoded as a single byte `0x6F`. This repo has no host environment
    /// producing real external references; the only `externref` values
    /// exercised are the WASM testsuite's own `ref.extern N` script literals
    /// (see `code/specs/W08-wasm-funcref-externref.md`).
    Externref,

    /// `v128` — a 128-bit SIMD lane vector (SIMD proposal).
    ///
    /// Encoded as a single byte `0x7B`. Unlike the numeric types above, its
    /// 16 raw bytes don't fit in this engine's shared `virtual-machine::
    /// Value` typed-stack slot (max 64 bits) — at the value level, a `v128`
    /// is carried as a handle into a WASM-execution-local heap, the same
    /// shape `Anyref`/`I31ref` already use for `wasm-execution::WasmValue::
    /// Ref`'s GC-heap handles. See `code/specs/
    /// W13-wasm-simd-v128-first-slice.md` for the full design.
    V128,

    /// `exnref` — `(ref null exn)`, a nullable reference to a caught
    /// exception (exceptions proposal; real spec type opcode `-0x17`).
    ///
    /// Encoded as the single byte `0x69` — the correct SLEB128 single-byte
    /// encoding of `-0x17` (`-23 & 0x7F = 0x69`, matching how `funcref`
    /// (`-0x10` → `0x70`) and `externref` (`-0x11` → `0x6F`) are encoded:
    /// their raw byte values ARE the SLEB128 encoding, not `-0x17`'s
    /// two's-complement-mod-256 byte (`0xE9`), which this variant used
    /// PRIOR to W24 — a real bug, security-reviewed and fixed there: `0xE9`
    /// has its LEB128 continuation bit set (`0xE9 >= 0x80`), so it can
    /// never be a complete single-byte value on its own, meaning it was
    /// genuinely ambiguous with the leading byte of a real, attacker-
    /// controlled multi-byte type-index encoding in `wasm-validator`/
    /// `wasm-execution`'s blocktype decoders (any module declaring 234+
    /// types could trigger it). `0x69` has its continuation bit clear, so
    /// — like every other special-cased blocktype byte — it can ONLY ever
    /// mean "the complete value -23," never a partial prefix of a larger
    /// number.
    ///
    /// A real, reified value as of W24 (`code/specs/
    /// W24-wasm-exceptions-exnref-catch-ref.md`): a `catch_ref`/
    /// `catch_all_ref` clause that matches pushes a genuine `exnref` —
    /// `wasm-execution::WasmValue::Ref(Some(handle))`, a handle into
    /// `WasmExecutionContext::exception_heap` — that `throw_ref` can later
    /// pop to re-raise the exact same exception. Before W24 this variant
    /// existed only so a module mixing `exnref`-typed functions alongside
    /// ordinary `catch`/`catch_all`-only ones (the real testsuite's own
    /// `try_table.wast` does exactly this, all in ONE module) still parsed
    /// and structurally validated as a whole (see W22's own scope
    /// section) — that parsing/validation role is unchanged, now backed by
    /// real runtime semantics too.
    Exnref,

    /// `nullfuncref` (a.k.a. `(ref null nofunc)`) -- the **bottom type** of
    /// the func hierarchy (GC/function-references proposals, W32 first
    /// slice: `code/specs/W32-wasm-non-null-concrete-reference-types.md`).
    ///
    /// A strict subtype of every nullable func-hierarchy type: `Funcref`,
    /// and (once concrete non-null funcrefs land in a later slice)
    /// `ConcreteFuncRef(_)` for every index. It is the type WASM assigns to
    /// a bare `(ref.null nofunc)`/`(ref.null func)` result before that null
    /// value has been narrowed to a more specific slot -- see
    /// `ref_null.wast`.
    ///
    /// Encoded as a single byte `0x73` -- verified against the real
    /// reference interpreter's `interpreter/binary/decode.ml`
    /// (`NoFuncHT = -0x0d`, and `-13 mod 128 = 0x73`), independently of
    /// this crate's own doc comments, per the discipline W24's `exnref`
    /// tag-byte bug established.
    NullFuncref,

    /// `nullexternref` (a.k.a. `(ref null noextern)`) -- the bottom type of
    /// the extern hierarchy. A strict subtype of `Externref` only (the
    /// extern hierarchy has no concrete subtypes in this repo).
    ///
    /// Encoded as a single byte `0x72` -- verified against
    /// `decode.ml`'s `NoExternHT = -0x0e` (`-14 mod 128 = 0x72`).
    NullExternref,

    /// `nullexnref` (a.k.a. `(ref null noexn)`) -- the bottom type of the
    /// exn hierarchy. A strict subtype of `Exnref` only.
    ///
    /// Encoded as a single byte `0x74` -- verified against
    /// `decode.ml`'s `NoExnHT = -0x0c` (`-12 mod 128 = 0x74`).
    NullExnref,

    /// `nullref` (a.k.a. `none`, `(ref null none)`) -- the bottom type of
    /// the `any` hierarchy. A strict subtype of `Anyref`, `I31ref`, and
    /// `StructRef(_)` for every index -- but NOT of `NonNullStructRef(_)`
    /// (W32 second slice): a null value can never satisfy a non-null slot,
    /// no matter how far down the bottom of the lattice it sits. See
    /// [`ValueType::is_bottom_subtype_of`].
    ///
    /// Encoded as a single byte `0x71` -- verified against
    /// `decode.ml`'s `NoneHT = -0x0f` (`-15 mod 128 = 0x71`).
    NullRef,

    /// `(ref $T)` -- NON-NULL reference to a concrete struct type (GC
    /// proposal; W32 second slice: `code/specs/
    /// W32-wasm-non-null-concrete-reference-types.md`).
    ///
    /// The nullable counterpart is [`ValueType::StructRef`] -- see that
    /// variant's own doc comment for the shared index-space convention
    /// (`WasmModule::struct_types`, offset by `types.len()`). This variant
    /// is its STRICT subtype: `NonNullStructRef(i) <: StructRef(i)` and,
    /// transitively, `<: Anyref` (see [`ValueType::is_non_null_subtype_of`])
    /// -- never the reverse, matching the exact one-directional shape
    /// `ConcreteFuncRef <: Funcref` (W11-B) and the four W32-first-slice
    /// bottom types already established one level of this lattice up.
    ///
    /// Binary: `0x64 <LEB128(idx)>` -- the function-references proposal's
    /// "non-null" type-constructor byte, independently verified against
    /// the real reference interpreter's `interpreter/binary/decode.ml`
    /// (`ref_type`'s `-0x1c -> (NoNull, heap_type s)` arm: `-28 mod 128 =
    /// 0x64`), distinct from `StructRef`/`ConcreteFuncRef`'s `0x63`
    /// ("nullable", `decode.ml`'s `-0x1d -> (Null, heap_type s)`,
    /// `-29 mod 128 = 0x63`) by exactly one.
    ///
    /// This crate's own `wasm-wast-parser` has no struct-type TEXT-format
    /// declarations at all (see `StructRef`/`ConcreteFuncRef`'s own doc
    /// comments) -- so, like its nullable counterpart, no real `.wast`
    /// TEXT source can produce this variant naming a struct type today,
    /// only a func type (`NonNullConcreteFuncRef`, via `(ref $t)` in the
    /// function-type index space, the shape `call_ref.wast`/
    /// `return_call_ref.wast` actually exercise). It exists as a real,
    /// independently-constructible Rust variant (used directly in this
    /// crate's own subtyping unit tests, and reachable via binary struct-
    /// field decoding -- see `wasm-module-parser::read_value_type`) so the
    /// struct-type half of the lattice is not silently missing.
    NonNullStructRef(u32),

    /// `(ref $t)` -- NON-NULL reference to a concrete function type
    /// (function-references proposal; W32 second slice).
    ///
    /// The nullable counterpart is [`ValueType::ConcreteFuncRef`] -- see
    /// that variant's own doc comment for the shared function-type index
    /// space (`WasmModule::types`, index 0..N-1 directly, no offset). This
    /// variant is its STRICT subtype: `NonNullConcreteFuncRef(i) <:
    /// ConcreteFuncRef(i) <: Funcref` (both directions checked directly,
    /// not derived by chaining -- see [`ValueType::is_non_null_subtype_of`])
    /// -- never the reverse.
    ///
    /// Binary: `0x64 <LEB128(idx)>`, same 2-byte shape and same
    /// independently-verified tag byte as `NonNullStructRef`'s own doc
    /// comment describes -- disambiguated from it purely by which index
    /// space `idx` falls in, exactly like `StructRef`/`ConcreteFuncRef`'s
    /// existing `0x63` disambiguation.
    ///
    /// This is the type `call_ref`/`return_call_ref`'s real spec typing
    /// rule needs on the PRODUCING side: `ref.func $f : [] -> [(ref $t)]`
    /// (function-references proposal's `Overview.md`, verified directly,
    /// not assumed) pushes a non-null reference typed to `$f`'s OWN
    /// function-type index -- this repo's `wasm-validator` now reflects
    /// that real rule instead of the pre-W32-second-slice placeholder of
    /// pushing bare `Funcref` for every `ref.func`. `call_ref`/
    /// `return_call_ref` THEMSELVES, however, accept the NULLABLE
    /// `ConcreteFuncRef` on their consuming side and trap on null at
    /// runtime (`call_ref $t : [t1* (ref null $t)] -> [t2*]`, "traps on
    /// null" -- independently verified against the real spec text, NOT
    /// the non-null-only operand this repo's own W32 spec document
    /// originally assumed before this slice checked): a
    /// `NonNullConcreteFuncRef` value flows into that nullable slot fine
    /// via the direct subtyping rule above, it is simply never REQUIRED.
    NonNullConcreteFuncRef(u32),

    /// `(ref null $t)` where `$t` names a concrete WasmGC **array** type
    /// (W33 fourth slice: `code/specs/W33-wasm-gc-recursive-type-subtyping.md`)
    /// -- the array-hierarchy analogue of [`ValueType::StructRef`].
    ///
    /// The `u32` payload is a **flat type-section index** (the same shared
    /// index space every other concrete reference variant here uses), NOT
    /// an index directly into [`WasmModule::array_types`] -- resolve it via
    /// [`WasmModule::array_type_at`], which knows how to map a flat index to
    /// the right `array_types` entry for both this crate's own TEXT-format
    /// parser (`WasmModule::type_kinds`-aware) and any hand-built module
    /// that never populates `type_kinds` at all.
    ///
    /// Binary encoding: `0x63 <LEB128(idx)>`, the identical two-byte shape
    /// `StructRef`/`ConcreteFuncRef` already share -- disambiguated purely
    /// by which index space `idx` falls in, exactly like those two
    /// disambiguate from each other.
    ArrayRef(u32),

    /// `(ref $t)` -- the NON-NULL counterpart of [`ValueType::ArrayRef`],
    /// same relationship [`ValueType::NonNullStructRef`] has to
    /// `StructRef`. Binary encoding: `0x64 <LEB128(idx)>`, same tag byte as
    /// `NonNullStructRef`/`NonNullConcreteFuncRef`.
    NonNullArrayRef(u32),

    /// `(ref array)` -- non-null reference to ANY array type (the abstract
    /// TOP of the array hierarchy, W33 fourth slice), distinct from
    /// [`ValueType::NonNullArrayRef`] (a SPECIFIC array type). Needed
    /// because `array.wast`'s own vendored corpus text declares its
    /// `array.len` helper's param this way FOUR separate times (`(func
    /// $len (param $v (ref array)) (result i32) (array.len (local.get
    /// $v)))`) -- matching the real GC proposal's own `array.len` typing
    /// rule, whose operand type is the abstract `array` heap type, not a
    /// concrete one (an array's length is a property of the heap object
    /// itself, independent of which specific array type declared it).
    ///
    /// No nullable counterpart is modeled (`(ref null array)`) -- the real
    /// corpus never spells it, and every existing abstract-heap-type
    /// variant in this crate (`Anyref`, `Funcref`, `Externref`, ...) is
    /// ALREADY the nullable one, so adding only the non-null top type here
    /// mirrors that established "model what the corpus actually uses"
    /// discipline rather than adding an unused variant preemptively.
    ///
    /// Binary encoding: `0x64 0x66` (non-null reftype prefix + the `array`
    /// abstract heap-type byte) -- this crate's own internal choice, never
    /// round-tripped through a real binary decoder in this pipeline (see
    /// `ArrayRef`'s own doc comment on why that's safe).
    NonNullArrayAny,
}

impl ValueType {
    /// Return the single-byte tag for this type, or `None` for multi-byte types.
    ///
    /// This mirrors the WASM 1.0 convention where numeric types are single
    /// bytes.  WasmGC types like `StructRef` need two or more bytes; callers
    /// that need the full encoding should use [`ValueType::encode`] instead.
    ///
    /// ```text
    /// I32 → Some(0x7F),  I64 → Some(0x7E),  F32 → Some(0x7D),  F64 → Some(0x7C)
    /// Anyref → Some(0x6E),  I31ref → Some(0x6C)
    /// StructRef(_) → None   (needs 2+ bytes)
    /// ```
    pub fn byte_tag(&self) -> Option<u8> {
        match self {
            ValueType::I32 => Some(0x7F),
            ValueType::I64 => Some(0x7E),
            ValueType::F32 => Some(0x7D),
            ValueType::F64 => Some(0x7C),
            ValueType::Anyref => Some(0x6E),
            ValueType::I31ref => Some(0x6C),
            ValueType::StructRef(_) => None,
            ValueType::ConcreteFuncRef(_) => None,
            ValueType::Funcref => Some(0x70),
            ValueType::Externref => Some(0x6F),
            ValueType::V128 => Some(0x7B),
            ValueType::Exnref => Some(0x69),
            ValueType::NullFuncref => Some(0x73),
            ValueType::NullExternref => Some(0x72),
            ValueType::NullExnref => Some(0x74),
            ValueType::NullRef => Some(0x71),
            // W32 second slice: multi-byte, like `StructRef`/`ConcreteFuncRef`.
            ValueType::NonNullStructRef(_) => None,
            ValueType::NonNullConcreteFuncRef(_) => None,
            // W33 fourth slice: multi-byte, same shape as the struct/func
            // concrete-reference variants above.
            ValueType::ArrayRef(_) => None,
            ValueType::NonNullArrayRef(_) => None,
            ValueType::NonNullArrayAny => None,
        }
    }

    /// Encode this `ValueType` into its WASM binary representation.
    ///
    /// ```text
    /// I32        → [0x7F]
    /// I64        → [0x7E]
    /// F32        → [0x7D]
    /// F64        → [0x7C]
    /// Anyref     → [0x6E]
    /// I31ref     → [0x6C]
    /// StructRef(n) → [0x63, ...LEB128(n)]
    /// ```
    ///
    /// The LEB128 encoding for small indices fits in one byte, so
    /// `StructRef(0)` → `[0x63, 0x00]` and `StructRef(127)` → `[0x63, 0x7F]`.
    pub fn encode(&self) -> Vec<u8> {
        use wasm_leb128::encode_unsigned;
        match self {
            ValueType::I32 => vec![0x7F],
            ValueType::I64 => vec![0x7E],
            ValueType::F32 => vec![0x7D],
            ValueType::F64 => vec![0x7C],
            ValueType::Anyref => vec![0x6E],
            ValueType::I31ref => vec![0x6C],
            ValueType::StructRef(idx) => {
                // 0x63 = nullable concrete reference tag.
                let mut bytes = vec![0x63u8];
                bytes.extend(encode_unsigned(*idx as u64));
                bytes
            }
            ValueType::ConcreteFuncRef(idx) => {
                // Same 2-byte shape as StructRef -- see ConcreteFuncRef's
                // own doc comment for why the two never collide.
                let mut bytes = vec![0x63u8];
                bytes.extend(encode_unsigned(*idx as u64));
                bytes
            }
            ValueType::Funcref => vec![0x70],
            ValueType::Externref => vec![0x6F],
            ValueType::V128 => vec![0x7B],
            ValueType::Exnref => vec![0x69],
            ValueType::NullFuncref => vec![0x73],
            ValueType::NullExternref => vec![0x72],
            ValueType::NullExnref => vec![0x74],
            ValueType::NullRef => vec![0x71],
            // W32 second slice: `0x64` -- the function-references
            // proposal's "non-null" type-constructor byte, independently
            // verified against the real reference interpreter's
            // `interpreter/binary/decode.ml` (see `NonNullStructRef`'s own
            // doc comment for the derivation) -- one more than `StructRef`/
            // `ConcreteFuncRef`'s `0x63` ("nullable"), same 2-byte shape.
            ValueType::NonNullStructRef(idx) => {
                let mut bytes = vec![0x64u8];
                bytes.extend(encode_unsigned(*idx as u64));
                bytes
            }
            ValueType::NonNullConcreteFuncRef(idx) => {
                let mut bytes = vec![0x64u8];
                bytes.extend(encode_unsigned(*idx as u64));
                bytes
            }
            // W33 fourth slice: same two tag bytes as the struct/func
            // concrete-reference variants -- see `ArrayRef`/`NonNullArrayRef`'s
            // own doc comments for why they never collide (different index
            // space, not a different byte).
            ValueType::ArrayRef(idx) => {
                let mut bytes = vec![0x63u8];
                bytes.extend(encode_unsigned(*idx as u64));
                bytes
            }
            ValueType::NonNullArrayRef(idx) => {
                let mut bytes = vec![0x64u8];
                bytes.extend(encode_unsigned(*idx as u64));
                bytes
            }
            ValueType::NonNullArrayAny => vec![0x64, 0x66],
        }
    }

    /// Whether `self` is a strict (never bidirectional) subtype of `other`
    /// under the W32 first-slice bottom-type lattice
    /// (`code/specs/W32-wasm-non-null-concrete-reference-types.md` §2).
    ///
    /// This is intentionally narrow: it only encodes the four bottom types'
    /// relationships to the nullable reference types that already exist in
    /// this crate. It does NOT attempt general reference-type subtyping
    /// (that's `wasm-validator`'s `is_assignable`-shaped logic, which calls
    /// this as one part of its lattice) and it does NOT include reflexivity
    /// (`T <: T`) -- callers that need "is `have` assignable to `want`"
    /// should check `have == want || have.is_bottom_subtype_of(want)`.
    ///
    /// ```text
    /// Func hierarchy:    NullFuncref   <: Funcref
    /// Extern hierarchy:  NullExternref <: Externref
    /// Exn hierarchy:     NullExnref    <: Exnref
    /// Any hierarchy:     NullRef       <: Anyref, I31ref, StructRef(_)
    /// ```
    ///
    /// The reverse direction never holds -- a nullable supertype is never a
    /// subtype of a narrower bottom type, matching the asymmetry
    /// `ConcreteFuncRef <: Funcref` (W11-B) already established one level
    /// up the same lattice.
    pub fn is_bottom_subtype_of(&self, other: &ValueType) -> bool {
        matches!(
            (self, other),
            (ValueType::NullFuncref, ValueType::Funcref)
                | (ValueType::NullFuncref, ValueType::ConcreteFuncRef(_))
                | (ValueType::NullExternref, ValueType::Externref)
                | (ValueType::NullExnref, ValueType::Exnref)
                | (ValueType::NullRef, ValueType::Anyref)
                | (ValueType::NullRef, ValueType::I31ref)
                | (ValueType::NullRef, ValueType::StructRef(_))
                // W33 fourth slice: `none` is the bottom of the WHOLE `any`
                // hierarchy, so it sits below `ArrayRef(_)` too, exactly
                // like it already does below `StructRef(_)` above.
                | (ValueType::NullRef, ValueType::ArrayRef(_))
        )
    }

    /// Whether `self` is a strict (never bidirectional) subtype of `other`
    /// under the W32 SECOND-slice non-null lattice (`code/specs/
    /// W32-wasm-non-null-concrete-reference-types.md` §2):
    ///
    /// ```text
    /// NonNullStructRef(i)     <: StructRef(i)        (same index)
    /// NonNullStructRef(i)     <: Anyref              (any index)
    /// NonNullConcreteFuncRef(i) <: ConcreteFuncRef(i)  (same index)
    /// NonNullConcreteFuncRef(i) <: Funcref             (any index)
    /// ```
    ///
    /// Both hops of each chain (`NonNullStructRef <: StructRef <: Anyref`,
    /// `NonNullConcreteFuncRef <: ConcreteFuncRef <: Funcref`) are listed
    /// here directly rather than derived by composing two separate
    /// one-hop checks -- `is_assignable` (`wasm-validator`) does not do
    /// transitive closure, so a type two hops down the lattice needs its
    /// own direct rule to the top, exactly the same non-derived shape
    /// `ConcreteFuncRef <: Funcref` (W11-B) already used one level up.
    ///
    /// The reverse direction never holds -- a NULLABLE type is never a
    /// subtype of its non-null counterpart, no matter how it's reached
    /// (see [`ValueType::is_bottom_subtype_of`]'s own doc comment on
    /// `NullRef`/`NullFuncref` for the same asymmetry one level further
    /// down): `StructRef(i)` is NOT `<: NonNullStructRef(i)`, and neither
    /// bottom type (`NullRef`/`NullFuncref`) is `<:` either non-null
    /// variant -- a null value can never satisfy a non-null slot.
    ///
    /// Named distinctly from `is_bottom_subtype_of` (rather than merged
    /// into one lattice method) because these two rule sets were added in
    /// two different slices of the same spec and check membership in two
    /// different-shaped relations (bottom-of-hierarchy vs. non-null-of-
    /// nullable) -- keeping them separate mirrors how the doc comments on
    /// each new `ValueType` variant already attribute their rules to a
    /// specific slice.
    pub fn is_non_null_subtype_of(&self, other: &ValueType) -> bool {
        matches!(
            (self, other),
            (ValueType::NonNullStructRef(i), ValueType::StructRef(j)) if i == j
        ) || matches!(
            (self, other),
            (ValueType::NonNullArrayRef(i), ValueType::ArrayRef(j)) if i == j
        ) || matches!(
            (self, other),
            (ValueType::NonNullStructRef(_), ValueType::Anyref)
                | (ValueType::NonNullArrayRef(_), ValueType::Anyref)
                | (ValueType::NonNullConcreteFuncRef(_), ValueType::Funcref)
                // W33 fourth slice: a SPECIFIC array type's non-null ref
                // flows wherever the abstract non-null array top type is
                // expected (`array.wast`'s own `array.len` helper, whose
                // param is bare `(ref array)`) -- and that abstract top
                // type is itself, transitively, `<: anyref` (both listed
                // directly rather than derived, matching this method's own
                // documented "no transitive closure" contract).
                | (ValueType::NonNullArrayRef(_), ValueType::NonNullArrayAny)
                | (ValueType::NonNullArrayAny, ValueType::Anyref)
        ) || matches!(
            (self, other),
            (ValueType::NonNullConcreteFuncRef(i), ValueType::ConcreteFuncRef(j)) if i == j
        )
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// WasmGC struct types
// ──────────────────────────────────────────────────────────────────────────────

/// One field in a WasmGC struct type.
///
/// In a `.wat` module, this corresponds to:
/// ```wat
/// (field $name (mut <val_type>))   ;; mutable
/// (field $name <val_type>)         ;; immutable
/// ```
///
/// The binary encoding of a field in the type section is:
/// ```text
/// <val_type encoding>   ;; the ValueType bytes
/// <mutability: 0x00 or 0x01>
/// ```
///
/// For a `LispyPair`, both `$head` and `$tail` are mutable `anyref` fields:
/// ```text
/// field $head: [0x6E, 0x01]   ;; anyref, mutable
/// field $tail: [0x6E, 0x01]   ;; anyref, mutable
/// ```
/// A field/element's declared **storage type** (WasmGC's `storagetype`
/// grammar: `storagetype ::= valtype | packedtype`, W33 fourth slice).
///
/// Most fields just hold an ordinary [`ValueType`] (`i32`, `anyref`, a
/// concrete reference, ...). The GC proposal ALSO allows two **packed**
/// storage types that exist only inside a struct field or array element —
/// never as a local, param, result, or global type — because they're a
/// storage-density optimization, not a real value type: `struct.get`/
/// `array.get` always sign- or zero-EXTEND a packed field back out to a
/// full `i32` the moment it's read (hence the mandatory `_s`/`_u` suffix
/// on those two ops specifically — see `struct.wast`'s own "Packed field
/// instructions" section), and `struct.set`/`array.set` TRUNCATE an `i32`
/// down to the field's real width on write.
///
/// ```text
/// (field i8)              -- StorageType::I8
/// (field (mut i16))       -- StorageType::I16, FieldType::mutable = true
/// (field i32)             -- StorageType::Val(ValueType::I32)
/// (field (ref $vec))      -- StorageType::Val(ValueType::NonNullStructRef/ArrayRef(_))
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageType {
    /// An ordinary value type — the overwhelming majority of fields.
    Val(ValueType),
    /// `i8` — packed 8-bit storage, sign/zero-extended to `i32` on read.
    I8,
    /// `i16` — packed 16-bit storage, sign/zero-extended to `i32` on read.
    I16,
}

impl StorageType {
    /// The `i32`-or-wider type a `struct.get`/`array.get` of this storage
    /// actually pushes onto the stack (packed storage always widens to a
    /// full `i32`; an ordinary [`ValueType`] round-trips unchanged). This is
    /// the type `wasm-validator`'s static stack-effect checker needs, NOT
    /// the on-disk/in-memory width.
    pub fn widened_type(&self) -> ValueType {
        match self {
            StorageType::Val(vt) => *vt,
            StorageType::I8 | StorageType::I16 => ValueType::I32,
        }
    }

    /// Whether this storage type is packed (`i8`/`i16`) — packed fields are
    /// the only ones that need a signed-vs-unsigned read distinction
    /// (`struct.get_s`/`struct.get_u`, `array.get_s`/`array.get_u`); an
    /// ordinary [`ValueType`] field only ever has a single `struct.get`/
    /// `array.get` reading it.
    pub fn is_packed(&self) -> bool {
        matches!(self, StorageType::I8 | StorageType::I16)
    }

    /// The storage width in bits, for packed storage only (`None` for an
    /// ordinary [`ValueType`], which has no truncation to perform).
    pub fn packed_bits(&self) -> Option<u32> {
        match self {
            StorageType::I8 => Some(8),
            StorageType::I16 => Some(16),
            StorageType::Val(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldType {
    /// The storage type of this field (W33 fourth slice: widened from a
    /// plain [`ValueType`] to [`StorageType`] so packed `i8`/`i16` fields
    /// — real WasmGC vocabulary this crate had no representation for at
    /// all before — have somewhere to live).
    pub storage: StorageType,
    /// Whether this field can be modified after the struct is created.
    /// `true` → mutable (heap write is legal via `struct.set`).
    /// `false` → immutable (write-once, set during `struct.new`).
    pub mutable: bool,
}

impl FieldType {
    /// Build a field/element with an ordinary (non-packed) value type —
    /// the common case, and a drop-in replacement for the pre-W33-fourth-
    /// slice `FieldType { val_type, mutable }` literal shape.
    pub fn plain(val_type: ValueType, mutable: bool) -> Self {
        FieldType { storage: StorageType::Val(val_type), mutable }
    }
}

/// A WasmGC struct type definition — an ordered list of fields.
///
/// In the WAT text format, this looks like:
/// ```wat
/// (type $LispyPair (struct
///   (field $head (mut (ref null any)))
///   (field $tail (mut (ref null any)))))
/// ```
///
/// ## Why structs?
///
/// WasmGC structs are the GC heap analogue of C structs.  They are
/// *heterogeneous* (fields can have different types) and *garbage-collected*
/// (the runtime tracks them; no `free` needed).  For a Lisp runtime, each
/// cons cell is a two-field struct with `$head` (the car) and `$tail` (the
/// cdr), both of type `anyref` so they can hold any Lisp value.
///
/// ## Type-section index
///
/// In the WASM binary, struct types live alongside function types in the
/// type section.  We store them separately in [`WasmModule::struct_types`]
/// and interleave them with function types during encoding.  Struct types
/// appear *after* all function types, so if there are `N` function types,
/// struct type `k` is at type-section index `N + k`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructType {
    /// The fields of this struct, in declaration order.
    /// Field index 0 is the first field, 1 the second, and so on.
    pub fields: Vec<FieldType>,
}

/// A WasmGC array type definition — a single, homogeneous, dynamically-sized
/// element type (W33 fourth slice: `code/specs/
/// W33-wasm-gc-recursive-type-subtyping.md`).
///
/// ```wat
/// (type $vec (array f32))            ;; immutable f32 elements
/// (type $mvec (array (mut f32)))     ;; mutable f32 elements
/// (type $bytes (array (mut i8)))     ;; packed, mutable
/// ```
///
/// Unlike [`StructType`] (a fixed-size, heterogeneous field LIST), an array
/// has exactly ONE element type/mutability pair — reusing [`FieldType`] for
/// it (rather than a bespoke `(StorageType, bool)` tuple) keeps the "storage
/// type + mutable flag" shape defined in exactly one place, and lines up
/// with the real GC proposal's own grammar, which defines `arraytype ::=
/// fieldtype` verbatim (an array type IS a single field type, structurally).
///
/// See [`WasmModule::array_types`]/[`WasmModule::array_type_at`] for how an
/// array type's own flat type-section index is resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayType {
    /// This array's element storage type and mutability.
    pub element: FieldType,
}

// ──────────────────────────────────────────────────────────────────────────────
// BlockType
// ──────────────────────────────────────────────────────────────────────────────

/// The "result type" of a structured control flow block (`block`, `loop`, `if`).
///
/// In WASM 1.0, a block can either produce no value, produce a single value,
/// or (with the multi-value proposal) produce multiple values by referencing a
/// `FuncType` in the type section.
///
/// ```text
/// block  ;; begins a block
///   i32.const 42
/// end    ;; leaves 42 on the stack if block_type = Value(I32)
/// ```
///
/// The byte encoding in the WASM binary:
/// ```text
///  0x40  →  Empty  (no result)
///  0x7F  →  Value(I32)
///  0x7E  →  Value(I64)
///  0x7D  →  Value(F32)
///  0x7C  →  Value(F64)
///  u32   →  TypeIndex(n)  (non-negative LEB128 integer)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockType {
    /// The block produces no values (the most common case).
    Empty,
    /// The block produces exactly one value of this type.
    Value(ValueType),
    /// The block's signature is a full function type from the type section.
    /// This is the "multi-value" extension to WASM 1.0.
    TypeIndex(u32),
}

/// The byte tag for an empty block type in the WASM binary format.
///
/// ```text
/// 0x40 in hex = 64 in decimal = -64 in signed LEB128
/// ```
pub const BLOCK_TYPE_EMPTY: u8 = 0x40;

// ──────────────────────────────────────────────────────────────────────────────
// ExternalKind
// ──────────────────────────────────────────────────────────────────────────────

/// What kind of entity is imported from or exported to the host environment.
///
/// A WASM module's boundary with the outside world is entirely described by
/// imports and exports. Both use `ExternalKind` to say *what* is being
/// imported/exported.
///
/// ```text
/// Byte encoding
/// ┌──────────┬──────┐
/// │  Kind    │ Byte │
/// ├──────────┼──────┤
/// │ Function │ 0x00 │
/// │ Table    │ 0x01 │
/// │ Memory   │ 0x02 │
/// │ Global   │ 0x03 │
/// └──────────┴──────┘
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExternalKind {
    /// A function — the most commonly imported/exported entity.
    Function = 0x00,
    /// A table — an array of references (function pointers in WASM 1.0).
    Table = 0x01,
    /// A memory — a linear block of bytes shared with the host.
    Memory = 0x02,
    /// A global variable.
    Global = 0x03,
    /// A tag (W21 — the exceptions proposal's `throw`/`try_table`). Matches
    /// the real exception-handling proposal's binary encoding exactly (its
    /// own `Exceptions.md` names `4` as the `Tag` import/export kind byte,
    /// live-fetched and confirmed, not assumed).
    Tag = 0x04,
}

// ──────────────────────────────────────────────────────────────────────────────
// FuncType
// ──────────────────────────────────────────────────────────────────────────────

/// A function's type signature: parameter types and result types.
///
/// WASM 1.0 allows at most one result type (the multi-value proposal lifts
/// this restriction, but the *type section* already supports vectors of
/// results even in 1.0). All function signatures are stored in the **type
/// section** and referenced by index elsewhere in the binary.
///
/// ```text
/// Binary encoding (in the type section):
///   0x60                  ;; function type tag
///   <num_params: LEB128>  ;; number of parameters
///   <param_type>*         ;; one byte per param (ValueType encoding)
///   <num_results: LEB128>
///   <result_type>*
///
/// Example: (i32, i64) -> f32
///   0x60  02  7F 7E  01  7D
///   │     │   │  │   │   └── result: F32
///   │     │   │  │   └────── 1 result
///   │     │   │  └────────── param[1]: I64
///   │     │   └──────────── param[0]: I32
///   │     └──────────────── 2 params
///   └────────────────────── func type tag
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncType {
    /// The types of the function's parameters, in order.
    pub params: Vec<ValueType>,
    /// The types of the function's return values, in order.
    pub results: Vec<ValueType>,
}

// ──────────────────────────────────────────────────────────────────────────────
// W33 first slice: GC-proposal `(sub [final] $parent (comptype))` nominal
// subtyping + `(rec ...)` type groups
// ──────────────────────────────────────────────────────────────────────────────

/// Per-type-section-entry nominal-subtyping metadata for the WebAssembly GC
/// proposal's `(sub [final] $parent* (comptype))` declaration syntax (W33
/// first slice: `code/specs/W33-wasm-gc-recursive-type-subtyping.md`).
///
/// Parallel array to [`WasmModule::types`] (function types only, this
/// slice -- struct/array TEXT-format declarations remain unparseable, see
/// `wasm-wast-parser`'s own doc comments): `WasmModule::type_subtyping[i]`
/// describes `types[i]`'s own declared supertype (if any), whether it
/// forecloses further subtyping, and which `(rec ...)` group it belongs to.
///
/// Any `WasmModule` that predates this field (every hand-built test
/// literal across this workspace, and `wasm-module-parser`'s binary
/// decoder, which has no GC `sub`/`rec` binary encoding at all) simply
/// leaves this vector empty or shorter than `types` -- every accessor here
/// ([`WasmModule::type_subtyping_at`] and everything built on it) treats a
/// missing entry as [`TypeSubtyping::default`] (final, no supertype,
/// singleton group), exactly the semantics every pre-W33 type already had.
/// This is a deliberate design choice, not an oversight: it means adding
/// this field never requires touching any of this workspace's many
/// existing `WasmModule { .. }` literals, and never risks an
/// index-out-of-bounds panic on one that doesn't know about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeSubtyping {
    /// The declared `sub $parent` supertype, if any -- an index into
    /// [`WasmModule::types`], the SAME "function-type index space, no
    /// offset" convention [`ValueType::ConcreteFuncRef`]/
    /// [`ValueType::NonNullConcreteFuncRef`] already use. `None` for a
    /// type with no `sub` clause at all, or an explicit `(sub (comptype))`
    /// with no parent listed.
    ///
    /// This crate only tracks a SINGLE declared supertype: the GC
    /// proposal's grammar allows `sub final? typeidx* comptype` (zero or
    /// more supertypes syntactically), but restricts to at most one in
    /// practice, and the real corpus never exercises more than one.
    pub supertype: Option<u32>,
    /// Whether this type forecloses further subtyping: `true` for a type
    /// with NO `sub` clause at all (the MVP/pre-GC default, per the real
    /// GC proposal's own rule) or an explicit `(sub final ...)`; `false`
    /// only for an explicit `(sub ...)` (with or without a `$parent`) that
    /// omits `final`.
    pub is_final: bool,
    /// How many sibling members (including itself) share this type's
    /// `(rec ...)` group -- `1` for a type declared standalone (an
    /// implicit singleton group, the real GC proposal's own rule for every
    /// non-`rec`-wrapped `(type ...)`) or inside an explicit `(rec (type
    /// ...))` with exactly one member.
    pub rec_group_size: u32,
    /// This type's own zero-based position within its `(rec ...)` group
    /// (see `rec_group_size`) -- always `0` for a singleton group.
    ///
    /// Needed ONLY for cross-module comparisons (import/tag type
    /// matching, `wasm-runtime`): the real WebAssembly GC proposal's
    /// canonical type-group equivalence algorithm considers two
    /// structurally-identical members of a `rec` group at DIFFERENT
    /// positions to be DISTINCT types (see `tag.wast`'s own
    /// `assert_unlinkable` case: `(rec (type $t1 (func)) (type $t2
    /// (func)))`'s `$t1`/`$t2` are byte-identical bodies at different
    /// positions, and importing under the wrong one must fail). This
    /// slice does NOT implement the real algorithm (`code/specs/
    /// W33-wasm-gc-recursive-type-subtyping.md`'s own "recursive
    /// type-group canonical equivalence" is explicitly out of scope) --
    /// a same-module (WITHIN-module) nominal `sub`-chain check never
    /// needs this field at all (a module's own type-section index is
    /// already a unique, unambiguous identity), so this field's only
    /// consumer is the conservative cross-module guard `wasm-runtime`
    /// applies alongside its pre-existing structural `FuncType` equality
    /// check: requiring a matching `(rec_group_size, rec_group_position)`
    /// on both sides NEVER accepts a canonically-DIFFERENT pair the real
    /// algorithm would reject (it only adds a strictly stronger necessary
    /// condition on top of a check that already runs), so it can only
    /// PREVENT a false accept, never cause a false reject beyond what the
    /// pre-existing simpler check already risked.
    pub rec_group_position: u32,
}

impl Default for TypeSubtyping {
    fn default() -> Self {
        Self { supertype: None, is_final: true, rec_group_size: 1, rec_group_position: 0 }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// W33 fourth slice: `type_kinds`, the flat-type-index -> {func,struct,array}
// composite-kind ledger struct/array TEXT-format parsing needs.
// ──────────────────────────────────────────────────────────────────────────────

/// What kind of composite type lives at one flat type-section index, and
/// (for struct/array) which real slot in [`WasmModule::struct_types`]/
/// [`WasmModule::array_types`] holds its actual data.
///
/// ## Why this exists
///
/// Before W33's fourth slice, `WasmModule` assumed every struct type is
/// encoded at type-section index `types.len() + k` (see `StructType`'s own
/// doc comment) — true for the BINARY format (whose type section is a fixed,
/// already-fully-decoded sequence: all func types, then all struct types,
/// exactly matching that formula) and for `wasm-wast-parser`'s pre-existing
/// func-only `(rec ...)`/`(sub ...)` machinery (which only ever grows
/// `types`, never `struct_types`).
///
/// Real WAT text, however, freely INTERLEAVES `(type $t (struct ...))`
/// declarations among `(type $t (func ...))` ones (`struct.wast`/
/// `array.wast`'s own "Binding structure" modules both do this), AND
/// `wasm-wast-parser`'s own two-pass design (`collect_symbols` then `build`)
/// can append MORE func types to `types` — via `dedup_type`, for a
/// function's inline-only signature — in the SECOND pass, strictly AFTER
/// every struct/array type has already been assigned its flat index in the
/// first pass. Both facts together break the `types.len() + k` formula: a
/// struct declared when `types.len() == 0` gets flat index `0`, but if pass
/// 2 later grows `types` to length 5, `struct_field_count`-style code
/// re-deriving the struct's index as `flat_idx - types.len()` would compute
/// `0 - 5`, an underflow, for a struct that parsed and validated perfectly
/// well.
///
/// `type_kinds[flat_idx]` sidesteps this entirely by recording each type's
/// real location DIRECTLY, at declaration time, rather than re-deriving it
/// from vector lengths that can still change later. It is a parallel array
/// to `types` (same length, same append-only growth, ALWAYS pushed in
/// lockstep by every code path that pushes to `types` — see `dedup_type`'s
/// updated doc comment) — a func-kind entry at index `i` names its real
/// payload's position within `types` itself (`types[i]` IS that payload,
/// unchanged from every pre-W33-fourth-slice consumer's assumption); a
/// struct/array-kind entry's `types[i]` slot instead holds an unused, never-
/// read dummy `FuncType` (kept only to preserve the "one slot per flat
/// index" length invariant `dedup_type`'s dedup-search relies on skipping).
///
/// Left EMPTY (or shorter than `types`) for every module built without this
/// bookkeeping — the binary decoder, every hand-built `WasmModule` literal
/// in this workspace's existing tests, and any OLDER text-format module —
/// exactly [`TypeSubtyping`]'s own "missing means legacy default" contract.
/// [`WasmModule::struct_type_at`]/[`WasmModule::array_type_at`] fall back to
/// the pre-existing `types.len() + k` offset formula whenever `type_kinds`
/// doesn't cover the index in question, so every pre-existing binary/LANG77
/// struct consumer is completely unaffected by this field's existence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    /// `types[flat_idx]` (the SAME index, since a func-kind entry never
    /// needs the struct/array indirection) holds this flat index's real
    /// `FuncType` directly.
    Func,
    /// `struct_types[.0]` holds this flat index's real `StructType`;
    /// `types[_]` (same flat index) holds an unused dummy `FuncType`.
    Struct(u32),
    /// `array_types[.0]` holds this flat index's real `ArrayType`;
    /// `types[_]` (same flat index) holds an unused dummy `FuncType`.
    Array(u32),
}

// ──────────────────────────────────────────────────────────────────────────────
// Limits
// ──────────────────────────────────────────────────────────────────────────────

/// Size constraints (min and optional max) for a memory or table.
///
/// Sizes are in *pages* for memories (1 page = 64 KiB = 65536 bytes) and
/// in *entries* for tables.
///
/// ```text
/// Binary encoding:
///   0x00  <min: LEB128>            ;; no maximum
///   0x01  <min: LEB128>  <max: LEB128>  ;; with maximum
///
/// Example: at least 1 page, at most 4 pages
///   0x01  01  04
/// ```
///
/// The WASM spec requires `min <= max` when a maximum is specified.
///
/// `min`/`max` are `u64`, not `u32` (W25 / memory64 proposal): a 64-bit
/// memory's limits can be declared up to `2^48` pages (the real spec's own
/// ceiling for a `memory64`-flagged memory — see `code/specs/
/// W25-wasm-memory64-first-slice.md`), which doesn't fit `u32`. `TableType`
/// shares this same struct and stays well within `u32`'s range for every
/// value this repo has ever seen (`table64`, the analogous widening for
/// tables, is a separate, out-of-scope proposal — see that spec's own
/// "Explicitly out of scope" section) — widening here is a pure, numerically
/// non-breaking generalization for every existing table/32-bit-memory caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    /// Minimum size (must always be present).
    pub min: u64,
    /// Optional maximum size. `None` means unbounded.
    pub max: Option<u64>,
}

// ──────────────────────────────────────────────────────────────────────────────
// MemoryType
// ──────────────────────────────────────────────────────────────────────────────

/// The type of a linear memory — just its size limits.
///
/// WASM 1.0 allows at most one memory per module. A memory is a contiguous
/// array of bytes that the module and host can both read and write. It can
/// grow at runtime via the `memory.grow` instruction (up to `limits.max`).
///
/// ```text
/// Host (JavaScript)           WASM module
/// ┌──────────────────────────────────────┐
/// │ memory = new WebAssembly.Memory(     │
/// │   { initial: 1, maximum: 4 }         │
/// │ )                                    │
/// └──────────────────────────────────────┘
///      limits = Limits { min: 1, max: Some(4) }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryType {
    /// The size constraints on this memory.
    pub limits: Limits,
    /// Whether this memory is declared `shared` (WASM18 / threads
    /// proposal, `(memory 1 1 shared)` in text). Purely a static/
    /// validation-time property in this repo — `wasm-execution` is a
    /// single-threaded interpreter, so there is no second agent to
    /// actually share memory with; `shared`-ness only gates which
    /// memories the atomic instruction family is allowed to touch (see
    /// `code/specs/W09-wasm-atomics-plain.md`). Defaults to `false`,
    /// matching every pre-existing (non-atomic) module.
    pub shared: bool,
    /// Whether this memory uses 64-bit addressing (memory64 proposal,
    /// W25: `(memory i64 ...)` in text, binary `limits` flags bit `0x04`).
    /// When `true`, `memory.size`/`memory.grow` and every load/store
    /// instruction targeting this memory operate on `i64` addresses
    /// instead of `i32` — see `code/specs/
    /// W25-wasm-memory64-first-slice.md`. Defaults to `false`, matching
    /// every pre-existing (32-bit) memory.
    pub is64: bool,
}

// ──────────────────────────────────────────────────────────────────────────────
// TableType
// ──────────────────────────────────────────────────────────────────────────────

/// The `funcref` element type — the only table element type in WASM 1.0.
///
/// WASM 1.0 tables hold function references. The byte value 0x70 is the tag
/// for `funcref` in the binary format. (The `externref` type was added in a
/// later proposal.)
pub const FUNCREF: u8 = 0x70;

/// The `externref` element type (reference-types proposal, task #96) —
/// an opaque host reference, distinct from `funcref`. Byte value 0x6F.
pub const EXTERNREF: u8 = 0x6F;

/// The type of a WASM table: an array of references with size limits.
///
/// Tables in WASM 1.0 hold function references (`funcref`). They are used
/// to implement indirect function calls: the `call_indirect` instruction
/// takes an index into the table and calls the function stored there.
///
/// ```text
/// Table layout (conceptually):
///
///   index:    0         1         2         3
///           ┌─────────┬─────────┬─────────┬─────────┐
///           │ func #5 │  null   │ func #2 │ func #7 │  ...
///           └─────────┴─────────┴─────────┴─────────┘
///
/// call_indirect type_idx table_idx
///   → pops an i32 index, looks up function ref, validates type, calls it
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableType {
    /// The reference type stored in this table.
    /// In WASM 1.0, this is always `FUNCREF` (0x70).
    pub element_type: u8,
    /// The size constraints on this table.
    pub limits: Limits,
    /// Whether this table uses 64-bit addressing (table64 proposal, W26:
    /// `(table i64 ...)` in text, binary `limits` flags bit `0x04` — the
    /// same flag bit `MemoryType::is64`, W25, already recognizes). When
    /// `true`, an eventual `table.get`/`table.set`/`table.grow`/
    /// `table.size`/`call_indirect` against this table would use an `i64`
    /// index instead of `i32` — W26 itself only wires the *declaration*
    /// and import-linking-compatibility surface, not those instructions
    /// (see `code/specs/W26-wasm-table64-first-slice.md`'s "Explicitly out
    /// of scope"). Defaults to `false`, matching every pre-existing
    /// (32-bit) table.
    pub is64: bool,
}

// ──────────────────────────────────────────────────────────────────────────────
// GlobalType
// ──────────────────────────────────────────────────────────────────────────────

/// The type of a global variable: its value type and mutability.
///
/// Immutable globals are constants (e.g., the base address of a data section).
/// Mutable globals hold state that can change across calls (e.g., a stack
/// pointer for a language runtime).
///
/// ```text
/// Binary encoding:
///   <value_type: byte>  <mutability: 0x00 or 0x01>
///
/// Example: mutable i32
///   7F 01
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalType {
    /// The type of value stored in this global.
    pub value_type: ValueType,
    /// Whether this global can be modified after initialization.
    /// `true` → mutable (`var`), `false` → immutable (`const`).
    pub mutable: bool,
}

// ──────────────────────────────────────────────────────────────────────────────
// Import / ImportTypeInfo
// ──────────────────────────────────────────────────────────────────────────────

/// Additional type information specific to each import kind.
///
/// An import declaration says "I need *this* from the host environment." The
/// `ImportTypeInfo` says what shape that thing must have.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportTypeInfo {
    /// Import a function; carries the index into the module's type section.
    Function(u32),
    /// Import a table; carries the table's type.
    Table(TableType),
    /// Import a memory; carries the memory's type.
    Memory(MemoryType),
    /// Import a global; carries the global's type.
    Global(GlobalType),
    /// Import a tag (W21); carries the index into the module's type
    /// section for the tag's function signature (its `results` must be
    /// empty — a module-level rule, checked by `wasm-validator`, not here).
    Tag(u32),
}

/// A single import declaration from the import section.
///
/// Every import names the *module* that provides it (e.g., `"env"`, `"wasi_snapshot_preview1"`)
/// and the *name* within that module (e.g., `"memory"`, `"fd_write"`).
///
/// ```text
/// Binary encoding (import section entry):
///   <module_name: length-prefixed UTF-8>
///   <name: length-prefixed UTF-8>
///   <kind: ExternalKind byte>
///   <type_info: varies by kind>
///
/// Example: import function "env"."abort" with type index 0
///   03 "env"  05 "abort"  00  00
///   │         │           │   └── type index 0
///   │         │           └────── ExternalKind::Function
///   │         └────────────────── name = "abort"
///   └──────────────────────────── module = "env"
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    /// The module namespace (e.g., `"env"`, `"wasi_snapshot_preview1"`).
    pub module_name: String,
    /// The name within the module (e.g., `"memory"`, `"fd_write"`).
    pub name: String,
    /// The kind (function, table, memory, or global).
    pub kind: ExternalKind,
    /// Type-specific information about what is being imported.
    pub type_info: ImportTypeInfo,
}

// ──────────────────────────────────────────────────────────────────────────────
// Export
// ──────────────────────────────────────────────────────────────────────────────

/// A single export declaration from the export section.
///
/// Exports make module-internal entities visible to the host. For example,
/// a compiled C program's `main` function and its heap memory would both
/// be exported.
///
/// ```text
/// Binary encoding:
///   <name: length-prefixed UTF-8>
///   <kind: ExternalKind byte>
///   <index: LEB128>  ;; index into the relevant index space
///
/// Example: export function 3 as "main"
///   04 "main"  00  03
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Export {
    /// The name visible to the host (e.g., `"main"`, `"memory"`).
    pub name: String,
    /// The kind of entity being exported.
    pub kind: ExternalKind,
    /// Index into the appropriate index space (function, table, memory, or global).
    pub index: u32,
}

// ──────────────────────────────────────────────────────────────────────────────
// Global
// ──────────────────────────────────────────────────────────────────────────────

/// A module-defined global variable with its initialization expression.
///
/// Global variables are initialized by running a *constant expression*
/// (a short sequence of instructions that must produce a compile-time constant).
/// The `init_expr` field stores the raw bytes of that expression, ending with
/// the `end` opcode (0x0B).
///
/// ```text
/// Example: `(global i32 (i32.const 42))`
///   type:     GlobalType { value_type: I32, mutable: false }
///   init_expr: [0x41, 0x2A, 0x0B]
///               │     │     └── end opcode
///               │     └──────── LEB128(42)
///               └────────────── i32.const opcode
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Global {
    /// The type of this global.
    pub global_type: GlobalType,
    /// The raw bytes of the constant initializer expression (includes trailing `end` 0x0B).
    pub init_expr: Vec<u8>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Element
// ──────────────────────────────────────────────────────────────────────────────

/// An element segment — initializes a range of table entries with function indices.
///
/// Element segments are the mechanism for populating function tables. At
/// module instantiation time, the runtime copies `function_indices` into
/// the table specified by `table_index`, starting at the position computed
/// by `offset_expr`.
///
/// ```text
/// Conceptually:
///   table[offset_expr()] = [func_0, func_1, func_2, ...]
///
/// Use case: C/C++ function pointer tables, vtables, dynamic dispatch.
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    /// Index of the table to initialize. Meaningless when `is_passive` is
    /// `true` -- kept `0`/unset by convention rather than wrapped in an
    /// `Option`, matching `DataSegment.memory_index`'s own style.
    pub table_index: u32,
    /// Constant expression that computes the starting offset in the table.
    /// Empty when `is_passive` is `true` (a passive segment has no offset
    /// at all -- it is never applied at instantiation time, only copied on
    /// demand by an explicit `table.init`, task #97).
    pub offset_expr: Vec<u8>,
    /// The function indices to write into the table. `Some(idx)` for a
    /// real `ref.func idx` entry (or a bare funcidx-list entry, binary
    /// modes 0-3); `None` for a `ref.null` entry (binary exprs-list modes
    /// 4-7, task #97) -- an explicit null table slot, not merely absent.
    /// Same `Some`/`None` shape `Table::elements`/`WasmValue::Ref` already
    /// use for exactly this "funcref, nullable" concept.
    pub function_indices: Vec<Option<u32>>,
    /// `true` for a passive segment (bulk-table proposal, task #97):
    /// declared with no offset expression (`(elem funcref ...)`, or binary
    /// segment-mode flag `0x01`/`0x05`), so `wasm-runtime::instantiate()`
    /// never applies it automatically -- it stays resident until an
    /// explicit `table.init` copies from it (any number of times) or
    /// `elem.drop` frees its backing indices. `false` for an ordinary
    /// active segment, applied once at instantiation via `offset_expr`/
    /// `table_index` above. Same role `DataSegment.is_passive` plays for
    /// data segments (task #95).
    pub is_passive: bool,
}

// ──────────────────────────────────────────────────────────────────────────────
// DataSegment
// ──────────────────────────────────────────────────────────────────────────────

/// A data segment — initializes a region of linear memory with static bytes.
///
/// Data segments are how compiled programs load their static data (string
/// literals, lookup tables, initialized global variables) into WASM memory
/// at instantiation time.
///
/// ```text
/// Conceptually:
///   memory[offset_expr()] = data[0..data.len()]
///
/// Example: store the string "hello" at byte 1024
///   memory_index: 0
///   offset_expr:  [0x41, 0x80 0x08, 0x0B]  ;; i32.const 1024; end
///   data:         [0x68, 0x65, 0x6C, 0x6C, 0x6F]  ;; "hello"
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DataSegment {
    /// Index of the memory to write into (always 0 in WASM 1.0). Meaningless
    /// when `is_passive` is `true` -- kept `0`/unset by convention rather
    /// than wrapped in an `Option`, matching this struct's existing
    /// flat-fields style.
    pub memory_index: u32,
    /// Constant expression that computes the byte offset into memory.
    /// Empty when `is_passive` is `true` (a passive segment has no offset
    /// at all -- it is never applied at instantiation time, only copied on
    /// demand by an explicit `memory.init`, task #95).
    pub offset_expr: Vec<u8>,
    /// The raw bytes to copy into memory.
    pub data: Vec<u8>,
    /// `true` for a passive segment (bulk-memory proposal, task #95):
    /// declared with no offset expression (`(data $d "bytes")`, or binary
    /// segment-mode flag `0x01`), so `wasm-runtime::instantiate()` never
    /// applies it automatically -- it stays resident until an explicit
    /// `memory.init` copies from it (any number of times) or `data.drop`
    /// frees its backing bytes. `false` for an ordinary WASM 1.0 active
    /// segment, applied once at instantiation via `offset_expr`/
    /// `memory_index` above.
    pub is_passive: bool,
}

// ──────────────────────────────────────────────────────────────────────────────
// FunctionBody
// ──────────────────────────────────────────────────────────────────────────────

/// The body of a locally-defined function: its locals and its bytecode.
///
/// In the WASM binary, locals are declared compactly (e.g., "3 locals of type
/// i32, 2 locals of type f64"). This struct stores them fully expanded —
/// one `ValueType` per local slot — for convenient access.
///
/// ```text
/// Binary structure (code section entry):
///   <body_size: LEB128>
///   <num_local_decls: LEB128>
///   (<count: LEB128>  <type: byte>)*  ;; run-length encoded locals
///   <instructions...>
///   0x0B                              ;; end opcode
///
/// Example: function with 2 i32 locals and code [i32.const 1, end]
///   locals: [I32, I32]
///   code:   [0x41, 0x01, 0x0B]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionBody {
    /// Local variable types (expanded from run-length encoding).
    /// Parameters are NOT included here — they are in the `FuncType`.
    pub locals: Vec<ValueType>,
    /// Raw instruction bytes for the function body (including the trailing `end` 0x0B).
    pub code: Vec<u8>,
}

// ──────────────────────────────────────────────────────────────────────────────
// CustomSection
// ──────────────────────────────────────────────────────────────────────────────

/// A custom section — arbitrary named data that tools can embed in a WASM file.
///
/// Custom sections (section ID 0) are ignored by the WASM runtime but carry
/// valuable metadata for tooling:
///
/// - `"name"` section — maps function indices to human-readable names (for debuggers)
/// - `"sourceMappingURL"` — points to a source map file
/// - DWARF debug info sections (used by `wasm-pack`, Rust's WASM target, etc.)
///
/// ```text
/// Binary encoding:
///   0x00                           ;; section ID = custom
///   <section_size: LEB128>
///   <name: length-prefixed UTF-8>  ;; name of this custom section
///   <data: bytes>                  ;; arbitrary payload
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CustomSection {
    /// The name of this custom section (e.g., `"name"`, `"sourceMappingURL"`).
    pub name: String,
    /// The raw payload bytes of this custom section.
    pub data: Vec<u8>,
}

// ──────────────────────────────────────────────────────────────────────────────
// WasmModule
// ──────────────────────────────────────────────────────────────────────────────

/// A WebAssembly module, extended to support WasmGC struct types.
///
/// This struct holds all data from all sections of a `.wasm` file after
/// parsing (or after lowering from IIR).  It is the intermediate
/// representation between the raw binary and any higher-level analysis
/// (validation, interpretation, compilation, GC type emission).
///
/// ## Relationship between fields
///
/// ```text
/// types[i]      ←── functions[j] (type index)
///               ←── imports with ImportTypeInfo::Function(i)
///               ←── BlockType::TypeIndex(i)
///
/// functions[j]  ←── code[j - num_imported_funcs]  (function body)
///
/// struct_types[k] is encoded at type-section index  types.len() + k
///
/// tables[0]     ←── elements[e].table_index
///
/// memories[0]   ←── data[d].memory_index
/// ```
///
/// ## WasmGC type section layout
///
/// The WasmGC proposal extends the type section to carry both function types
/// and GC types.  In the binary, each entry is tagged:
///
/// - Function type: starts with `0x60` (the WASM 1.0 function-type prefix).
/// - Struct type (open sub-type): starts with `0x50 0x00 0x5F` (sub-type
///   marker, zero supertypes, struct marker).
///
/// We keep `types: Vec<FuncType>` for the function types and add a new
/// `struct_types: Vec<StructType>` for the GC types.  The encoder emits
/// function types first (indices 0..N-1), then struct types (indices N..).
///
/// The `Default` impl produces an empty module (no sections), which is the
/// natural starting point for an incremental builder or parser.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WasmModule {
    /// Type section (§1): all function signatures, deduplicated.
    pub types: Vec<FuncType>,

    /// Per-`types`-entry GC-proposal nominal-subtyping/`rec`-group metadata
    /// (W33 first slice) -- see [`TypeSubtyping`]'s own doc comment for why
    /// this is allowed to be shorter than `types` (or empty) and what that
    /// means for every accessor that reads it.
    pub type_subtyping: Vec<TypeSubtyping>,

    /// WasmGC struct type definitions (also in the type section, after `types`).
    ///
    /// Struct type `k` is at type-section index `types.len() + k` — this
    /// LEGACY formula only, still, when `type_kinds` is empty/doesn't cover
    /// the index (the binary decoder's own convention, unchanged); a
    /// `type_kinds`-aware module resolves a struct's real index via
    /// [`WasmModule::struct_type_at`] instead — see [`TypeKind`]'s own doc
    /// comment for why the two need to differ.
    /// When this vec is empty, the type section contains only function types
    /// and the encoding is identical to WASM 1.0.
    pub struct_types: Vec<StructType>,

    /// WasmGC array type definitions (W33 fourth slice) — the array
    /// counterpart of `struct_types`, resolved the same `type_kinds`-aware
    /// way via [`WasmModule::array_type_at`].
    pub array_types: Vec<ArrayType>,

    /// Per-flat-type-index composite-kind ledger (W33 fourth slice) — see
    /// [`TypeKind`]'s own doc comment for why this exists and what it means
    /// for a `WasmModule` (the overwhelming majority of this workspace's
    /// existing ones, all func-only or binary-decoded) that leaves it empty.
    pub type_kinds: Vec<TypeKind>,

    /// Import section (§2): things the module needs from the host.
    pub imports: Vec<Import>,
    /// Function section (§3): type indices for locally-defined functions.
    /// `functions[i]` is an index into `types`.
    pub functions: Vec<u32>,
    /// Table section (§4): function-reference tables.
    pub tables: Vec<TableType>,
    /// Memory section (§5): linear memory declarations.
    pub memories: Vec<MemoryType>,
    /// Global section (§6): module-defined global variables.
    pub globals: Vec<Global>,
    /// Export section (§7): names the module exposes to the host.
    pub exports: Vec<Export>,
    /// Start section (§8): optional index of a function to call on instantiation.
    pub start: Option<u32>,
    /// Element section (§9): table initialization data.
    pub elements: Vec<Element>,
    /// Code section (§10): function bodies (parallel array with `functions`).
    pub code: Vec<FunctionBody>,
    /// Data section (§11): memory initialization data.
    pub data: Vec<DataSegment>,
    /// Custom sections (§0): tool metadata (debug info, names, etc.).
    pub customs: Vec<CustomSection>,

    /// Tag section (§13, W21 — the exceptions proposal): type indices for
    /// MODULE-DEFINED tags only, in declaration order — mirrors
    /// `functions: Vec<u32>`'s own "imports live in `imports`, this Vec is
    /// only the module-defined ones" convention. The real binary section
    /// id (13) sits, in file position, between the memory section (5) and
    /// the global section (6) — same "numeric id != file position"
    /// convention the MVP's own `datacount` section (id 12, positioned
    /// between `elem` and `code`) already established; this repo's
    /// text-only `wasm-wast-parser` pipeline for this field never
    /// round-trips through a real binary layout, so that ordering detail
    /// doesn't affect anything here, only documented for fidelity.
    pub tags: Vec<u32>,
}

/// The hop cap [`nominal_subtype_chain`] enforces -- see that function's
/// own doc comment for why a bounded walk (rather than one scaled to a
/// module's `types.len()`) matters for algorithmic complexity, not just
/// termination.
const MAX_SUBTYPE_CHAIN_HOPS: u32 = 1_000;

/// Whether `sub_idx` is a reflexive, transitive NOMINAL subtype of
/// `super_idx`, per each type's own declared `sub $parent` chain (W33
/// first slice), walking `type_subtyping` by absolute type-section index.
/// Correct WITHIN one module (an index is a unique, unambiguous identity
/// there) -- NOT across modules, which need a different, more
/// conservative check (`WasmModule::type_group_shape`).
///
/// A free function taking a bare `&[TypeSubtyping]` slice (rather than a
/// method on `WasmModule`) so callers that only carry a flat, per-type
/// subtyping table -- e.g. `wasm-execution::WasmExecutionContext`, which
/// deliberately doesn't hold a full `WasmModule` (see that struct's own
/// doc comments on `types`/`func_types`) -- can reuse the exact same,
/// security-reviewed walk at a new runtime call site (`call_indirect`,
/// `ref.cast`, `ref.test`) instead of re-implementing it and risking the
/// two copies drifting apart. [`WasmModule::func_type_is_nominal_subtype`]
/// is now a thin wrapper around this.
///
/// Bounded to [`MAX_SUBTYPE_CHAIN_HOPS`] hops so a malformed/cyclic chain
/// can never loop forever -- see that constant's own doc comment for why
/// the bound matters for algorithmic complexity too, not just
/// termination. This is also this function's own answer to the "what if
/// a caller invokes this against an UNVALIDATED module" security
/// question (W33 addendum): `wasm-validator`'s `check_type_subtyping_is_
/// acyclic` runs at module-validation time and rejects a cyclic chain
/// before it can reach any caller that only validates first, but a
/// caller that skips validation entirely (documented as a real
/// possibility for `wasm-execution`'s own test suite, and for any
/// embedder that constructs an engine by hand) gets a SAFE outcome here
/// regardless: the hop cap still terminates, and a chain that loops
/// within the cap simply reports "not a nominal subtype" past the
/// cutoff -- a false negative, never a false positive, and never an
/// unbounded walk.
///
/// W34 third slice (`code/specs/W34-wasm-gc-canonical-type-equivalence.md`):
/// `canonical_types` is a second, parallel slice (typically `ValidatedModule::
/// canonical_types`/`WasmExecutionContext::canonical_types` -- the SAME
/// per-flat-index `Vec<Option<(Rc<CanonicalGroup>, u32)>>` this crate's own
/// `canonicalize_types` produces) that upgrades BOTH the reflexive base
/// case and every hop's own termination check from raw index equality to
/// real canonical equivalence -- exactly the GC proposal's own rule
/// (`code/specs/W34-wasm-gc-canonical-type-equivalence.md`'s "Subtyping
/// across type indices": "`$t <: $t'` iff `$t` and `$t'` define equivalent
/// types [not merely `$t = $t'`], or ... `$t'' <: $t'`... Effectively,
/// this means that subtyping is 'nominal' modulo type canonicalisation.").
/// Pass an empty slice (`&[]`) for a caller that has no canonical data at
/// all (or predates this slice) -- [`canonical_types_equivalent`] always
/// reports `false` for an empty/too-short slice, so this is a strict,
/// zero-behavior-change superset of the old nominal-only rule, never a
/// new false accept.
pub fn nominal_subtype_chain(type_subtyping: &[TypeSubtyping], canonical_types: &[Option<(Rc<CanonicalGroup>, u32)>], sub_idx: u32, super_idx: u32) -> bool {
    if sub_idx == super_idx || canonical_types_equivalent(canonical_types, sub_idx, super_idx) {
        return true;
    }
    let mut cur = sub_idx;
    let at = |idx: u32| type_subtyping.get(idx as usize).copied().unwrap_or_default();
    for _ in 0..MAX_SUBTYPE_CHAIN_HOPS {
        match at(cur).supertype {
            Some(parent) if parent == super_idx || canonical_types_equivalent(canonical_types, parent, super_idx) => return true,
            Some(parent) => cur = parent,
            None => return false,
        }
    }
    false
}

/// Whether SOME type in the ascending nominal `sub` chain starting at
/// `start_idx` -- reflexively including `start_idx` itself -- within ONE
/// module's own `type_subtyping`/`canonical_types` tables is canonically
/// equivalent to an EXTERNAL `target` (W34 fourth slice: `code/specs/
/// W34-wasm-gc-canonical-type-equivalence.md`).
///
/// This is [`nominal_subtype_chain`]'s cross-module counterpart: that
/// function compares two indices INTO THE SAME table (so its raw
/// `sub_idx == super_idx` reflexive check is a valid same-module fast
/// path); this one compares a chain walked in ONE module's table against
/// a target that lives in a DIFFERENT, independently-canonicalized
/// module's own table -- raw index equality is meaningless there (the two
/// modules share no numbering at all), so every hop's termination,
/// including the reflexive base case, is checked by real canonical
/// equivalence only, via [`canonical_type_entries_equivalent`].
///
/// Needed for cross-module linking's own subtyping rule: a WASM func
/// import may be satisfied by an export whose ACTUAL declared type is a
/// nominal subtype (not merely canonically equivalent) of the import's
/// declared type -- MVP.md's own "subtyping is nominal modulo type
/// canonicalisation," applied across the module boundary rather than
/// within one module. A declared `sub $parent` relationship is only ever
/// meaningful within the module that declared it (there is no such thing
/// as a supertype relationship spanning two modules), so this walks
/// EXACTLY ONE module's own chain -- the EXPORTING module's -- checking
/// each ancestor against the IMPORTING module's declared type.
///
/// Same [`MAX_SUBTYPE_CHAIN_HOPS`] bound as [`nominal_subtype_chain`], for
/// the identical reason (a malformed/cyclic chain, or one from a
/// hand-built, never-validated `WasmModule`, must still terminate and
/// stay linear rather than looping or scaling with an attacker-chosen
/// chain length).
pub fn canonical_chain_reaches(type_subtyping: &[TypeSubtyping], canonical_types: &[Option<(Rc<CanonicalGroup>, u32)>], start_idx: u32, target: Option<&(Rc<CanonicalGroup>, u32)>) -> bool {
    let Some(target) = target else {
        return false;
    };
    if canonical_type_entries_equivalent(canonical_types.get(start_idx as usize).and_then(|o| o.as_ref()), Some(target)) {
        return true;
    }
    let mut cur = start_idx;
    let at = |idx: u32| type_subtyping.get(idx as usize).copied().unwrap_or_default();
    for _ in 0..MAX_SUBTYPE_CHAIN_HOPS {
        match at(cur).supertype {
            Some(parent) => {
                if canonical_type_entries_equivalent(canonical_types.get(parent as usize).and_then(|o| o.as_ref()), Some(target)) {
                    return true;
                }
                cur = parent;
            }
            None => return false,
        }
    }
    false
}

/// Whether flat type-section indices `i` and `j` are canonically
/// equivalent (W34: `code/specs/W34-wasm-gc-canonical-type-equivalence.md`),
/// given a `canonical_types` table shaped like [`canonicalize_types`]'s own
/// return value (one `Option<(Rc<CanonicalGroup>, u32)>` per flat index).
/// `false`, conservatively, whenever EITHER side is out of range or wasn't
/// canonicalized (`None`) -- never a wrong `true`. This is the single
/// shared comparison both [`nominal_subtype_chain`] (the chain-walk's own
/// termination check) and `wasm-validator::ValidatedModule::
/// canonically_equivalent` (the public, post-validation accessor) use, so
/// the two can never drift apart.
pub fn canonical_types_equivalent(canonical_types: &[Option<(Rc<CanonicalGroup>, u32)>], i: u32, j: u32) -> bool {
    canonical_type_entries_equivalent(
        canonical_types.get(i as usize).and_then(|o| o.as_ref()),
        canonical_types.get(j as usize).and_then(|o| o.as_ref()),
    )
}

/// The actual comparison [`canonical_types_equivalent`] performs, factored
/// out to a two-`Option`-references shape (W34 fourth slice: `code/specs/
/// W34-wasm-gc-canonical-type-equivalence.md`) so it can also back
/// CROSS-MODULE comparisons, where the two entries being compared live in
/// two entirely separate `canonical_types` tables (one per independently-
/// validated `WasmModule`, no shared numbering at all -- see
/// `wasm-runtime`'s import-compatibility check, the one real caller that
/// needs this two-table shape) rather than two indices into the SAME
/// table. `None` on either side is conservatively `false` -- "not known to
/// be equivalent," never a wrong `true` -- exactly [`canonical_types_equivalent`]'s
/// own existing contract, preserved here verbatim, not weakened by the
/// refactor.
pub fn canonical_type_entries_equivalent(a: Option<&(Rc<CanonicalGroup>, u32)>, b: Option<&(Rc<CanonicalGroup>, u32)>) -> bool {
    match (a, b) {
        // Security review finding (W34 third slice): `Rc::ptr_eq` FIRST,
        // as a fast path -- `canonicalize_types`'s own interning (see that
        // function's doc comment) guarantees two content-identical groups
        // PRODUCED BY THE SAME CALL always share one allocation, so this
        // hits for every within-module comparison after the first (the
        // overwhelmingly common case this function is actually called for
        // per-instruction, per-hop). Falls back to full derived `PartialEq`
        // (a real recursive structural walk, still bounded by `CanonicalCost`'s
        // construction-time caps) only when the two `Rc`s come from
        // different allocations -- e.g. two independently-validated
        // modules' own `canonical_types` tables (the cross-module case
        // this slice wires in via `canonical_type_entries_equivalent`
        // directly, and `canonical_types_equivalent`'s own single-table
        // callers keep hitting the fast path exactly as before, since
        // interning is per-`canonicalize_types`-call and every within-
        // module comparison's two entries necessarily came from the SAME
        // call).
        (Some((a, pa)), Some((b, pb))) => pa == pb && (Rc::ptr_eq(a, b) || a == b),
        _ => false,
    }
}

/// Whether ANY entry in `type_subtyping` declares real GC-proposal nominal
/// info -- a non-default supertype, non-final, or a real (>1-member) `rec`
/// group -- as opposed to every entry being `TypeSubtyping::default()` (no
/// `sub` clause, final, singleton group: the pre-GC/MVP shape every type
/// gets by construction, see `dedup_type`'s own doc comment in `wasm-wast-
/// parser`, which pushes exactly this default for every type it declares,
/// `sub`-declared or not -- so `type_subtyping.is_empty()` is NOT a
/// reliable "this module never uses `sub`" signal, since the vector is
/// fully populated, one entry per type, regardless).
///
/// `wasm-execution`'s runtime dispatch checks (`call_indirect`, `ref.cast`,
/// `ref.test` -- W33 second slice, item 4) use this to decide which of two
/// rules applies: `false` means fall back to the engine's original, pre-W33
/// plain structural-equality check (safe and correct for every module that
/// never uses `sub` -- the overwhelming majority of the conformance
/// corpus); `true` means switch to the real nominal (reflexive index
/// equality OR declared `sub`-chain) check GC-proposal type identity
/// requires, since once ANY type in the module is nominal, coincidental
/// structural equality between two OTHER, unrelated types is no longer a
/// reliable equivalence signal (`type-subtyping.wast` lines 373-401 is the
/// corpus proof: three distinct nominal `(func)`-shaped types related only
/// by a `sub` chain).
pub fn any_declares_subtyping(type_subtyping: &[TypeSubtyping]) -> bool {
    type_subtyping.iter().any(|t| *t != TypeSubtyping::default())
}

impl WasmModule {
    /// Returns `idx`'s own [`TypeSubtyping`] metadata, or the default
    /// (final, no supertype, singleton group) if `idx` is out of range or
    /// this module never populated the vector -- see `TypeSubtyping`'s own
    /// doc comment for why that fallback is always safe rather than a bug
    /// to guard against.
    pub fn type_subtyping_at(&self, idx: u32) -> TypeSubtyping {
        self.type_subtyping.get(idx as usize).copied().unwrap_or_default()
    }

    /// Whether function type `sub_idx` is a reflexive, transitive NOMINAL
    /// subtype of `super_idx`, per each type's own declared `sub $parent`
    /// chain (W33 first slice). Walks the chain by absolute type-section
    /// index -- always correct WITHIN one module (an index is a unique,
    /// unambiguous identity there), unlike cross-module comparisons (see
    /// [`Self::type_group_shape`]'s own doc comment for why those need a
    /// different, more conservative check instead).
    ///
    /// Bounded to at most `types.len()` hops so a malformed/cyclic chain
    /// (which `wasm-validator`'s own module-level check rejects before a
    /// module's function bodies are ever type-checked against it) can
    /// never loop forever.
    pub fn func_type_is_nominal_subtype(&self, sub_idx: u32, super_idx: u32) -> bool {
        // Nominal-only, by design: `WasmModule` never carries `canonical_
        // types` itself (see `canonicalize_types`'s own doc comment for
        // why that lives only on `ValidatedModule`/`WasmExecutionContext`,
        // computed post-validation) -- so this convenience method passes
        // an empty canonical-equivalence table, which `nominal_subtype_
        // chain` guarantees is behaviorally identical to the pre-W34
        // nominal-only rule (see that function's own doc comment). Callers
        // that DO have real canonical data (`wasm-validator::type_check`'s
        // `is_assignable`, `wasm-execution`'s runtime dispatch) call
        // `nominal_subtype_chain` directly instead, passing their own
        // `canonical_types` slice.
        nominal_subtype_chain(&self.type_subtyping, &[], sub_idx, super_idx)
    }

    /// `(rec_group_size, rec_group_position)` for type `idx` -- see
    /// [`TypeSubtyping::rec_group_position`]'s own doc comment for why
    /// this exists and how `wasm-runtime`'s cross-module import/tag
    /// type-compatibility check uses it.
    pub fn type_group_shape(&self, idx: u32) -> (u32, u32) {
        let st = self.type_subtyping_at(idx);
        (st.rec_group_size, st.rec_group_position)
    }

    /// This flat type-section index's real [`TypeKind`], if `type_kinds`
    /// covers it — `None` when `type_kinds` is empty/shorter (a legacy
    /// module; see [`TypeKind`]'s own doc comment), in which case callers
    /// fall back to the pre-W33-fourth-slice offset formulas directly.
    fn type_kind_at(&self, idx: u32) -> Option<TypeKind> {
        self.type_kinds.get(idx as usize).copied()
    }

    /// Resolve flat type-section index `type_idx` to its [`StructType`], if
    /// any — `type_kinds`-aware first (correct for any module built by this
    /// crate's own text-format parser, which may interleave struct/func/array
    /// declarations in arbitrary source order), falling back to the LEGACY
    /// `types.len() + k` offset convention when `type_kinds` doesn't cover
    /// this index at all (the binary decoder's own modules, and any
    /// hand-built `WasmModule` literal that never populates `type_kinds`).
    ///
    /// Returns `None` (never panics or underflows) for an out-of-range
    /// index, or one that names a func/array type instead of a struct.
    pub fn struct_type_at(&self, type_idx: u32) -> Option<&StructType> {
        match self.type_kind_at(type_idx) {
            Some(TypeKind::Struct(k)) => self.struct_types.get(k as usize),
            Some(_) => None,
            None if self.type_kinds.is_empty() => {
                let k = (type_idx as usize).checked_sub(self.types.len())?;
                self.struct_types.get(k)
            }
            None => None,
        }
    }

    /// The array-type analogue of [`Self::struct_type_at`] — see that
    /// method's own doc comment for the `type_kinds`-aware-first, legacy-
    /// offset-fallback strategy. The legacy offset accounts for
    /// `struct_types` too (arrays are conventionally encoded after every
    /// func type AND every struct type), matching [`StructType`]'s own doc
    /// comment on where structs sit relative to `types`.
    pub fn array_type_at(&self, type_idx: u32) -> Option<&ArrayType> {
        match self.type_kind_at(type_idx) {
            Some(TypeKind::Array(k)) => self.array_types.get(k as usize),
            Some(_) => None,
            None if self.type_kinds.is_empty() => {
                let k = (type_idx as usize).checked_sub(self.types.len() + self.struct_types.len())?;
                self.array_types.get(k)
            }
            None => None,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// W34 first slice: canonical type-group equivalence -- singleton groups only
// ──────────────────────────────────────────────────────────────────────────────
//
// `code/specs/W34-wasm-gc-canonical-type-equivalence.md` (grounded in the
// real WasmGC proposal's `MVP.md` and the reference interpreter's actual
// `interpreter/syntax/types.ml`/`interpreter/valid/match.ml`) is the real
// canonicalization algorithm: recognizing two separately-declared `(rec
// ...)` groups as "the same type" whenever their SHAPES match, even across
// modules that share no numbering at all. This slice implements exactly the
// narrowest non-trivial piece of it -- `rec_group_size == 1` groups only
// (every plain, non-`rec`-wrapped `(type ...)` field, AND every explicit
// `(rec (type ...))` with exactly one member) -- proving the De Bruijn
// "rolling"/`Rec` marker mechanism (a self-reference becomes `Rec(0)`, not
// an absolute index) and the cross-module comparability property it exists
// for, before attempting real multi-member De Bruijn numbering (deferred to
// a later slice; see the spec's own "Recommended slice decomposition").

/// A self-contained, De-Bruijn-tied value tree for one `rec` group's worth
/// of composite types -- comparable via ordinary structural equality
/// (`PartialEq`/`Eq`/`Hash` all derived, exactly like OCaml's polymorphic
/// `=` in the reference interpreter's own `match_def_type`) across TWO
/// DIFFERENT [`WasmModule`]s with no shared numbering at all, per the real
/// WasmGC proposal's canonicalization algorithm (MVP.md's own Note 2:
/// "type equivalence checks can be implemented in constant-time by
/// representing all types as trees in tied form and canonicalising them
/// bottom-up in linear time upfront").
///
/// The W34 first slice only ever constructed a `CanonicalGroup` with
/// exactly one member (`rec_group_size == 1` groups only). The W34 second
/// slice lifted that restriction with real multi-member De Bruijn
/// numbering -- `members` now holds one [`CanonicalSubtype`] per real
/// `rec`-group member, in declaration order, without this type itself
/// needing to change shape at all: this mirrors `interpreter/valid/
/// valid.ml`'s `check_rec_type`, which builds every member of a group
/// together in one call.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalGroup {
    pub members: Vec<CanonicalSubtype>,
}

/// One member of a [`CanonicalGroup`] -- the tied form of a single `(sub
/// final? $parent? (comptype))` declaration (MVP.md, `#### Equivalence`:
/// "two subtypes are equivalent if their structure is equivalent, they
/// have equivalent supertypes, and their finality flag matches").
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalSubtype {
    pub is_final: bool,
    /// The declared `sub $parent` supertype, tied the same way the body
    /// is: [`CanonicalHeapRef::Rec`] for a reference to ANY member of the
    /// SAME group being tied (not just this member itself -- a supertype
    /// naming an earlier sibling within a multi-member group is ordinary
    /// and, unlike a literal self-supertype, not rejected by anything
    /// upstream), [`CanonicalHeapRef::Outer`] for a reference to an
    /// earlier, already-canonicalized group. `None` for no declared
    /// supertype.
    pub supertype: Option<CanonicalHeapRef>,
    pub comp: CanonicalCompType,
}

/// The tied form of a `comptype` body -- mirrors [`FuncType`]/
/// [`StructType`]/[`ArrayType`], but every concrete/self/group reference
/// inside has been resolved to a [`CanonicalHeapRef`] instead of a raw flat
/// type-section index.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CanonicalCompType {
    Func(Vec<CanonicalValType>, Vec<CanonicalValType>),
    Struct(Vec<CanonicalFieldType>),
    Array(CanonicalFieldType),
}

/// Mirrors [`FieldType`], with `storage`'s own index (if any) tied.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalFieldType {
    pub storage: CanonicalStorageType,
    pub mutable: bool,
}

/// Mirrors [`StorageType`] -- `I8`/`I16` carry no index to tie, so only the
/// `Val` arm's inner [`CanonicalValType`] differs from its untied source.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CanonicalStorageType {
    Val(CanonicalValType),
    I8,
    I16,
}

/// Mirrors [`ValueType`], with every concrete/self/group reference resolved
/// to a [`CanonicalHeapRef`] and every abstract heap type folded into
/// [`AbstractHeapKind`] -- so two [`ValueType`] values that spell the same
/// real type differently (e.g. `Anyref` is always exactly one shape, but a
/// concrete `StructRef(3)` in one module and `StructRef(9)` in an unrelated
/// one can still tie to the identical `CanonicalValType`) compare equal
/// once tied.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CanonicalValType {
    I32,
    I64,
    F32,
    F64,
    V128,
    /// `true` = nullable (`StructRef`-shaped source variants); `false` =
    /// non-null (`NonNullStructRef`-shaped ones) -- see
    /// [`canonicalize_value_type`]'s own exhaustive match for the full
    /// per-[`ValueType`]-variant mapping.
    Ref(bool, CanonicalHeapRef),
}

/// The abstract (non-index-carrying) WasmGC heap-type kinds this crate's
/// [`ValueType`] can express. `Eq` and `Struct` (the abstract top of the
/// struct hierarchy) have no corresponding [`ValueType`] variant in this
/// crate today (no bare `eqref`/`(ref struct)` support yet) but are
/// included for the same "model the real GC proposal's full lattice, not
/// just what's reachable today" reason [`ValueType::NonNullArrayAny`]'s own
/// doc comment gives for the `array` top type it already models -- adding
/// them now costs nothing and avoids a second enum-widening pass later.
/// `Exn`/`NoExn` are NOT in the WasmGC proposal's own MVP.md lattice at all
/// (they're the separate exception-handling proposal's own heap types,
/// W24: `code/specs/W24-wasm-exceptions-exnref-catch-ref.md`) -- included
/// here because this crate's own [`ValueType::Exnref`]/[`ValueType::
/// NullExnref`] already exist and a canonicalizer that panics or silently
/// mismodels them on first contact would be a real, not merely
/// theoretical, gap (this spec's own design section's `AbstractHeapKind`
/// sketch predates `Exnref` being re-checked against the current code and
/// listed only the ten proposal-native kinds; the addendum records this
/// as the one place re-verification found the design section itself needed
/// correcting, not just re-confirming).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AbstractHeapKind {
    Any,
    Eq,
    I31,
    Struct,
    Array,
    Func,
    None,
    Extern,
    NoExtern,
    NoFunc,
    Exn,
    NoExn,
}

/// A resolved heap-type reference within a tied [`CanonicalGroup`] --
/// either an abstract kind, a De Bruijn self/in-group reference, or a
/// wholesale-embedded earlier group.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CanonicalHeapRef {
    Abstract(AbstractHeapKind),
    /// A reference within the SAME group being tied (MVP.md's "rolling"/
    /// "tying" -- `interpreter/syntax/types.ml`'s `roll_rec_type`): `i` is
    /// the target member's own position within the group (`0` for a
    /// singleton's self-reference; `0..N-1` for any member of a real
    /// `N`-member group, per the W34 second slice's group-relative
    /// numbering -- `roll_rec_type`'s own `Int32.sub x' x`, group-start-
    /// relative, not the module's absolute type-section index).
    Rec(u32),
    /// A reference to an EARLIER group, embedded wholesale (already fully
    /// tied/closed when it was computed -- `match.ml`'s `subst_of`: "embed
    /// that earlier group's already-rolled `def_type` value wholesale, one
    /// level, and stops"), plus this reference's own position within that
    /// group (the referenced member's position within its OWN group --
    /// `0` for a reference to a singleton group, `0..N-1` for a reference
    /// into a real `N`-member earlier group).
    ///
    /// `Rc`, not the design sketch's `Box` -- `Rc` (not an owned clone) is
    /// the right choice at every embed site, not just the top-level
    /// per-index one (`wasm-validator::ValidatedModule`'s own cache, added
    /// alongside this crate by this same slice): a `Box` here would
    /// deep-clone the entire referenced group's tree every time it's
    /// embedded, so a module with
    /// several singleton groups each referencing the SAME earlier one
    /// (`type-rec.wast`'s "Static matching" module references `$f1`/`$f2`
    /// several times each) would duplicate that shared subtree once per
    /// reference; `Rc` shares the one already-computed allocation instead,
    /// while `derive(PartialEq, Eq, Hash)` still compares/hashes through
    /// to the pointee's CONTENTS (never the pointer), so equivalence
    /// across two independently-`Rc`-allocated but structurally-identical
    /// groups (the whole point of a cross-module comparison) is
    /// unaffected by this choice.
    Outer(Rc<CanonicalGroup>, u32),
}

/// One flat type-section index's real composite-type payload, resolved the
/// same `type_kinds`-aware-first, legacy-offset-fallback way
/// [`WasmModule::struct_type_at`]/[`WasmModule::array_type_at`] already do
/// -- a small local enum so [`canonicalize_types`] can build a
/// [`CanonicalCompType`] from whichever of `types`/`struct_types`/
/// `array_types` actually holds this index's data, without three separate
/// near-identical call sites.
enum CompTypeRef<'a> {
    Func(&'a FuncType),
    Struct(&'a StructType),
    Array(&'a ArrayType),
}

fn comp_type_at(module: &WasmModule, idx: u32) -> Option<CompTypeRef<'_>> {
    match module.type_kind_at(idx) {
        Some(TypeKind::Func) => module.types.get(idx as usize).map(CompTypeRef::Func),
        Some(TypeKind::Struct(_)) => module.struct_type_at(idx).map(CompTypeRef::Struct),
        Some(TypeKind::Array(_)) => module.array_type_at(idx).map(CompTypeRef::Array),
        None if module.type_kinds.is_empty() => {
            let types_len = module.types.len();
            let struct_len = module.struct_types.len();
            if (idx as usize) < types_len {
                module.types.get(idx as usize).map(CompTypeRef::Func)
            } else if (idx as usize) < types_len + struct_len {
                module.struct_type_at(idx).map(CompTypeRef::Struct)
            } else {
                module.array_type_at(idx).map(CompTypeRef::Array)
            }
        }
        None => None,
    }
}

/// The total number of flat type-section indices this module declares --
/// `types.len()` when `type_kinds` covers the whole type section (every
/// struct/array-kind entry already occupies a dummy `FuncType` slot in
/// `types` too, per [`TypeKind`]'s own doc comment, so `types.len()` IS the
/// true flat count in that case), or the legacy `types.len() +
/// struct_types.len() + array_types.len()` sum when `type_kinds` is empty
/// (the binary decoder's convention, and every hand-built `WasmModule`
/// literal that predates `type_kinds`).
fn total_type_count(module: &WasmModule) -> usize {
    if module.type_kinds.is_empty() {
        module.types.len() + module.struct_types.len() + module.array_types.len()
    } else {
        module.types.len()
    }
}

/// Bookkeeping threaded alongside every value [`resolve_heap_index`] and
/// friends build, so [`canonicalize_types`] can reject a tree before it
/// becomes dangerous to the compiler-derived `Drop`/`PartialEq`/`Hash`
/// traversals `CanonicalGroup` and friends need for correctness
/// (structural, not pointer, comparison is the whole point of canonical
/// equivalence). Two DIFFERENT costs, because they bound two DIFFERENT
/// resources:
///
/// - `depth`: how many `Outer`-embedding hops deep the LONGEST single
///   reference chain reaches. Bounds STACK depth -- a long chain of
///   singleton (or now, W34 second slice, multi-member) groups, each
///   referencing only the one immediately before it, makes those derived
///   traversals recurse `depth` frames deep (see [`MAX_CANONICAL_OUTER_
///   DEPTH`]'s own doc comment for the W34 first slice's empirical
///   stack-overflow finding this closed).
/// - `weight`: how many nodes a FULLY-UNSHARED expansion of this tree
///   would contain. Bounds TOTAL WORK -- a reference chain that also
///   BRANCHES (several sibling positions, or several members of the same
///   `rec` group, all embedding the same earlier group) MULTIPLIES,
///   rather than adds, the node count a full structural comparison must
///   visit at each level, even though `Rc` sharing keeps actual memory
///   linear (see [`MAX_CANONICAL_TREE_WEIGHT`]'s own doc comment for a
///   worked numeric example -- `depth` alone cannot catch this, because
///   branching leaves the LONGEST single chain short even as the total
///   node count a naive recursive comparison visits explodes).
#[derive(Debug, Clone, Copy)]
struct CanonicalCost {
    depth: u32,
    weight: u64,
}

impl CanonicalCost {
    /// The cost of a single leaf node that embeds nothing further -- a
    /// scalar `ValueType`, an `I8`/`I16` storage type, or a `Rec` marker
    /// (which never embeds another group, unlike `Outer`).
    const LEAF: CanonicalCost = CanonicalCost { depth: 0, weight: 1 };
    /// The cost of "nothing yet" -- the starting accumulator for a sum
    /// over a member's params/results/fields, or a `None`-placeholder
    /// slot's cost (never actually read in that case, since a `None`
    /// entry's cost can never be looked up by [`resolve_heap_index`]
    /// without that lookup itself already having failed first).
    const ZERO: CanonicalCost = CanonicalCost { depth: 0, weight: 0 };

    /// Whether this cost is still within both caps -- checked as soon as
    /// possible after every partial sum, not only once at the very end, so
    /// a pathological module is rejected before its (never fully built)
    /// tree could grow any larger.
    fn within_caps(self) -> bool {
        self.depth <= MAX_CANONICAL_OUTER_DEPTH && self.weight <= MAX_CANONICAL_TREE_WEIGHT
    }

    /// Combines the cost of two SIBLING pieces of one member's own body
    /// (two params, two fields, a supertype reference alongside the body,
    /// ...): `depth` is the max of the two (a stack only ever recurses
    /// down ONE of them at a time), `weight` is their SUM (a full
    /// structural comparison visits BOTH, so their total node counts add
    /// -- and, transitively, MULTIPLY across levels when the same
    /// weight-heavy group is referenced from more than one sibling
    /// position, which is exactly the blowup this cost exists to catch).
    /// `saturating_add`, not `+`, purely as defense in depth: every input
    /// is already capped at [`MAX_CANONICAL_TREE_WEIGHT`] before it can be
    /// combined into anything else, so an actual overflow is not
    /// reachable even from an implausible module, but a saturating sum
    /// can never panic regardless.
    fn combine_sum(self, other: CanonicalCost) -> CanonicalCost {
        CanonicalCost { depth: self.depth.max(other.depth), weight: self.weight.saturating_add(other.weight) }
    }
}

/// Resolves a single reference (by flat type-section index) into a
/// [`CanonicalHeapRef`], given the group currently being tied (`[group_
/// start, group_end)`, a half-open range of flat indices -- `group_end -
/// group_start` is that group's own member count, `1` for a singleton)
/// and every EARLIER group's already-computed canonical form
/// (`canonical_so_far`, indexed the same way the final result of
/// [`canonicalize_types`] is).
///
/// - `group_start <= target_idx < group_end` -- a reference to ANY member
///   of the SAME group being tied (W34 second slice's real De Bruijn
///   numbering, `interpreter/syntax/types.ml`'s `roll_rec_type`: `Int32.
///   sub x' x`, group-start-relative) -- ties to `Rec(target_idx -
///   group_start)`. This subsumes the W34 first slice's singleton
///   self-reference case exactly (`group_start == target_idx ==
///   group_end - 1` reduces to `Rec(0)`).
/// - `target_idx < group_start` -- a reference to an EARLIER, already-
///   canonicalized group. `canonical_so_far[target_idx]` must already
///   hold a computed `Some((group, position))` (guaranteed for any
///   in-range, earlier group, since [`canonicalize_types`] processes
///   groups in strictly increasing flat-index order and a validated
///   module's own `sub`/`rec` forward-reference rule -- already enforced
///   by `wasm-wast-parser`/`wasm-validator` before this ever runs -- means
///   a type can only reference an index `< group_end`) -- embeds that
///   group wholesale via [`CanonicalHeapRef::Outer`], provided doing so
///   would not push `depth` past [`MAX_CANONICAL_OUTER_DEPTH`] or
///   `weight` past [`MAX_CANONICAL_TREE_WEIGHT`] (see [`CanonicalCost`]'s
///   own doc comment for what each bounds).
/// - `target_idx >= group_end` -- a forward reference, either into this
///   group's own not-yet-fully-declared tail (impossible for a
///   syntactically real `rec` group, whose members are exactly `[group_
///   start, group_end)`, but not impossible for a hand-built, unvalidated
///   `WasmModule`) or into a later group entirely -- never valid (WASM's
///   own ordering rule) -- returns `None`.
/// - Anything else (out of range entirely, or a reference into an earlier
///   group that itself failed to canonicalize) also returns `None` -- this
///   type's own canonical form is therefore ALSO `None` (see
///   [`canonicalize_types`]'s use of this), never a wrong or partial
///   value: a caller sees "not yet canonicalized," never a silently-
///   incomplete tree. This is also this function's whole answer to "what
///   if a caller hands in indices that don't actually satisfy the
///   ordering invariant" (an unvalidated, hand-built `WasmModule`, say,
///   with a forward or out-of-bounds reference): there is no recursion
///   here at all -- only a single array index into ALREADY-COMPUTED
///   entries -- so a malformed index can only ever produce `None` (an
///   honest "can't canonicalize this"), never a panic, an infinite loop,
///   or a stack overflow.
fn resolve_heap_index(
    target_idx: u32,
    group_start: u32,
    group_end: u32,
    canonical_so_far: &[Option<(Rc<CanonicalGroup>, u32)>],
    costs: &[CanonicalCost],
) -> Option<(CanonicalHeapRef, CanonicalCost)> {
    if target_idx >= group_start && target_idx < group_end {
        return Some((CanonicalHeapRef::Rec(target_idx - group_start), CanonicalCost::LEAF));
    }
    if target_idx >= group_end {
        return None;
    }
    let (group, position) = canonical_so_far.get(target_idx as usize)?.as_ref()?;
    let target_cost = costs.get(target_idx as usize).copied().unwrap_or(CanonicalCost::LEAF);
    let cost = CanonicalCost { depth: target_cost.depth + 1, weight: target_cost.weight.saturating_add(1) };
    if !cost.within_caps() {
        return None;
    }
    Some((CanonicalHeapRef::Outer(Rc::clone(group), *position), cost))
}

/// The hop cap [`resolve_heap_index`] enforces on `depth` -- see
/// [`CanonicalCost`]'s own doc comment for the STACK-depth concern this
/// bounds (distinct from [`MAX_CANONICAL_TREE_WEIGHT`]'s total-work
/// concern), and this crate's own pre-existing [`MAX_SUBTYPE_CHAIN_HOPS`]
/// for the established "1,000 hops is this codebase's own accepted safe
/// magnitude for a chain-shaped bound" precedent this mirrors.
const MAX_CANONICAL_OUTER_DEPTH: u32 = 1_000;

/// The total-node-count cap [`resolve_heap_index`] enforces on `weight` --
/// see [`CanonicalCost`]'s own doc comment for the branching-multiplication
/// finding this defends against (W34 second slice: real multi-member `rec`
/// groups make it far more natural for one group to reference an earlier
/// one from SEVERAL sibling positions at once than the first slice's
/// singleton-only groups ever could). A chain of `L` groups, each
/// referencing the one immediately before it from exactly TWO sibling
/// positions (e.g. `type[i] = func(param (ref i-1) (ref i-1))`), has
/// `depth == L` (bounded fine by [`MAX_CANONICAL_OUTER_DEPTH`]'s 1,000-hop
/// cap) but `weight` DOUBLING at every level -- `2^L`, which exceeds even
/// a generous cap by `L` in the low tens, long before `depth`'s own cap
/// would ever engage. `1_000_000` is generous relative to anything this
/// crate's own real corpus needs (every vendored `rec` group is a handful
/// of members referencing a handful of earlier groups) while still small
/// enough that even a maximally adversarial doubling chain cannot reach
/// more than ~20 levels before hitting it, keeping a worst-case rejected
/// canonicalization itself cheap to detect.
const MAX_CANONICAL_TREE_WEIGHT: u64 = 1_000_000;

/// Ties one [`ValueType`] -- the full, exhaustive per-variant mapping this
/// slice's `CanonicalValType`/`CanonicalHeapRef`/`AbstractHeapKind` design
/// exists for. `None` propagates a [`resolve_heap_index`] failure (an
/// unresolvable concrete reference, or either cap) up to the caller
/// unchanged. Returns the tied value alongside its own [`CanonicalCost`]
/// (`CanonicalCost::LEAF` for every scalar/abstract variant and for
/// `Rec`, since neither embeds another group).
fn canonicalize_value_type(
    vt: ValueType,
    group_start: u32,
    group_end: u32,
    canonical_so_far: &[Option<(Rc<CanonicalGroup>, u32)>],
    costs: &[CanonicalCost],
) -> Option<(CanonicalValType, CanonicalCost)> {
    use AbstractHeapKind as A;
    use CanonicalHeapRef::Abstract;
    let resolve = |i: u32| resolve_heap_index(i, group_start, group_end, canonical_so_far, costs);
    Some(match vt {
        ValueType::I32 => (CanonicalValType::I32, CanonicalCost::LEAF),
        ValueType::I64 => (CanonicalValType::I64, CanonicalCost::LEAF),
        ValueType::F32 => (CanonicalValType::F32, CanonicalCost::LEAF),
        ValueType::F64 => (CanonicalValType::F64, CanonicalCost::LEAF),
        ValueType::V128 => (CanonicalValType::V128, CanonicalCost::LEAF),
        ValueType::Anyref => (CanonicalValType::Ref(true, Abstract(A::Any)), CanonicalCost::LEAF),
        // Non-null in this crate -- see `ValueType::I31ref`'s own doc
        // comment ("(ref i31)", not "(ref null i31)").
        ValueType::I31ref => (CanonicalValType::Ref(false, Abstract(A::I31)), CanonicalCost::LEAF),
        ValueType::Funcref => (CanonicalValType::Ref(true, Abstract(A::Func)), CanonicalCost::LEAF),
        ValueType::Externref => (CanonicalValType::Ref(true, Abstract(A::Extern)), CanonicalCost::LEAF),
        ValueType::Exnref => (CanonicalValType::Ref(true, Abstract(A::Exn)), CanonicalCost::LEAF),
        ValueType::NullFuncref => (CanonicalValType::Ref(true, Abstract(A::NoFunc)), CanonicalCost::LEAF),
        ValueType::NullExternref => (CanonicalValType::Ref(true, Abstract(A::NoExtern)), CanonicalCost::LEAF),
        ValueType::NullExnref => (CanonicalValType::Ref(true, Abstract(A::NoExn)), CanonicalCost::LEAF),
        ValueType::NullRef => (CanonicalValType::Ref(true, Abstract(A::None)), CanonicalCost::LEAF),
        ValueType::NonNullArrayAny => (CanonicalValType::Ref(false, Abstract(A::Array)), CanonicalCost::LEAF),
        ValueType::StructRef(i) => {
            let (r, c) = resolve(i)?;
            (CanonicalValType::Ref(true, r), c)
        }
        ValueType::ConcreteFuncRef(i) => {
            let (r, c) = resolve(i)?;
            (CanonicalValType::Ref(true, r), c)
        }
        ValueType::ArrayRef(i) => {
            let (r, c) = resolve(i)?;
            (CanonicalValType::Ref(true, r), c)
        }
        ValueType::NonNullStructRef(i) => {
            let (r, c) = resolve(i)?;
            (CanonicalValType::Ref(false, r), c)
        }
        ValueType::NonNullConcreteFuncRef(i) => {
            let (r, c) = resolve(i)?;
            (CanonicalValType::Ref(false, r), c)
        }
        ValueType::NonNullArrayRef(i) => {
            let (r, c) = resolve(i)?;
            (CanonicalValType::Ref(false, r), c)
        }
    })
}

fn canonicalize_field_type(
    f: FieldType,
    group_start: u32,
    group_end: u32,
    canonical_so_far: &[Option<(Rc<CanonicalGroup>, u32)>],
    costs: &[CanonicalCost],
) -> Option<(CanonicalFieldType, CanonicalCost)> {
    let (storage, cost) = match f.storage {
        StorageType::I8 => (CanonicalStorageType::I8, CanonicalCost::LEAF),
        StorageType::I16 => (CanonicalStorageType::I16, CanonicalCost::LEAF),
        StorageType::Val(vt) => {
            let (cvt, c) = canonicalize_value_type(vt, group_start, group_end, canonical_so_far, costs)?;
            (CanonicalStorageType::Val(cvt), c)
        }
    };
    Some((CanonicalFieldType { storage, mutable: f.mutable }, cost))
}

fn canonicalize_comp_type(
    module: &WasmModule,
    idx: u32,
    group_start: u32,
    group_end: u32,
    canonical_so_far: &[Option<(Rc<CanonicalGroup>, u32)>],
    costs: &[CanonicalCost],
) -> Option<(CanonicalCompType, CanonicalCost)> {
    match comp_type_at(module, idx)? {
        CompTypeRef::Func(ft) => {
            let mut cost = CanonicalCost::ZERO;
            let params = ft
                .params
                .iter()
                .map(|vt| {
                    let (cvt, c) = canonicalize_value_type(*vt, group_start, group_end, canonical_so_far, costs)?;
                    cost = cost.combine_sum(c);
                    if !cost.within_caps() {
                        return None;
                    }
                    Some(cvt)
                })
                .collect::<Option<Vec<_>>>()?;
            let results = ft
                .results
                .iter()
                .map(|vt| {
                    let (cvt, c) = canonicalize_value_type(*vt, group_start, group_end, canonical_so_far, costs)?;
                    cost = cost.combine_sum(c);
                    if !cost.within_caps() {
                        return None;
                    }
                    Some(cvt)
                })
                .collect::<Option<Vec<_>>>()?;
            Some((CanonicalCompType::Func(params, results), cost))
        }
        CompTypeRef::Struct(st) => {
            let mut cost = CanonicalCost::ZERO;
            let fields = st
                .fields
                .iter()
                .map(|f| {
                    let (cf, c) = canonicalize_field_type(*f, group_start, group_end, canonical_so_far, costs)?;
                    cost = cost.combine_sum(c);
                    if !cost.within_caps() {
                        return None;
                    }
                    Some(cf)
                })
                .collect::<Option<Vec<_>>>()?;
            Some((CanonicalCompType::Struct(fields), cost))
        }
        CompTypeRef::Array(at) => {
            let (field, cost) = canonicalize_field_type(at.element, group_start, group_end, canonical_so_far, costs)?;
            Some((CanonicalCompType::Array(field), cost))
        }
    }
}

/// Builds the [`CanonicalSubtype`] for ONE member of the group currently
/// being tied (`member_idx`, somewhere in `[group_start, group_end)`), or
/// `None` if any reference inside it can't yet be resolved or either cost
/// cap would be exceeded (see [`resolve_heap_index`]'s own doc comment).
/// Returns the subtype alongside its own [`CanonicalCost`], for
/// [`canonicalize_types`] to fold into the whole GROUP's own cost (a full
/// structural traversal of the group visits every member, so the group's
/// total cost is the SUM of its members' costs, not just one of them).
///
/// This is the one function the W34 second slice's real multi-member
/// numbering actually needed to change the SHAPE of, versus the first
/// slice's `build_singleton_canonical`: it now takes the group's `(group_
/// start, group_end)` bounds as an explicit parameter (rather than
/// assuming `group_end == group_start + 1`), and is called once per
/// member of a real `rec` group, not once per (always-singleton) group.
/// Every other helper it calls (`resolve_heap_index`, `canonicalize_
/// value_type`, `canonicalize_field_type`, `canonicalize_comp_type`) is
/// reused UNCHANGED in shape from the first slice, per that slice's own
/// addendum note that only the numbering itself, not these helpers,
/// needed to grow group-awareness.
fn build_member_canonical(
    module: &WasmModule,
    member_idx: u32,
    group_start: u32,
    group_end: u32,
    canonical_so_far: &[Option<(Rc<CanonicalGroup>, u32)>],
    costs: &[CanonicalCost],
) -> Option<(CanonicalSubtype, CanonicalCost)> {
    let ts = module.type_subtyping_at(member_idx);
    let mut cost = CanonicalCost::ZERO;
    let supertype = match ts.supertype {
        Some(sup_idx) => {
            let (r, c) = resolve_heap_index(sup_idx, group_start, group_end, canonical_so_far, costs)?;
            cost = cost.combine_sum(c);
            if !cost.within_caps() {
                return None;
            }
            Some(r)
        }
        None => None,
    };
    let (comp, comp_cost) = canonicalize_comp_type(module, member_idx, group_start, group_end, canonical_so_far, costs)?;
    cost = cost.combine_sum(comp_cost);
    if !cost.within_caps() {
        return None;
    }
    Some((CanonicalSubtype { is_final: ts.is_final, supertype, comp }, cost))
}

/// Whether the `rec` group claimed to start at flat index `group_start`
/// (with member count `size`, read from `group_start`'s own [`TypeSubtyping`])
/// is internally CONSISTENT -- every one of its `size` claimed members
/// actually exists in range and agrees with the group's own claimed shape
/// (`rec_group_size == size`, `rec_group_position` matching its own offset
/// from `group_start`). This is the defensive check a hand-built,
/// never-validated `WasmModule` needs (a real, `wasm-wast-parser`-produced
/// module's own `rec`-group metadata is always internally consistent by
/// construction, but [`canonicalize_types`] must never assume that): a
/// module claiming an inconsistent shape is simply unresolvable at
/// `group_start`, not a license to guess.
fn group_bounds_are_consistent(module: &WasmModule, group_start: u32, size: u32, total: usize) -> bool {
    size >= 1
        && (group_start as u64).saturating_add(size as u64) <= total as u64
        && (0..size).all(|offset| {
            let member = module.type_subtyping_at(group_start + offset);
            member.rec_group_size == size && member.rec_group_position == offset
        })
}

/// Computes this module's own canonical type-group forms. One entry per
/// flat type-section index (`total_type_count` long); `None` at any index
/// whose own group's metadata is internally inconsistent (see
/// [`group_bounds_are_consistent`]), or whose body/supertype (or ANY
/// sibling member's, in a real multi-member group -- see below) couldn't
/// be resolved (see [`resolve_heap_index`]).
///
/// Processes GROUPS (a contiguous range of flat indices sharing one
/// `rec_group_size`/`rec_group_position` shape -- a size-1 range for a
/// singleton) in strictly increasing flat-index order, and -- critically
/// for both correctness and termination -- NEVER recurses into an earlier
/// group's own computation while computing a later one: each group's
/// canonical form is built by looking up already-finished entries in
/// `out` (the reference interpreter's own incremental, group-ordered
/// design -- `interpreter/valid/valid.ml`'s `check_rec_type`, called once
/// per group with the running context so far). A reference to ANY member
/// of the group currently being built (not just to `member_idx` itself)
/// ties to `Rec(i)` (checked BEFORE the "look up an earlier entry" path in
/// `resolve_heap_index`, so it never even attempts to index `out` at a
/// position within the group not-yet-pushed). There is therefore no
/// recursive descent of any kind in this function or anything it calls --
/// no cyclic or self/group-referential type structure can make this loop,
/// panic, or overflow the stack, regardless of whether the module was
/// ever validated (see [`resolve_heap_index`]'s own doc comment for the
/// full argument).
///
/// A real `rec` group's `Rc<CanonicalGroup>` is built ONCE, containing
/// EVERY member's [`CanonicalSubtype`] together (the group is a single
/// tied unit, per MVP.md's own `tie($t) = tie_$t(<ctxtype>)`), and shared
/// via `Rc::clone` across every one of that group's `size` flat indices --
/// only the `u32` position half of each index's `(Rc<CanonicalGroup>,
/// u32)` entry differs between sibling members. If ANY member of a group
/// fails to canonicalize (an unresolvable reference, or either
/// [`CanonicalCost`] cap), the WHOLE group's every member becomes `None`
/// -- never a partial group with some members present and others missing,
/// which would let a later `Outer` embed of that "group" silently omit
/// the failed member's own tied form.
///
/// The natural, non-disruptive caching point for this is `wasm-validator`'s
/// `ValidatedModule` (see that crate's own `validate()`, called right after
/// `check_type_subtyping_is_acyclic` succeeds -- canonicalization's
/// termination argument above already assumes references only ever point
/// at an earlier-or-same group, exactly what that acyclicity/ordering
/// check establishes) -- NOT a field on `WasmModule` itself, so an
/// unvalidated module can never carry a stale or attacker-supplied
/// `canonical_types` value.
pub fn canonicalize_types(module: &WasmModule) -> Vec<Option<(Rc<CanonicalGroup>, u32)>> {
    let n = total_type_count(module);
    let mut out: Vec<Option<(Rc<CanonicalGroup>, u32)>> = Vec::with_capacity(n);
    // W34 third-slice security-review finding, fixed proactively: two
    // SEPARATELY-declared groups with byte-identical tied content (no
    // `Outer`/`Rec` relationship between them at all -- the exact
    // cross-module-comparability case this whole mechanism exists for,
    // now also reachable WITHIN one module once `is_assignable`/`call_
    // indirect_type_matches` consult canonical equivalence per instruction,
    // W34 third slice) used to get their own SEPARATE `Rc::new` allocation
    // here, even when identical -- making every later `canonical_types_
    // equivalent(a, b)` call pay derived `PartialEq`'s FULL recursive
    // structural walk (bounded per-call by `CanonicalCost`'s own caps, but
    // NOT bounded across the many times the SAME pair gets compared: once
    // per instruction that flows a value between them). A crafted module
    // with two near-`MAX_CANONICAL_TREE_WEIGHT`-sized identical groups,
    // referenced from a long function body's repeated `local.get`/`local.
    // set` between two locals of those two types, reproducibly took over a
    // minute to validate in a security-review sub-agent's own measured
    // reproduction (empty-cache costs 505µs; ~62s with the same body size
    // once `is_assignable` reaches the heavy comparison every instruction)
    // -- a real, ~100,000x algorithmic-complexity DoS, not a theoretical
    // one. `interned` deduplicates: the FIRST time a given tied shape is
    // built in this call, it's inserted; every LATER group with the
    // IDENTICAL shape reuses that SAME `Rc` allocation (`HashSet<Rc<
    // CanonicalGroup>>`'s `get` looks up by borrowed `&CanonicalGroup`
    // content via `Rc<T>: Borrow<T>`, so this never needs a redundant
    // clone of the candidate just to query the set). This turns
    // `Rc::ptr_eq` into a SOUND, ALWAYS-HITS-WHEN-EQUAL fast path for
    // `canonical_types_equivalent` to try first, for every pair this
    // function itself produced (same call, i.e. exactly the within-module
    // case W34's third slice wires) -- collapsing a repeated O(this
    // group's own weight) walk into O(1) after the first comparison,
    // matching MVP.md's own Note 2 ("canonicalising them bottom-up in
    // linear time upfront" for construction, "constant-time" for
    // comparison after) precisely instead of only in spirit. Interning
    // costs at most one extra `Hash`+lookup per group -- the SAME order of
    // work `canonicalize_types` already pays to BUILD that group's value in
    // the first place, so this adds a constant factor, not a new
    // algorithmic-complexity class, and every existing `CanonicalCost` cap
    // still bounds it exactly as before. Cross-module comparison (two
    // SEPARATE `canonicalize_types` calls, e.g. two different modules'
    // `ValidatedModule`s) still falls back to the full structural `==` --
    // this cache is local to one call, deliberately not a global/thread-
    // shared interner (which would need synchronization and unbounded
    // process-lifetime memory for no benefit this slice's own reachable
    // call sites need); revisit if slice 4's cross-module wiring measures
    // a real need.
    let mut interned: HashSet<Rc<CanonicalGroup>> = HashSet::new();
    // Parallel to `out`: `costs[idx]` is `out[idx]`'s own group's total
    // `CanonicalCost` (the SAME value repeated for every member index of
    // one group -- an `Outer` reference to ANY member embeds the WHOLE
    // group, so the relevant cost for a later reference is the group's
    // total, not one member's own share of it). Never read for a `None`
    // entry, since `resolve_heap_index` already rejects an unresolvable
    // target before it would consult this table. See `CanonicalCost`'s
    // own doc comment for why both dimensions of this bound exist (real,
    // security-review-confirmed findings in the derived `Drop`/
    // `PartialEq`/`Hash` traversals an unbounded tree would otherwise
    // let through).
    let mut costs: Vec<CanonicalCost> = Vec::with_capacity(n);
    let mut idx: u32 = 0;
    while (idx as usize) < n {
        let group_start = idx;
        let size = module.type_subtyping_at(group_start).rec_group_size;
        let is_group_start = module.type_subtyping_at(group_start).rec_group_position == 0;
        if !is_group_start || !group_bounds_are_consistent(module, group_start, size, n) {
            // Metadata that doesn't hold together as a real group starting
            // HERE -- unresolvable, and NOT safe to skip past: advance by
            // exactly one flat index so a malformed module can never
            // cause this loop to misalign with real group boundaries an
            // EARLIER, already-pushed entry might still depend on.
            out.push(None);
            costs.push(CanonicalCost::ZERO);
            idx += 1;
            continue;
        }
        let group_end = group_start + size;
        let mut members = Vec::with_capacity(size as usize);
        let mut group_cost = CanonicalCost::ZERO;
        let mut all_members_ok = true;
        for member_idx in group_start..group_end {
            match build_member_canonical(module, member_idx, group_start, group_end, &out, &costs) {
                Some((subtype, cost)) => {
                    group_cost = group_cost.combine_sum(cost);
                    members.push(subtype);
                    if !group_cost.within_caps() {
                        all_members_ok = false;
                        break;
                    }
                }
                None => {
                    all_members_ok = false;
                    break;
                }
            }
        }
        if all_members_ok {
            let candidate = CanonicalGroup { members };
            // Intern: reuse an earlier, content-identical group's `Rc`
            // rather than always allocating a fresh one -- see this
            // function's own doc comment for why this is the fix for a
            // real, security-review-confirmed per-comparison DoS, not
            // merely a memory optimization.
            let group_rc = match interned.get(&candidate) {
                Some(existing) => Rc::clone(existing),
                None => {
                    let rc = Rc::new(candidate);
                    interned.insert(Rc::clone(&rc));
                    rc
                }
            };
            for position in 0..size {
                out.push(Some((Rc::clone(&group_rc), position)));
                costs.push(group_cost);
            }
        } else {
            for _ in 0..size {
                out.push(None);
                costs.push(CanonicalCost::ZERO);
            }
        }
        idx = group_end;
    }
    out
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test 1: ValueType byte encoding matches WASM spec ─────────────────────
    //
    // WasmGC dropped #[repr(u8)] because StructRef needs a 2-byte encoding.
    // Use .encode() and .byte_tag() instead of `as u8`.

    #[test]
    fn value_type_byte_values() {
        // Single-byte numeric types.
        assert_eq!(ValueType::I32.encode(), vec![0x7F], "i32 tag");
        assert_eq!(ValueType::I64.encode(), vec![0x7E], "i64 tag");
        assert_eq!(ValueType::F32.encode(), vec![0x7D], "f32 tag");
        assert_eq!(ValueType::F64.encode(), vec![0x7C], "f64 tag");
        // WasmGC reference types.
        assert_eq!(ValueType::Anyref.encode(), vec![0x6E], "anyref tag");
        assert_eq!(ValueType::I31ref.encode(), vec![0x6C], "i31ref tag");
        // StructRef(0) → [0x63, 0x00]
        assert_eq!(ValueType::StructRef(0).encode(), vec![0x63, 0x00], "struct ref tag");
        // StructRef(1) → [0x63, 0x01]
        assert_eq!(ValueType::StructRef(1).encode(), vec![0x63, 0x01]);
        // ConcreteFuncRef shares StructRef's 2-byte encoding shape.
        assert_eq!(ValueType::ConcreteFuncRef(0).encode(), vec![0x63, 0x00], "concrete func ref tag");
        assert_eq!(ValueType::ConcreteFuncRef(1).encode(), vec![0x63, 0x01]);
        // Reference-types proposal (WASM17): funcref / externref.
        assert_eq!(ValueType::Funcref.encode(), vec![0x70], "funcref tag");
        assert_eq!(ValueType::Externref.encode(), vec![0x6F], "externref tag");
    }

    #[test]
    fn value_type_byte_tag_singles() {
        assert_eq!(ValueType::I32.byte_tag(), Some(0x7F));
        assert_eq!(ValueType::I64.byte_tag(), Some(0x7E));
        assert_eq!(ValueType::F32.byte_tag(), Some(0x7D));
        assert_eq!(ValueType::F64.byte_tag(), Some(0x7C));
        assert_eq!(ValueType::Anyref.byte_tag(), Some(0x6E));
        assert_eq!(ValueType::I31ref.byte_tag(), Some(0x6C));
        // StructRef has no single-byte tag.
        assert_eq!(ValueType::StructRef(0).byte_tag(), None);
        // Neither does ConcreteFuncRef.
        assert_eq!(ValueType::ConcreteFuncRef(0).byte_tag(), None);
        assert_eq!(ValueType::Funcref.byte_tag(), Some(0x70));
        assert_eq!(ValueType::Externref.byte_tag(), Some(0x6F));
    }

    #[test]
    fn value_type_funcref_externref_are_distinct() {
        // Funcref and Externref must not compare equal to each other or to
        // Anyref — the validator relies on ValueType's PartialEq to catch a
        // funcref-vs-externref mixup (see W08 spec's wasm-validator section).
        assert_ne!(ValueType::Funcref, ValueType::Externref);
        assert_ne!(ValueType::Funcref, ValueType::Anyref);
        assert_ne!(ValueType::Externref, ValueType::Anyref);
    }

    #[test]
    fn value_type_concrete_func_ref_distinct_from_struct_ref_and_funcref() {
        // ConcreteFuncRef and StructRef are DIFFERENT Rust enum variants
        // (even though they share a binary tag byte, see ConcreteFuncRef's
        // own doc comment) -- PartialEq must never conflate them, and
        // neither should compare equal to the general `Funcref`.
        assert_ne!(ValueType::ConcreteFuncRef(0), ValueType::StructRef(0));
        assert_ne!(ValueType::ConcreteFuncRef(0), ValueType::Funcref);
        assert_ne!(ValueType::ConcreteFuncRef(0), ValueType::ConcreteFuncRef(1));
        assert_eq!(ValueType::ConcreteFuncRef(3), ValueType::ConcreteFuncRef(3));
    }

    // ── Test 2: ExternalKind byte values ─────────────────────────────────────

    #[test]
    fn external_kind_byte_values() {
        assert_eq!(ExternalKind::Function as u8, 0x00);
        assert_eq!(ExternalKind::Table as u8, 0x01);
        assert_eq!(ExternalKind::Memory as u8, 0x02);
        assert_eq!(ExternalKind::Global as u8, 0x03);
    }

    // ── Test 3: BLOCK_TYPE_EMPTY constant ────────────────────────────────────

    #[test]
    fn block_type_empty_constant() {
        assert_eq!(BLOCK_TYPE_EMPTY, 0x40);
    }

    // ── Test 4: FuncType construction and equality ────────────────────────────

    #[test]
    fn func_type_construction_and_equality() {
        let a = FuncType {
            params: vec![ValueType::I32, ValueType::I64],
            results: vec![ValueType::F32],
        };
        let b = FuncType {
            params: vec![ValueType::I32, ValueType::I64],
            results: vec![ValueType::F32],
        };
        assert_eq!(a, b);
    }

    // ── Test 5: FuncType with empty params and results ────────────────────────

    #[test]
    fn func_type_empty() {
        let ft = FuncType {
            params: vec![],
            results: vec![],
        };
        assert!(ft.params.is_empty());
        assert!(ft.results.is_empty());
    }

    // ── Test 6: FuncType with multiple params and results ─────────────────────

    #[test]
    fn func_type_multiple_params_and_results() {
        let ft = FuncType {
            params: vec![ValueType::I32, ValueType::I32, ValueType::F64],
            results: vec![ValueType::I64, ValueType::F32],
        };
        assert_eq!(ft.params.len(), 3);
        assert_eq!(ft.results.len(), 2);
        assert_eq!(ft.params[2], ValueType::F64);
        assert_eq!(ft.results[0], ValueType::I64);
    }

    // ── Test 7: Limits with only min ──────────────────────────────────────────

    #[test]
    fn limits_min_only() {
        let lim = Limits { min: 1, max: None };
        assert_eq!(lim.min, 1);
        assert_eq!(lim.max, None);
    }

    // ── Test 8: Limits with min and max ──────────────────────────────────────

    #[test]
    fn limits_min_and_max() {
        let lim = Limits { min: 1, max: Some(4) };
        assert_eq!(lim.min, 1);
        assert_eq!(lim.max, Some(4));
    }

    // ── Test 9: MemoryType construction ───────────────────────────────────────

    #[test]
    fn memory_type_construction() {
        let mt = MemoryType {
            limits: Limits { min: 2, max: Some(8) },
            shared: false,
            is64: false,
        };
        assert_eq!(mt.limits.min, 2);
        assert_eq!(mt.limits.max, Some(8));
    }

    // ── Test 9b: MemoryType with 64-bit addressing (W25 / memory64) ───────────

    #[test]
    fn memory_type_is64_construction() {
        // A real, spec-valid 64-bit memory's limits can exceed u32::MAX --
        // see code/specs/W25-wasm-memory64-first-slice.md.
        let big: u64 = (u32::MAX as u64) + 1;
        let mt = MemoryType {
            limits: Limits { min: big, max: Some(big * 2) },
            shared: false,
            is64: true,
        };
        assert!(mt.is64);
        assert_eq!(mt.limits.min, big);
        assert_eq!(mt.limits.max, Some(big * 2));
    }

    // ── Test 10: TableType default element type is FUNCREF ────────────────────

    #[test]
    fn table_type_default_element_type() {
        let tt = TableType {
            element_type: FUNCREF,
            limits: Limits { min: 0, max: None },
            is64: false,
        };
        assert_eq!(tt.element_type, 0x70);
        assert_eq!(tt.element_type, FUNCREF);
    }

    // ── Test 10b: TableType with 64-bit addressing (W26 / table64) ───────────

    #[test]
    fn table_type_is64_construction() {
        // A real, spec-valid 64-bit table's limits can exceed u32::MAX --
        // see code/specs/W26-wasm-table64-first-slice.md (table64's own
        // real spec ceiling is u64::MAX, unlike memory64's 2^48 pages).
        let big: u64 = (u32::MAX as u64) + 1;
        let tt = TableType {
            element_type: FUNCREF,
            limits: Limits { min: big, max: Some(big * 2) },
            is64: true,
        };
        assert!(tt.is64);
        assert_eq!(tt.limits.min, big);
        assert_eq!(tt.limits.max, Some(big * 2));
    }

    // ── Test 11: GlobalType mutable and immutable ─────────────────────────────

    #[test]
    fn global_type_mutability() {
        let mutable_g = GlobalType { value_type: ValueType::I32, mutable: true };
        let const_g = GlobalType { value_type: ValueType::F64, mutable: false };
        assert!(mutable_g.mutable);
        assert!(!const_g.mutable);
        assert_eq!(mutable_g.value_type, ValueType::I32);
        assert_eq!(const_g.value_type, ValueType::F64);
        // Verify encoding still works after WasmGC refactor.
        assert_eq!(mutable_g.value_type.encode(), vec![0x7F]);
        assert_eq!(const_g.value_type.encode(), vec![0x7C]);
    }

    // ── Test 12: Import for each ExternalKind ────────────────────────────────

    #[test]
    fn import_function() {
        let imp = Import {
            module_name: "env".to_string(),
            name: "abort".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(0),
        };
        assert_eq!(imp.kind, ExternalKind::Function);
        assert_eq!(imp.type_info, ImportTypeInfo::Function(0));
    }

    #[test]
    fn import_table() {
        let imp = Import {
            module_name: "env".to_string(),
            name: "table".to_string(),
            kind: ExternalKind::Table,
            type_info: ImportTypeInfo::Table(TableType {
                element_type: FUNCREF,
                limits: Limits { min: 0, max: None },
                is64: false,
            }),
        };
        assert_eq!(imp.kind, ExternalKind::Table);
    }

    #[test]
    fn import_memory() {
        let imp = Import {
            module_name: "env".to_string(),
            name: "memory".to_string(),
            kind: ExternalKind::Memory,
            type_info: ImportTypeInfo::Memory(MemoryType {
                limits: Limits { min: 1, max: Some(2) },
                shared: false,
                is64: false,
            }),
        };
        assert_eq!(imp.kind, ExternalKind::Memory);
    }

    #[test]
    fn import_global() {
        let imp = Import {
            module_name: "env".to_string(),
            name: "stack_ptr".to_string(),
            kind: ExternalKind::Global,
            type_info: ImportTypeInfo::Global(GlobalType {
                value_type: ValueType::I32,
                mutable: true,
            }),
        };
        assert_eq!(imp.kind, ExternalKind::Global);
    }

    // ── Test 13: Export construction ──────────────────────────────────────────

    #[test]
    fn export_construction() {
        let exp = Export {
            name: "main".to_string(),
            kind: ExternalKind::Function,
            index: 3,
        };
        assert_eq!(exp.name, "main");
        assert_eq!(exp.kind, ExternalKind::Function);
        assert_eq!(exp.index, 3);
    }

    // ── Test 14: Global with init_expr ────────────────────────────────────────

    #[test]
    fn global_with_init_expr() {
        // i32.const 42 ; end  →  [0x41, 0x2A, 0x0B]
        let g = Global {
            global_type: GlobalType { value_type: ValueType::I32, mutable: false },
            init_expr: vec![0x41, 0x2A, 0x0B],
        };
        assert_eq!(g.init_expr, vec![0x41, 0x2A, 0x0B]);
        assert_eq!(g.global_type.value_type, ValueType::I32);
    }

    // ── Test 15: Element with function_indices ────────────────────────────────

    #[test]
    fn element_with_function_indices() {
        let elem = Element {
            table_index: 0,
            offset_expr: vec![0x41, 0x00, 0x0B], // i32.const 0; end
            function_indices: vec![Some(1), Some(3), Some(5), Some(7)],
            is_passive: false,
        };
        assert_eq!(elem.table_index, 0);
        assert_eq!(elem.function_indices, vec![Some(1), Some(3), Some(5), Some(7)]);
        assert_eq!(elem.function_indices.len(), 4);
    }

    // ── Test 16: DataSegment ──────────────────────────────────────────────────

    #[test]
    fn data_segment() {
        let seg = DataSegment {
            memory_index: 0,
            offset_expr: vec![0x41, 0x80, 0x08, 0x0B], // i32.const 1024; end
            data: b"hello".to_vec(),
            is_passive: false,
        };
        assert_eq!(seg.memory_index, 0);
        assert_eq!(seg.data, b"hello");
    }

    // ── Test 17: FunctionBody ─────────────────────────────────────────────────

    #[test]
    fn function_body() {
        let body = FunctionBody {
            locals: vec![ValueType::I32, ValueType::I32],
            code: vec![0x41, 0x01, 0x0B], // i32.const 1; end
        };
        assert_eq!(body.locals.len(), 2);
        assert_eq!(body.locals[0], ValueType::I32);
        assert_eq!(body.code, vec![0x41, 0x01, 0x0B]);
    }

    // ── Test 18: CustomSection ────────────────────────────────────────────────

    #[test]
    fn custom_section() {
        let sec = CustomSection {
            name: "name".to_string(),
            data: vec![0x01, 0x02, 0x03],
        };
        assert_eq!(sec.name, "name");
        assert_eq!(sec.data.len(), 3);
    }

    // ── Test 19: WasmModule has all required fields ───────────────────────────

    #[test]
    fn wasm_module_has_all_fields() {
        let m = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![ValueType::I32] }],
            type_subtyping: vec![],
            struct_types: vec![],
            array_types: vec![],
            type_kinds: vec![],
            imports: vec![],
            functions: vec![0],
            tables: vec![],
            memories: vec![MemoryType { limits: Limits { min: 1, max: None }, shared: false, is64: false }],
            globals: vec![],
            exports: vec![Export { name: "main".to_string(), kind: ExternalKind::Function, index: 0 }],
            start: Some(0),
            elements: vec![],
            code: vec![FunctionBody { locals: vec![], code: vec![0x0B] }],
            data: vec![],
            customs: vec![],
            tags: vec![],
        };
        assert_eq!(m.types.len(), 1);
        assert_eq!(m.struct_types.len(), 0);
        assert_eq!(m.functions, vec![0]);
        assert_eq!(m.start, Some(0));
        assert_eq!(m.exports[0].name, "main");
        assert!(m.tags.is_empty());
    }

    // ── Test 20: WasmModule default is all-empty ──────────────────────────────

    #[test]
    fn wasm_module_default_is_empty() {
        let m = WasmModule::default();
        assert!(m.types.is_empty());
        assert!(m.struct_types.is_empty());
        assert!(m.imports.is_empty());
        assert!(m.functions.is_empty());
        assert!(m.tables.is_empty());
        assert!(m.memories.is_empty());
        assert!(m.globals.is_empty());
        assert!(m.exports.is_empty());
        assert_eq!(m.start, None);
        assert!(m.elements.is_empty());
        assert!(m.code.is_empty());
        assert!(m.data.is_empty());
        assert!(m.customs.is_empty());
        assert!(m.tags.is_empty());
    }

    // ── WasmGC type tests ──────────────────────────────────────────────────────

    // Test 21: FieldType construction
    #[test]
    fn field_type_construction() {
        let f = FieldType::plain(ValueType::Anyref, true);
        assert_eq!(f.storage, StorageType::Val(ValueType::Anyref));
        assert!(f.mutable);

        let g = FieldType::plain(ValueType::I32, false);
        assert_eq!(g.storage, StorageType::Val(ValueType::I32));
        assert!(!g.mutable);
    }

    // Test 22: StructType for LispyPair has two anyref fields
    #[test]
    fn struct_type_lispy_pair() {
        let lispy_pair = StructType {
            fields: vec![
                FieldType::plain(ValueType::Anyref, true), // $head
                FieldType::plain(ValueType::Anyref, true), // $tail
            ],
        };
        assert_eq!(lispy_pair.fields.len(), 2);
        assert_eq!(lispy_pair.fields[0].storage, StorageType::Val(ValueType::Anyref));
        assert!(lispy_pair.fields[0].mutable);
        assert_eq!(lispy_pair.fields[1].storage, StorageType::Val(ValueType::Anyref));
        assert!(lispy_pair.fields[1].mutable);
    }

    // Test 23: WasmModule with struct_types carries the GC definition
    #[test]
    fn wasm_module_with_struct_types() {
        let m = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            struct_types: vec![StructType {
                fields: vec![
                    FieldType::plain(ValueType::Anyref, true),
                    FieldType::plain(ValueType::Anyref, true),
                ],
            }],
            ..Default::default()
        };
        assert_eq!(m.types.len(), 1);
        assert_eq!(m.struct_types.len(), 1);
        // The struct type index in the type section is types.len() + 0 = 1.
        assert_eq!(m.types.len(), 1);
    }

    // ── W33 fourth slice: StorageType / ArrayType / TypeKind ───────────────────

    #[test]
    fn storage_type_widened_type_extends_packed_to_i32_and_passes_through_val() {
        assert_eq!(StorageType::I8.widened_type(), ValueType::I32);
        assert_eq!(StorageType::I16.widened_type(), ValueType::I32);
        assert_eq!(StorageType::Val(ValueType::F64).widened_type(), ValueType::F64);
    }

    #[test]
    fn storage_type_is_packed_and_packed_bits() {
        assert!(StorageType::I8.is_packed());
        assert!(StorageType::I16.is_packed());
        assert!(!StorageType::Val(ValueType::I32).is_packed());
        assert_eq!(StorageType::I8.packed_bits(), Some(8));
        assert_eq!(StorageType::I16.packed_bits(), Some(16));
        assert_eq!(StorageType::Val(ValueType::I32).packed_bits(), None);
    }

    #[test]
    fn field_type_plain_matches_the_old_val_type_shape() {
        let f = FieldType::plain(ValueType::I32, true);
        assert_eq!(f.storage, StorageType::Val(ValueType::I32));
        assert!(f.mutable);
    }

    #[test]
    fn array_type_carries_its_element_field() {
        let at = ArrayType { element: FieldType::plain(ValueType::F32, false) };
        assert_eq!(at.element.storage, StorageType::Val(ValueType::F32));
        assert!(!at.element.mutable);
    }

    #[test]
    fn struct_type_at_uses_type_kinds_when_present() {
        // Two func types, then a struct DECLARED BETWEEN them in flat index
        // space (index 1) -- exactly the interleaving the legacy
        // `types.len() + k` formula cannot represent, since the struct isn't
        // "after all func types."
        let m = WasmModule {
            types: vec![
                FuncType { params: vec![], results: vec![] },
                FuncType { params: vec![], results: vec![] }, // dummy at struct's flat index
                FuncType { params: vec![ValueType::I32], results: vec![] },
            ],
            type_kinds: vec![TypeKind::Func, TypeKind::Struct(0), TypeKind::Func],
            struct_types: vec![StructType { fields: vec![FieldType::plain(ValueType::I64, false)] }],
            ..Default::default()
        };
        assert!(m.struct_type_at(0).is_none(), "index 0 is a func, not a struct");
        let st = m.struct_type_at(1).expect("index 1 is the struct");
        assert_eq!(st.fields.len(), 1);
        assert!(m.struct_type_at(2).is_none(), "index 2 is a func, not a struct");
        assert!(m.struct_type_at(99).is_none(), "out of range");
    }

    #[test]
    fn struct_type_at_falls_back_to_legacy_offset_when_type_kinds_is_empty() {
        // The pre-W33-fourth-slice shape: type_kinds never populated at all
        // (binary decoder, or any older hand-built WasmModule).
        let m = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            struct_types: vec![StructType { fields: vec![FieldType::plain(ValueType::Anyref, true)] }],
            ..Default::default()
        };
        assert!(m.type_kinds.is_empty());
        let st = m.struct_type_at(1).expect("legacy offset: struct 0 is at types.len() + 0 = 1");
        assert_eq!(st.fields.len(), 1);
        assert!(m.struct_type_at(0).is_none(), "index 0 is the func type, not the struct");
    }

    #[test]
    fn array_type_at_uses_type_kinds_when_present() {
        let m = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            type_kinds: vec![TypeKind::Array(0)],
            array_types: vec![ArrayType { element: FieldType::plain(ValueType::I32, true) }],
            ..Default::default()
        };
        let at = m.array_type_at(0).expect("index 0 is the array");
        assert!(at.element.mutable);
        assert!(m.struct_type_at(0).is_none(), "an array is not a struct");
    }

    #[test]
    fn array_type_at_falls_back_to_legacy_offset_past_struct_types() {
        let m = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            struct_types: vec![StructType { fields: vec![] }],
            array_types: vec![ArrayType { element: FieldType::plain(ValueType::F32, false) }],
            ..Default::default()
        };
        // Legacy offset: array 0 is at types.len() + struct_types.len() + 0 = 2.
        assert!(m.array_type_at(2).is_some());
        assert!(m.array_type_at(1).is_none(), "index 1 is the struct, not the array");
    }

    // Test 24: StructRef(idx) encodes correctly
    #[test]
    fn struct_ref_encoding() {
        // StructRef(0) → [0x63, 0x00]
        assert_eq!(ValueType::StructRef(0).encode(), vec![0x63, 0x00]);
        // StructRef(5) → [0x63, 0x05]
        assert_eq!(ValueType::StructRef(5).encode(), vec![0x63, 0x05]);
        // Large index: 128 needs 2 LEB128 bytes.
        let enc = ValueType::StructRef(128).encode();
        assert_eq!(enc[0], 0x63);
        assert!(enc.len() >= 3); // 0x63 + 2 LEB128 bytes for 128
    }

    // Test 25: Anyref and I31ref encode as single bytes
    #[test]
    fn anyref_i31ref_single_byte() {
        assert_eq!(ValueType::Anyref.encode(), vec![0x6E]);
        assert_eq!(ValueType::I31ref.encode(), vec![0x6C]);
    }

    // ── Additional: BlockType variants ───────────────────────────────────────

    #[test]
    fn block_type_variants() {
        assert_eq!(BlockType::Empty, BlockType::Empty);
        assert_eq!(BlockType::Value(ValueType::I32), BlockType::Value(ValueType::I32));
        assert_ne!(BlockType::Value(ValueType::I32), BlockType::Value(ValueType::I64));
        assert_eq!(BlockType::TypeIndex(5), BlockType::TypeIndex(5));
    }

    // ── Additional: ValueType Clone semantics ────────────────────────────────
    //
    // ValueType lost Copy when we added StructRef(u32) — that variant holds
    // a u32 but the enum is no longer #[repr(u8)].  WasmGC encoding instead
    // uses the ValueType::encode() method.  Use Clone where a copy is needed.

    #[test]
    fn value_type_is_clone() {
        let a = ValueType::I32;
        let b = a;
        assert_eq!(a, b);
        let c = ValueType::StructRef(42);
        let d = c;
        assert_eq!(c, d);
    }

    // ── Additional: FUNCREF constant ─────────────────────────────────────────

    #[test]
    fn funcref_constant() {
        assert_eq!(FUNCREF, 0x70);
    }

    // ── W32 first slice: the four bottom reference types ─────────────────────
    //
    // `code/specs/W32-wasm-non-null-concrete-reference-types.md` section 1
    // adds `NullFuncref`/`NullExternref`/`NullExnref`/`NullRef`. The tag
    // bytes below were independently verified against the real reference
    // interpreter's `interpreter/binary/decode.ml` (`NoFuncHT = -0x0d`,
    // `NoExternHT = -0x0e`, `NoExnHT = -0x0c`, `NoneHT = -0x0f`; SLEB128
    // single-byte encoding of a small negative value is `value mod 128`),
    // NOT just re-asserted from this crate's own doc comments -- the same
    // "verify independently" discipline W24's `exnref` tag-byte bug
    // established.

    #[test]
    fn bottom_ref_types_encode_to_the_verified_tag_bytes() {
        assert_eq!(ValueType::NullFuncref.encode(), vec![0x73], "nullfuncref / nofunc");
        assert_eq!(ValueType::NullExternref.encode(), vec![0x72], "nullexternref / noextern");
        assert_eq!(ValueType::NullExnref.encode(), vec![0x74], "nullexnref / noexn");
        assert_eq!(ValueType::NullRef.encode(), vec![0x71], "nullref / none");
    }

    #[test]
    fn bottom_ref_types_byte_tag_matches_encode() {
        assert_eq!(ValueType::NullFuncref.byte_tag(), Some(0x73));
        assert_eq!(ValueType::NullExternref.byte_tag(), Some(0x72));
        assert_eq!(ValueType::NullExnref.byte_tag(), Some(0x74));
        assert_eq!(ValueType::NullRef.byte_tag(), Some(0x71));
    }

    #[test]
    fn bottom_ref_types_are_distinct_from_each_other_and_their_supertypes() {
        // PartialEq must never conflate a bottom type with its nullable
        // supertype (the exact mistake an earlier "lossy aliasing" pass
        // made -- see `wasm-wast-parser::module::parse_value_type`'s own
        // doc comment on this variant).
        assert_ne!(ValueType::NullFuncref, ValueType::Funcref);
        assert_ne!(ValueType::NullExternref, ValueType::Externref);
        assert_ne!(ValueType::NullExnref, ValueType::Exnref);
        assert_ne!(ValueType::NullRef, ValueType::Anyref);
        assert_ne!(ValueType::NullRef, ValueType::I31ref);
        assert_ne!(ValueType::NullRef, ValueType::StructRef(0));
        // And distinct from each other, across hierarchies.
        assert_ne!(ValueType::NullFuncref, ValueType::NullExternref);
        assert_ne!(ValueType::NullFuncref, ValueType::NullExnref);
        assert_ne!(ValueType::NullFuncref, ValueType::NullRef);
        assert_ne!(ValueType::NullExternref, ValueType::NullExnref);
        assert_ne!(ValueType::NullExternref, ValueType::NullRef);
        assert_ne!(ValueType::NullExnref, ValueType::NullRef);
    }

    // ── W32 §2: bottom-type subtyping lattice -- POSITIVE directions ─────────
    //
    // Each rule from the spec's section 2 needs both a positive test (the
    // subtype IS accepted) and a negative test (the reverse direction is
    // REJECTED) -- see the spec's own "Verification plan".

    #[test]
    fn nullfuncref_is_a_bottom_subtype_of_funcref_and_concrete_funcref() {
        assert!(ValueType::NullFuncref.is_bottom_subtype_of(&ValueType::Funcref));
        assert!(ValueType::NullFuncref.is_bottom_subtype_of(&ValueType::ConcreteFuncRef(0)));
        assert!(ValueType::NullFuncref.is_bottom_subtype_of(&ValueType::ConcreteFuncRef(99)), "bottom of the WHOLE func hierarchy, every index");
    }

    #[test]
    fn nullexternref_is_a_bottom_subtype_of_externref() {
        assert!(ValueType::NullExternref.is_bottom_subtype_of(&ValueType::Externref));
    }

    #[test]
    fn nullexnref_is_a_bottom_subtype_of_exnref() {
        assert!(ValueType::NullExnref.is_bottom_subtype_of(&ValueType::Exnref));
    }

    #[test]
    fn nullref_is_a_bottom_subtype_of_anyref_i31ref_and_every_structref() {
        assert!(ValueType::NullRef.is_bottom_subtype_of(&ValueType::Anyref));
        assert!(ValueType::NullRef.is_bottom_subtype_of(&ValueType::I31ref));
        assert!(ValueType::NullRef.is_bottom_subtype_of(&ValueType::StructRef(0)));
        assert!(ValueType::NullRef.is_bottom_subtype_of(&ValueType::StructRef(42)), "any struct-type index");
    }

    #[test]
    fn nullref_is_a_bottom_subtype_of_every_arrayref() {
        assert!(ValueType::NullRef.is_bottom_subtype_of(&ValueType::ArrayRef(0)));
        assert!(ValueType::NullRef.is_bottom_subtype_of(&ValueType::ArrayRef(7)), "any array-type index");
    }

    #[test]
    fn array_ref_and_non_null_array_ref_encode_like_struct_ref() {
        assert_eq!(ValueType::ArrayRef(0).encode(), vec![0x63, 0x00]);
        assert_eq!(ValueType::ArrayRef(5).encode(), vec![0x63, 0x05]);
        assert_eq!(ValueType::NonNullArrayRef(0).encode(), vec![0x64, 0x00]);
        assert!(ValueType::ArrayRef(0).byte_tag().is_none());
        assert!(ValueType::NonNullArrayRef(0).byte_tag().is_none());
    }

    #[test]
    fn non_null_arrayref_is_a_subtype_of_arrayref_same_index_and_of_anyref() {
        assert!(ValueType::NonNullArrayRef(3).is_non_null_subtype_of(&ValueType::ArrayRef(3)));
        assert!(!ValueType::NonNullArrayRef(3).is_non_null_subtype_of(&ValueType::ArrayRef(4)), "index must match");
        assert!(ValueType::NonNullArrayRef(3).is_non_null_subtype_of(&ValueType::Anyref));
        assert!(!ValueType::ArrayRef(3).is_non_null_subtype_of(&ValueType::NonNullArrayRef(3)), "never reverses");
    }

    // ── W32 §2: bottom-type subtyping lattice -- NEGATIVE directions ─────────
    //
    // "The reverse direction never holds" -- a nullable supertype is never
    // a subtype of its hierarchy's bottom type, and NEITHER direction holds
    // for a NON-NULL slot (bottom types are still nullable, just never
    // satisfy a slot this crate doesn't yet model as non-null anyway --
    // this slice has no non-null variants to test against, see the spec's
    // "explicitly out of scope"). Also: no cross-hierarchy subtyping.

    #[test]
    fn funcref_is_never_a_bottom_subtype_of_nullfuncref() {
        assert!(!ValueType::Funcref.is_bottom_subtype_of(&ValueType::NullFuncref));
        assert!(!ValueType::ConcreteFuncRef(0).is_bottom_subtype_of(&ValueType::NullFuncref));
    }

    #[test]
    fn externref_is_never_a_bottom_subtype_of_nullexternref() {
        assert!(!ValueType::Externref.is_bottom_subtype_of(&ValueType::NullExternref));
    }

    #[test]
    fn exnref_is_never_a_bottom_subtype_of_nullexnref() {
        assert!(!ValueType::Exnref.is_bottom_subtype_of(&ValueType::NullExnref));
    }

    #[test]
    fn anyref_i31ref_structref_are_never_bottom_subtypes_of_nullref() {
        assert!(!ValueType::Anyref.is_bottom_subtype_of(&ValueType::NullRef));
        assert!(!ValueType::I31ref.is_bottom_subtype_of(&ValueType::NullRef));
        assert!(!ValueType::StructRef(0).is_bottom_subtype_of(&ValueType::NullRef));
    }

    #[test]
    fn bottom_types_never_cross_into_a_different_hierarchy() {
        // NullFuncref is bottom of the FUNC hierarchy only -- never a
        // subtype of extern/exn/any-hierarchy types, and vice versa for
        // the other three bottom types.
        assert!(!ValueType::NullFuncref.is_bottom_subtype_of(&ValueType::Externref));
        assert!(!ValueType::NullFuncref.is_bottom_subtype_of(&ValueType::Exnref));
        assert!(!ValueType::NullFuncref.is_bottom_subtype_of(&ValueType::Anyref));
        assert!(!ValueType::NullExternref.is_bottom_subtype_of(&ValueType::Funcref));
        assert!(!ValueType::NullExternref.is_bottom_subtype_of(&ValueType::Exnref));
        assert!(!ValueType::NullExternref.is_bottom_subtype_of(&ValueType::Anyref));
        assert!(!ValueType::NullExnref.is_bottom_subtype_of(&ValueType::Funcref));
        assert!(!ValueType::NullExnref.is_bottom_subtype_of(&ValueType::Externref));
        assert!(!ValueType::NullExnref.is_bottom_subtype_of(&ValueType::Anyref));
        assert!(!ValueType::NullRef.is_bottom_subtype_of(&ValueType::Funcref));
        assert!(!ValueType::NullRef.is_bottom_subtype_of(&ValueType::Externref));
        assert!(!ValueType::NullRef.is_bottom_subtype_of(&ValueType::Exnref));
    }

    #[test]
    fn is_bottom_subtype_of_is_not_reflexive_and_ignores_equal_types() {
        // Documented contract: this method does NOT encode reflexivity --
        // callers combine it with `==` themselves (see `wasm_validator::
        // type_check::is_assignable`, which does exactly that).
        assert!(!ValueType::Funcref.is_bottom_subtype_of(&ValueType::Funcref));
        assert!(!ValueType::NullFuncref.is_bottom_subtype_of(&ValueType::NullFuncref));
        assert!(!ValueType::I32.is_bottom_subtype_of(&ValueType::I32));
    }

    // ── W32 second slice: non-null concrete reference types ──────────────────
    //
    // `code/specs/W32-wasm-non-null-concrete-reference-types.md`'s addendum
    // section 1 adds `NonNullStructRef(u32)`/`NonNullConcreteFuncRef(u32)`,
    // tag byte `0x64` -- independently verified against the real reference
    // interpreter's `interpreter/binary/decode.ml` (`ref_type`'s
    // `-0x1c -> (NoNull, heap_type s)` arm: `-28 mod 128 = 0x64`), NOT just
    // re-asserted from this crate's own doc comments.

    #[test]
    fn non_null_concrete_refs_encode_to_the_verified_0x64_tag_byte() {
        assert_eq!(ValueType::NonNullStructRef(0).encode(), vec![0x64, 0x00], "non-null struct ref tag");
        assert_eq!(ValueType::NonNullStructRef(5).encode(), vec![0x64, 0x05]);
        assert_eq!(ValueType::NonNullConcreteFuncRef(0).encode(), vec![0x64, 0x00], "non-null concrete func ref tag");
        assert_eq!(ValueType::NonNullConcreteFuncRef(1).encode(), vec![0x64, 0x01]);
        // Large index needs 2 LEB128 bytes, same as StructRef/ConcreteFuncRef.
        let enc = ValueType::NonNullStructRef(128).encode();
        assert_eq!(enc[0], 0x64);
        assert!(enc.len() >= 3);
    }

    #[test]
    fn non_null_concrete_refs_have_no_single_byte_tag() {
        // Multi-byte, like their nullable counterparts.
        assert_eq!(ValueType::NonNullStructRef(0).byte_tag(), None);
        assert_eq!(ValueType::NonNullConcreteFuncRef(0).byte_tag(), None);
    }

    #[test]
    fn non_null_concrete_refs_are_distinct_rust_variants_from_their_nullable_counterparts() {
        // Same discipline as `ConcreteFuncRef`/`StructRef`'s own test: two
        // Rust enum variants that happen to share a binary tag-byte SHAPE
        // (0x63 vs 0x64 here -- distinct bytes, but the same "index space
        // disambiguates" convention) must never be `==` to each other, nor
        // to the wrong index of themselves.
        assert_ne!(ValueType::NonNullStructRef(0), ValueType::StructRef(0));
        assert_ne!(ValueType::NonNullConcreteFuncRef(0), ValueType::ConcreteFuncRef(0));
        assert_ne!(ValueType::NonNullStructRef(0), ValueType::NonNullConcreteFuncRef(0));
        assert_ne!(ValueType::NonNullStructRef(0), ValueType::NonNullStructRef(1));
        assert_eq!(ValueType::NonNullConcreteFuncRef(3), ValueType::NonNullConcreteFuncRef(3));
    }

    // ── W32 §2 (second slice): non-null subtyping lattice -- POSITIVE ────────

    #[test]
    fn non_null_structref_is_a_subtype_of_structref_same_index_and_of_anyref() {
        assert!(ValueType::NonNullStructRef(0).is_non_null_subtype_of(&ValueType::StructRef(0)));
        assert!(ValueType::NonNullStructRef(7).is_non_null_subtype_of(&ValueType::StructRef(7)));
        assert!(ValueType::NonNullStructRef(0).is_non_null_subtype_of(&ValueType::Anyref));
        assert!(ValueType::NonNullStructRef(99).is_non_null_subtype_of(&ValueType::Anyref), "any index");
    }

    #[test]
    fn non_null_concrete_funcref_is_a_subtype_of_concrete_funcref_same_index_and_of_funcref() {
        assert!(ValueType::NonNullConcreteFuncRef(0).is_non_null_subtype_of(&ValueType::ConcreteFuncRef(0)));
        assert!(ValueType::NonNullConcreteFuncRef(3).is_non_null_subtype_of(&ValueType::ConcreteFuncRef(3)));
        assert!(ValueType::NonNullConcreteFuncRef(0).is_non_null_subtype_of(&ValueType::Funcref));
        assert!(ValueType::NonNullConcreteFuncRef(50).is_non_null_subtype_of(&ValueType::Funcref), "any index");
    }

    // ── W32 §2 (second slice): non-null subtyping lattice -- NEGATIVE ────────
    //
    // "The reverse direction never holds" -- this is the asymmetry the
    // spec calls out explicitly: a NULLABLE type must NOT be accepted
    // where non-null is required, no matter how it's reached.

    #[test]
    fn structref_and_anyref_are_never_non_null_subtypes_of_non_null_structref() {
        assert!(!ValueType::StructRef(0).is_non_null_subtype_of(&ValueType::NonNullStructRef(0)));
        assert!(!ValueType::Anyref.is_non_null_subtype_of(&ValueType::NonNullStructRef(0)));
    }

    #[test]
    fn concrete_funcref_and_funcref_are_never_non_null_subtypes_of_non_null_concrete_funcref() {
        assert!(!ValueType::ConcreteFuncRef(0).is_non_null_subtype_of(&ValueType::NonNullConcreteFuncRef(0)));
        assert!(!ValueType::Funcref.is_non_null_subtype_of(&ValueType::NonNullConcreteFuncRef(0)));
    }

    #[test]
    fn non_null_structref_does_not_flow_across_a_mismatched_index() {
        assert!(!ValueType::NonNullStructRef(0).is_non_null_subtype_of(&ValueType::StructRef(1)));
        assert!(!ValueType::NonNullConcreteFuncRef(0).is_non_null_subtype_of(&ValueType::ConcreteFuncRef(1)));
    }

    #[test]
    fn bottom_types_never_satisfy_a_non_null_slot() {
        // A null value can never satisfy a non-null slot, no matter how
        // far down the bottom of the lattice it sits -- neither
        // `is_bottom_subtype_of` nor `is_non_null_subtype_of` should ever
        // report a bottom type as flowing into a `NonNull*` variant (the
        // asymmetry `NullRef`/`NullFuncref`'s own doc comments call out).
        assert!(!ValueType::NullRef.is_bottom_subtype_of(&ValueType::NonNullStructRef(0)));
        assert!(!ValueType::NullRef.is_non_null_subtype_of(&ValueType::NonNullStructRef(0)));
        assert!(!ValueType::NullFuncref.is_bottom_subtype_of(&ValueType::NonNullConcreteFuncRef(0)));
        assert!(!ValueType::NullFuncref.is_non_null_subtype_of(&ValueType::NonNullConcreteFuncRef(0)));
    }

    #[test]
    fn is_non_null_subtype_of_is_not_reflexive_and_ignores_equal_types() {
        assert!(!ValueType::NonNullStructRef(0).is_non_null_subtype_of(&ValueType::NonNullStructRef(0)));
        assert!(!ValueType::Funcref.is_non_null_subtype_of(&ValueType::Funcref));
        assert!(!ValueType::I32.is_non_null_subtype_of(&ValueType::I32));
    }

    // ── W33 first slice: `TypeSubtyping` / `WasmModule` GC nominal-subtyping
    // and `rec`-group helpers ───────────────────────────────────────────────

    #[test]
    fn type_subtyping_default_is_final_no_supertype_singleton_group() {
        let st = TypeSubtyping::default();
        assert_eq!(st.supertype, None);
        assert!(st.is_final);
        assert_eq!(st.rec_group_size, 1);
        assert_eq!(st.rec_group_position, 0);
    }

    #[test]
    fn type_subtyping_at_falls_back_to_default_when_module_never_populated_it() {
        // A `WasmModule` built before this field existed (every hand-built
        // test literal, `wasm-module-parser`'s binary decoder) leaves
        // `type_subtyping` empty even though `types` is not -- this must
        // never panic, and must report the pre-W33 "plain" semantics.
        let m = WasmModule { types: vec![FuncType { params: vec![], results: vec![] }], ..Default::default() };
        assert_eq!(m.type_subtyping_at(0), TypeSubtyping::default());
        // Out-of-range is just as safe.
        assert_eq!(m.type_subtyping_at(99), TypeSubtyping::default());
    }

    #[test]
    fn func_type_is_nominal_subtype_is_reflexive() {
        let m = WasmModule { types: vec![FuncType { params: vec![], results: vec![] }], ..Default::default() };
        assert!(m.func_type_is_nominal_subtype(0, 0));
    }

    #[test]
    fn func_type_is_nominal_subtype_walks_a_direct_parent_chain() {
        // t2 sub t1, t3 sub t2 -- t3 <: t2 <: t1, transitively t3 <: t1.
        let m = WasmModule {
            types: vec![
                FuncType { params: vec![], results: vec![] },
                FuncType { params: vec![], results: vec![] },
                FuncType { params: vec![], results: vec![] },
            ],
            type_subtyping: vec![
                TypeSubtyping::default(),
                TypeSubtyping { supertype: Some(0), ..Default::default() },
                TypeSubtyping { supertype: Some(1), ..Default::default() },
            ],
            ..Default::default()
        };
        assert!(m.func_type_is_nominal_subtype(1, 0)); // t2 <: t1 (direct)
        assert!(m.func_type_is_nominal_subtype(2, 1)); // t3 <: t2 (direct)
        assert!(m.func_type_is_nominal_subtype(2, 0)); // t3 <: t1 (transitive)
        // Never the reverse direction.
        assert!(!m.func_type_is_nominal_subtype(0, 1));
        assert!(!m.func_type_is_nominal_subtype(0, 2));
        assert!(!m.func_type_is_nominal_subtype(1, 2));
    }

    #[test]
    fn func_type_is_nominal_subtype_false_for_unrelated_types() {
        let m = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }, FuncType { params: vec![], results: vec![] }],
            ..Default::default()
        };
        // Two independently-declared final types with no `sub` chain
        // between them are NOT nominal subtypes of each other, even if
        // structurally identical -- `func_type_is_nominal_subtype` itself
        // stays nominal-only BY DESIGN (see its own doc comment), since
        // `WasmModule` never carries `canonical_types`; canonical
        // equivalence is exactly what `nominal_subtype_chain` gains when a
        // REAL caller (below) passes one in.
        assert!(!m.func_type_is_nominal_subtype(0, 1));
        assert!(!m.func_type_is_nominal_subtype(1, 0));
    }

    /// W34 third slice (`code/specs/W34-wasm-gc-canonical-type-equivalence.md`):
    /// positive case -- two independently-declared, nominally-UNRELATED
    /// (no `sub` chain at all) but canonically-EQUIVALENT (byte-identical
    /// tied shape) types must be accepted once a real `canonical_types`
    /// table is supplied, even though `func_type_is_nominal_subtype`
    /// (nominal-only, no canonical data) correctly rejects the exact same
    /// pair -- this is the direct proof that `nominal_subtype_chain`'s new
    /// `canonical_types` parameter, not just its existing nominal chain
    /// walk, is what closes the gap.
    #[test]
    fn nominal_subtype_chain_accepts_canonically_equivalent_but_nominally_unrelated_types() {
        let m = WasmModule {
            types: vec![
                FuncType { params: vec![ValueType::I32], results: vec![] },
                FuncType { params: vec![ValueType::I32], results: vec![] },
            ],
            ..Default::default()
        };
        // Nominal-only: correctly rejected, no declared `sub` relationship.
        assert!(!m.func_type_is_nominal_subtype(0, 1));
        assert!(!m.func_type_is_nominal_subtype(1, 0));
        // With real canonical data: both directions accepted, since `<:`
        // per the GC proposal's own rule is "nominal modulo canonical
        // equivalence" -- reflexive-equivalent types are subtypes of each
        // other regardless of `sub`-chain declaration.
        let canonical = canonicalize_types(&m);
        assert!(nominal_subtype_chain(&m.type_subtyping, &canonical, 0, 1));
        assert!(nominal_subtype_chain(&m.type_subtyping, &canonical, 1, 0));
    }

    /// W34 third slice: negative case -- two independently-declared,
    /// nominally-unrelated AND canonically-INEQUIVALENT (genuinely
    /// different shape) types must still be correctly rejected even with a
    /// real `canonical_types` table present -- proves the upgrade never
    /// introduces a false accept for a genuinely different pair.
    #[test]
    fn nominal_subtype_chain_still_rejects_canonically_inequivalent_unrelated_types() {
        let m = WasmModule {
            types: vec![
                FuncType { params: vec![ValueType::I32], results: vec![] },
                FuncType { params: vec![ValueType::I64], results: vec![] },
            ],
            ..Default::default()
        };
        let canonical = canonicalize_types(&m);
        assert!(!nominal_subtype_chain(&m.type_subtyping, &canonical, 0, 1));
        assert!(!nominal_subtype_chain(&m.type_subtyping, &canonical, 1, 0));
    }

    /// W34 third slice: an empty `canonical_types` slice (the exact table
    /// `func_type_is_nominal_subtype` itself passes) must behave IDENTICALLY
    /// to the pre-W34 nominal-only rule -- proves the new parameter is a
    /// strict, zero-behavior-change superset for every caller that has no
    /// canonical data at all, not a silent behavior change.
    #[test]
    fn nominal_subtype_chain_with_empty_canonical_table_matches_old_nominal_only_behavior() {
        let m = WasmModule {
            types: vec![
                FuncType { params: vec![ValueType::I32], results: vec![] },
                FuncType { params: vec![ValueType::I32], results: vec![] },
                FuncType { params: vec![], results: vec![] },
            ],
            type_subtyping: vec![
                TypeSubtyping::default(),
                TypeSubtyping { supertype: Some(0), ..Default::default() },
                TypeSubtyping::default(),
            ],
            ..Default::default()
        };
        assert!(nominal_subtype_chain(&m.type_subtyping, &[], 1, 0)); // declared sub chain still works
        assert!(!nominal_subtype_chain(&m.type_subtyping, &[], 0, 2)); // unrelated, no canonical data at all
    }

    #[test]
    fn func_type_is_nominal_subtype_never_loops_forever_on_a_cycle() {
        // A malformed/cyclic chain (should be rejected by `wasm-validator`
        // before this is ever called on it) must still terminate here.
        let m = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }, FuncType { params: vec![], results: vec![] }],
            type_subtyping: vec![
                TypeSubtyping { supertype: Some(1), ..Default::default() },
                TypeSubtyping { supertype: Some(0), ..Default::default() },
            ],
            ..Default::default()
        };
        assert!(!m.func_type_is_nominal_subtype(0, 99));
    }

    #[test]
    fn func_type_is_nominal_subtype_bounds_chain_walk_to_a_fixed_hop_cap() {
        // Security review finding (W33 first slice): a chain longer than
        // `MAX_SUBTYPE_CHAIN_HOPS` must report "not a nominal subtype"
        // rather than walking arbitrarily far -- the whole point is
        // bounding PER-QUERY cost to a constant regardless of how many
        // types a (potentially adversarial) module declares. Build a
        // chain one hop longer than the cap: types[i] sub types[i-1] for
        // i in 1..=N, with N > MAX_SUBTYPE_CHAIN_HOPS.
        let n = (MAX_SUBTYPE_CHAIN_HOPS + 10) as usize;
        let types = vec![FuncType { params: vec![], results: vec![] }; n + 1];
        let mut type_subtyping = vec![TypeSubtyping::default()];
        for i in 1..=n {
            type_subtyping.push(TypeSubtyping { supertype: Some((i - 1) as u32), is_final: false, ..Default::default() });
        }
        let m = WasmModule { types, type_subtyping, ..Default::default() };
        // A query landing WITHIN the cap still succeeds.
        assert!(m.func_type_is_nominal_subtype(10, 0));
        // A query whose real chain length exceeds the cap reports false
        // -- a safe false negative (this can only make the caller
        // reject something a deeper walk might have accepted), not a
        // false positive.
        assert!(!m.func_type_is_nominal_subtype(n as u32, 0));
    }

    #[test]
    fn type_group_shape_reports_size_and_position() {
        let m = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }; 2],
            type_subtyping: vec![
                TypeSubtyping { rec_group_size: 2, rec_group_position: 0, ..Default::default() },
                TypeSubtyping { rec_group_size: 2, rec_group_position: 1, ..Default::default() },
            ],
            ..Default::default()
        };
        assert_eq!(m.type_group_shape(0), (2, 0));
        assert_eq!(m.type_group_shape(1), (2, 1));
        // Out of range falls back to the singleton default.
        assert_eq!(m.type_group_shape(5), (1, 0));
    }

    // ────────────────────────────────────────────────────────────────────
    // W34 first slice: canonical type-group equivalence, singleton groups
    // ────────────────────────────────────────────────────────────────────

    /// `type-rec.wast` line 4: `(type (func (param (ref 0)) (result (ref
    /// 0))))` -- a flat, non-`rec`-wrapped self-referencing type. The ONLY
    /// in-group reference a singleton group can express must tie to
    /// `Rec(0)`, never a raw absolute index.
    #[test]
    fn self_referencing_singleton_ties_to_rec_zero() {
        let m = WasmModule {
            types: vec![FuncType { params: vec![ValueType::ConcreteFuncRef(0)], results: vec![ValueType::ConcreteFuncRef(0)] }],
            type_subtyping: vec![TypeSubtyping::default()],
            ..Default::default()
        };
        let canonical = canonicalize_types(&m);
        assert_eq!(canonical.len(), 1);
        let (group, pos) = canonical[0].as_ref().expect("singleton self-reference must canonicalize");
        assert_eq!(*pos, 0);
        assert_eq!(
            group.members,
            vec![CanonicalSubtype {
                is_final: true,
                supertype: None,
                comp: CanonicalCompType::Func(
                    vec![CanonicalValType::Ref(true, CanonicalHeapRef::Rec(0))],
                    vec![CanonicalValType::Ref(true, CanonicalHeapRef::Rec(0))],
                ),
            }]
        );
    }

    /// `type-rec.wast` line 14: `(rec (type $g (func (param (ref $g))
    /// (result (ref $g)))))` -- an EXPLICIT singleton `rec` group
    /// self-referencing. Must tie identically to the implicit-singleton
    /// case above (same `Rec(0)` marker) -- the whole point of "singleton
    /// group" being one concept regardless of `rec`-wrapping syntax.
    #[test]
    fn explicit_singleton_rec_group_self_reference_ties_the_same_as_implicit() {
        let explicit = WasmModule {
            types: vec![FuncType { params: vec![ValueType::ConcreteFuncRef(0)], results: vec![ValueType::ConcreteFuncRef(0)] }],
            type_subtyping: vec![TypeSubtyping { rec_group_size: 1, rec_group_position: 0, ..Default::default() }],
            ..Default::default()
        };
        let implicit = WasmModule {
            types: vec![FuncType { params: vec![ValueType::ConcreteFuncRef(0)], results: vec![ValueType::ConcreteFuncRef(0)] }],
            type_subtyping: vec![TypeSubtyping::default()],
            ..Default::default()
        };
        assert_eq!(canonicalize_types(&explicit), canonicalize_types(&implicit));
    }

    /// Cross-module comparability -- the whole point of canonicalization
    /// (MVP.md's own "no shared numbering needed" promise). Two
    /// INDEPENDENTLY-constructed `WasmModule`s, with the self-referencing
    /// type sitting at completely different flat indices (padded with
    /// unrelated leading types in the second module), must still tie to
    /// byte-identical `CanonicalGroup` values.
    #[test]
    fn two_independently_indexed_isomorphic_singletons_canonicalize_identically() {
        let module_a = WasmModule {
            types: vec![FuncType { params: vec![ValueType::ConcreteFuncRef(0)], results: vec![] }],
            type_subtyping: vec![TypeSubtyping::default()],
            ..Default::default()
        };
        // module_b: the SAME self-referencing shape, but at flat index 3,
        // preceded by three unrelated plain `i32 -> i32` singleton types --
        // no shared numbering with module_a at all.
        let padding = FuncType { params: vec![ValueType::I32], results: vec![ValueType::I32] };
        let module_b = WasmModule {
            types: vec![
                padding.clone(),
                padding.clone(),
                padding,
                FuncType { params: vec![ValueType::ConcreteFuncRef(3)], results: vec![] },
            ],
            type_subtyping: vec![TypeSubtyping::default(); 4],
            ..Default::default()
        };
        let canonical_a = canonicalize_types(&module_a);
        let canonical_b = canonicalize_types(&module_b);
        assert_eq!(canonical_a[0], canonical_b[3]);
        // Sanity: the padding types (plain i32->i32, no self-reference) are
        // themselves canonicalized too, and are NOT equal to the
        // self-referencing shape.
        assert_ne!(canonical_b[0], canonical_b[3]);
    }

    /// Security review finding (W34 third slice): a real, empirically-
    /// confirmed algorithmic-complexity DoS -- two SEPARATELY-declared,
    /// byte-identical (but nominally unrelated) groups WITHIN ONE module
    /// used to get their own separate `Rc` allocation from `canonicalize_
    /// types`, so every later `canonical_types_equivalent` call comparing
    /// them (reachable per-instruction via `is_assignable`/`call_indirect_
    /// type_matches` once this slice wired canonical equivalence into
    /// real decision points) paid a full recursive structural walk EVERY
    /// time, with no caching across calls -- a crafted module with two
    /// near-`MAX_CANONICAL_TREE_WEIGHT`-sized identical groups referenced
    /// repeatedly from one function body reproducibly took the security
    /// review's own sub-agent over a minute to validate (~100,000x
    /// slower than an equal-sized module that never triggers the deep
    /// comparison). Fixed by interning: `canonicalize_types` now
    /// deduplicates content-identical groups within ONE call into a
    /// single shared `Rc` allocation, and `canonical_types_equivalent`
    /// tries `Rc::ptr_eq` first -- turning the COMMON, actually-reachable
    /// within-module case into a real O(1) check, matching MVP.md's own
    /// "constant-time" comparison promise instead of merely gesturing at
    /// it. This test proves the mechanism directly: two independently-
    /// declared, differently-indexed, byte-identical multi-member `rec`
    /// groups (deliberately NOT the trivial "same index" case, and with
    /// NO declared `sub` relationship at all) canonicalize to the exact
    /// SAME `Rc` allocation (`Rc::ptr_eq`, not merely `==`) once produced
    /// by the SAME `canonicalize_types` call.
    #[test]
    fn identical_groups_within_one_module_intern_to_the_same_rc_allocation() {
        // Two separate 2-member mutual `rec` groups, byte-identical in
        // shape, at different flat-index offsets, no `sub` between them.
        let group = |a: u32, b: u32| {
            vec![
                FuncType { params: vec![ValueType::ConcreteFuncRef(a)], results: vec![] },
                FuncType { params: vec![ValueType::ConcreteFuncRef(b)], results: vec![] },
            ]
        };
        let mut types = group(1, 0); // group G: indices 0,1 (mutually referencing)
        types.extend(group(3, 2)); // group H: indices 2,3 (identical shape, own offset)
        let m = WasmModule {
            types,
            type_subtyping: vec![
                TypeSubtyping { rec_group_size: 2, rec_group_position: 0, ..Default::default() },
                TypeSubtyping { rec_group_size: 2, rec_group_position: 1, ..Default::default() },
                TypeSubtyping { rec_group_size: 2, rec_group_position: 0, ..Default::default() },
                TypeSubtyping { rec_group_size: 2, rec_group_position: 1, ..Default::default() },
            ],
            ..Default::default()
        };
        let canonical = canonicalize_types(&m);
        let (g_rc, g_pos) = canonical[0].as_ref().expect("group G must canonicalize");
        let (h_rc, h_pos) = canonical[2].as_ref().expect("group H must canonicalize");
        // Structurally equal (the pre-existing, always-correct property)...
        assert_eq!((g_rc, g_pos), (h_rc, h_pos));
        // ...AND literally the same allocation now (the fix): interning
        // means comparing G vs H never needs a fresh structural walk again.
        assert!(Rc::ptr_eq(g_rc, h_rc), "identical groups produced by the same canonicalize_types call must share one Rc allocation");
        // The shared allocation makes canonical_types_equivalent's ptr_eq
        // fast path fire for this exact pair, positions matching too.
        assert!(canonical_types_equivalent(&canonical, 0, 2));
        assert!(canonical_types_equivalent(&canonical, 1, 3));
        // Different positions within the (now-shared) group are still
        // correctly NOT equivalent to each other.
        assert!(!canonical_types_equivalent(&canonical, 0, 3));
    }

    /// Two singleton groups with genuinely different shapes (one
    /// self-referencing, one a plain `i32 -> i32`) must NOT canonicalize
    /// equal -- a canonicalizer that's too permissive (e.g. one that
    /// ignores the operand entirely) would silently defeat the whole
    /// mechanism's purpose.
    #[test]
    fn genuinely_different_shapes_do_not_canonicalize_equal() {
        let self_ref = WasmModule {
            types: vec![FuncType { params: vec![ValueType::ConcreteFuncRef(0)], results: vec![] }],
            type_subtyping: vec![TypeSubtyping::default()],
            ..Default::default()
        };
        let plain = WasmModule {
            types: vec![FuncType { params: vec![ValueType::I32], results: vec![] }],
            type_subtyping: vec![TypeSubtyping::default()],
            ..Default::default()
        };
        assert_ne!(canonicalize_types(&self_ref)[0], canonicalize_types(&plain)[0]);
    }

    /// `type-equivalence.wast` lines 6-7: `$t1 (func (param f32 f32)
    /// (result f32))` vs `$t2 (func (param $x f32) (param $y f32) (result
    /// f32))` -- identical bodies, differing only in whether params carry a
    /// (this crate doesn't even model) name. `ValueType` never carries
    /// parameter names to begin with, so this is a direct proof that the
    /// representation "throws away irrelevant syntax" the spec's own
    /// slice-1 corpus citation calls for.
    #[test]
    fn identical_bodies_canonicalize_equal_regardless_of_declared_param_names() {
        // Both `$t1`/`$t2` collapse to the same `FuncType { params: [F32,
        // F32], results: [F32] }` by the time they reach `wasm_types` --
        // parameter names are a `wasm-wast-parser`-only, symbol-table-only
        // concept. Two SEPARATE singleton types with that identical body
        // must canonicalize equal.
        let m = WasmModule {
            types: vec![
                FuncType { params: vec![ValueType::F32, ValueType::F32], results: vec![ValueType::F32] },
                FuncType { params: vec![ValueType::F32, ValueType::F32], results: vec![ValueType::F32] },
            ],
            type_subtyping: vec![TypeSubtyping::default(); 2],
            ..Default::default()
        };
        let canonical = canonicalize_types(&m);
        assert_eq!(canonical[0], canonical[1]);
    }

    /// `type-equivalence.wast`'s "Indirect types" module: a chain of
    /// non-self-referencing singleton groups (`$s0`, `$s1` referencing
    /// `$s0`). Two independently-built chains with isomorphic shapes but
    /// no shared numbering must canonicalize equal at every step of the
    /// chain, proving `Outer` embedding (not just `Rec` self-reference)
    /// works across modules too.
    #[test]
    fn chained_non_self_referencing_singletons_canonicalize_equal_across_modules() {
        fn build(offset: u32) -> WasmModule {
            // `offset` leading unrelated padding types, then:
            //   s0 = (func (param i32) (result f32))
            //   s1 = (func (param i32 (ref s0)) (result (ref s0)))
            let padding = FuncType { params: vec![], results: vec![] };
            let mut types = vec![padding; offset as usize];
            types.push(FuncType { params: vec![ValueType::I32], results: vec![ValueType::F32] });
            let s0 = offset;
            types.push(FuncType {
                params: vec![ValueType::I32, ValueType::ConcreteFuncRef(s0)],
                results: vec![ValueType::ConcreteFuncRef(s0)],
            });
            WasmModule { type_subtyping: vec![TypeSubtyping::default(); types.len()], types, ..Default::default() }
        }
        let module_a = build(0);
        let module_b = build(5);
        let canonical_a = canonicalize_types(&module_a);
        let canonical_b = canonicalize_types(&module_b);
        assert_eq!(canonical_a[0], canonical_b[5]); // s0 == s0
        assert_eq!(canonical_a[1], canonical_b[6]); // s1 == s1
    }

    // ────────────────────────────────────────────────────────────────────
    // W34 second slice: real multi-member `rec`-group De Bruijn numbering
    // ────────────────────────────────────────────────────────────────────

    /// Builds the `TypeSubtyping` entries for a `size`-member group, with
    /// the given per-member `(supertype, is_final)` pairs (`supertype`
    /// indices are ABSOLUTE flat indices, same convention as everywhere
    /// else in this crate -- the group's own start doesn't matter here,
    /// only each member's `rec_group_position` relative to it).
    fn group_subtyping(size: u32, members: &[(Option<u32>, bool)]) -> Vec<TypeSubtyping> {
        assert_eq!(members.len(), size as usize);
        (0..size)
            .map(|off| {
                let (supertype, is_final) = members[off as usize];
                TypeSubtyping { supertype, is_final, rec_group_size: size, rec_group_position: off }
            })
            .collect()
    }

    /// A previously-unresolvable case: a genuine `rec_group_size > 1`
    /// group now DOES canonicalize -- `type-rec.wast`'s own 2-member
    /// mutual pair (lines 15-18): `(rec (type $h (func (param (ref $k))))
    /// (type $k (func (result (ref $h)))))`. `$h` (flat index 0) references
    /// `$k` (flat index 1, the OTHER end of the SAME group) -- group-
    /// relative, so `Rec(1)`, not the module-absolute `1`; `$k` references
    /// `$h` (flat index 0, this group's own start) -- `Rec(0)`.
    #[test]
    fn two_member_mutual_group_ties_with_group_relative_rec_numbering() {
        let m = WasmModule {
            types: vec![
                FuncType { params: vec![ValueType::ConcreteFuncRef(1)], results: vec![] }, // $h
                FuncType { params: vec![], results: vec![ValueType::ConcreteFuncRef(0)] }, // $k
            ],
            type_subtyping: group_subtyping(2, &[(None, true), (None, true)]),
            ..Default::default()
        };
        let canonical = canonicalize_types(&m);
        let (h_group, h_pos) = canonical[0].as_ref().expect("$h must canonicalize");
        let (k_group, k_pos) = canonical[1].as_ref().expect("$k must canonicalize");
        // Both flat indices share the SAME underlying group (the whole
        // point of `Outer`/position-pair identity -- a `rec` group is one
        // tied unit, not two independent ones).
        assert!(Rc::ptr_eq(h_group, k_group));
        assert_eq!(*h_pos, 0);
        assert_eq!(*k_pos, 1);
        assert_eq!(
            h_group.members,
            vec![
                CanonicalSubtype { is_final: true, supertype: None, comp: CanonicalCompType::Func(vec![CanonicalValType::Ref(true, CanonicalHeapRef::Rec(1))], vec![]) },
                CanonicalSubtype { is_final: true, supertype: None, comp: CanonicalCompType::Func(vec![], vec![CanonicalValType::Ref(true, CanonicalHeapRef::Rec(0))]) },
            ]
        );
    }

    /// Two SEPARATELY-declared multi-member groups, at completely
    /// different flat indices in two different modules, with the SAME
    /// shape and the SAME internal reference wiring, must canonicalize to
    /// byte-identical forms -- cross-module comparability (MVP.md's "no
    /// shared numbering needed" promise), now proven for a real
    /// multi-member group rather than just a singleton.
    #[test]
    fn two_independently_indexed_isomorphic_multi_member_groups_canonicalize_identically() {
        fn build(offset: u32) -> WasmModule {
            let padding = FuncType { params: vec![], results: vec![] };
            let mut types = vec![padding; offset as usize];
            types.push(FuncType { params: vec![ValueType::ConcreteFuncRef(offset + 1)], results: vec![] });
            types.push(FuncType { params: vec![], results: vec![ValueType::ConcreteFuncRef(offset)] });
            let mut type_subtyping = vec![TypeSubtyping::default(); offset as usize];
            type_subtyping.extend(group_subtyping(2, &[(None, true), (None, true)]));
            WasmModule { types, type_subtyping, ..Default::default() }
        }
        let module_a = build(0);
        let module_b = build(4);
        let canonical_a = canonicalize_types(&module_a);
        let canonical_b = canonicalize_types(&module_b);
        assert_eq!(canonical_a[0], canonical_b[4]);
        assert_eq!(canonical_a[1], canonical_b[5]);
    }

    /// W34 fourth slice (`code/specs/W34-wasm-gc-canonical-type-equivalence.md`):
    /// [`canonical_type_entries_equivalent`] is the two-DIFFERENT-tables
    /// shape [`wasm-runtime`]'s cross-module import check actually needs
    /// (as opposed to [`canonical_types_equivalent`]'s own two-indices-
    /// into-ONE-table shape, correct only within a single module) --
    /// proven directly here at the narrowest possible layer, before any
    /// `wasm-runtime`/`wasm-conformance` wiring, matching this campaign's
    /// own "verify the mechanism before the plumbing" discipline. Reuses
    /// the SAME two isomorphic-but-differently-indexed modules the
    /// preceding test already proved `canonicalize_types` handles
    /// correctly -- this test's own job is only to prove the COMPARISON
    /// function itself (not `canonicalize_types`) is genuinely usable
    /// across two independent tables with no shared numbering.
    #[test]
    fn canonical_type_entries_equivalent_compares_across_two_independent_tables() {
        fn build(offset: u32) -> WasmModule {
            let padding = FuncType { params: vec![], results: vec![] };
            let mut types = vec![padding; offset as usize];
            types.push(FuncType { params: vec![ValueType::ConcreteFuncRef(offset + 1)], results: vec![] });
            types.push(FuncType { params: vec![], results: vec![ValueType::ConcreteFuncRef(offset)] });
            let mut type_subtyping = vec![TypeSubtyping::default(); offset as usize];
            type_subtyping.extend(group_subtyping(2, &[(None, true), (None, true)]));
            WasmModule { types, type_subtyping, ..Default::default() }
        }
        let canonical_a = canonicalize_types(&build(0));
        let canonical_b = canonicalize_types(&build(4));
        // Isomorphic entries, from two SEPARATE `canonicalize_types` calls
        // (so interning cannot have unified them into one `Rc` allocation
        // -- this exercises the full structural `==` fallback path, not
        // just the same-call `Rc::ptr_eq` fast path).
        assert!(canonical_type_entries_equivalent(canonical_a[0].as_ref(), canonical_b[4].as_ref()));
        assert!(canonical_type_entries_equivalent(canonical_a[1].as_ref(), canonical_b[5].as_ref()));
        // A genuinely different position within the SAME isomorphic group
        // is correctly NOT equivalent.
        assert!(!canonical_type_entries_equivalent(canonical_a[0].as_ref(), canonical_b[5].as_ref()));
        // `None` on either side is conservatively `false`, never a wrong
        // `true`.
        assert!(!canonical_type_entries_equivalent(None, canonical_b[4].as_ref()));
        assert!(!canonical_type_entries_equivalent(canonical_a[0].as_ref(), None));
        assert!(!canonical_type_entries_equivalent(None, None));
    }

    /// W34 fourth slice: [`canonical_chain_reaches`] is the cross-module
    /// counterpart to [`nominal_subtype_chain`]'s own termination check --
    /// proving directly (not just via `wasm-runtime`'s own end-to-end
    /// linking tests) that climbing ONE module's own local `sub` chain
    /// past the reflexive start reaches a target from a DIFFERENT module's
    /// own canonical-type table. Mirrors `type-subtyping.wast`'s own `M6`/
    /// `M7` "Linking" cases: an export declared `(sub $parent (func))` is
    /// importable at its own `$parent` type, not only at its own exact
    /// type.
    #[test]
    fn canonical_chain_reaches_climbs_one_modules_own_chain_to_match_an_external_target() {
        // Exporting module: $parent (idx0, no supertype, open), $child
        // (idx1, sub $parent, open) -- both empty `(func)` bodies.
        let empty_func = FuncType { params: vec![], results: vec![] };
        let exporter = WasmModule {
            types: vec![empty_func.clone(), empty_func.clone()],
            type_subtyping: vec![TypeSubtyping { supertype: None, is_final: false, ..Default::default() }, TypeSubtyping { supertype: Some(0), is_final: false, ..Default::default() }],
            ..Default::default()
        };
        // Importing module declares ITS OWN, differently-indexed copy of
        // just $parent's shape (idx0 here, after a padding type at idx...
        // actually no padding needed: the point is this table is entirely
        // SEPARATE from the exporter's, indices are irrelevant to compare).
        let importer = WasmModule {
            types: vec![empty_func],
            type_subtyping: vec![TypeSubtyping { supertype: None, is_final: false, ..Default::default() }],
            ..Default::default()
        };
        let exporter_canonical = canonicalize_types(&exporter);
        let importer_canonical = canonicalize_types(&importer);
        let target = importer_canonical[0].as_ref().expect("importer's $parent must canonicalize");

        // Climbing from $child (idx1) reaches $parent (idx0) after one hop,
        // which IS canonically equivalent to the importer's own $parent.
        assert!(canonical_chain_reaches(&exporter.type_subtyping, &exporter_canonical, 1, Some(target)));
        // The reflexive case (climbing from $parent itself) also matches,
        // with zero hops.
        assert!(canonical_chain_reaches(&exporter.type_subtyping, &exporter_canonical, 0, Some(target)));
        // `None` target is conservatively `false`.
        assert!(!canonical_chain_reaches(&exporter.type_subtyping, &exporter_canonical, 0, None));
    }

    /// The negative counterpart: a target with NO ancestor anywhere in the
    /// chain (a genuinely unrelated type) must correctly report `false`,
    /// not accidentally match via the hop walk running past the real
    /// ancestor.
    #[test]
    fn canonical_chain_reaches_does_not_match_an_unrelated_target() {
        let empty_func = FuncType { params: vec![], results: vec![] };
        let one_param_func = FuncType { params: vec![ValueType::I32], results: vec![] };
        let exporter = WasmModule {
            types: vec![empty_func.clone(), empty_func],
            type_subtyping: vec![TypeSubtyping { supertype: None, is_final: false, ..Default::default() }, TypeSubtyping { supertype: Some(0), is_final: false, ..Default::default() }],
            ..Default::default()
        };
        let unrelated = WasmModule { types: vec![one_param_func], ..Default::default() };
        let exporter_canonical = canonicalize_types(&exporter);
        let unrelated_canonical = canonicalize_types(&unrelated);
        let target = unrelated_canonical[0].as_ref().expect("unrelated type must canonicalize");
        assert!(!canonical_chain_reaches(&exporter.type_subtyping, &exporter_canonical, 1, Some(target)));
    }

    /// Two multi-member groups with the SAME member count (2) but a
    /// DIFFERENT internal reference pattern must NOT canonicalize equal --
    /// this is the actual point of group-relative numbering, not just
    /// "same shape, ignore the wiring." Group A is the alternating 2-cycle
    /// above (`$h` -> `$k`, `$k` -> `$h`); group B has BOTH members
    /// reference the SAME sibling (`$p` -> `$p` itself, `$q` -> `$p`) --
    /// same member count, same total reference count, genuinely different
    /// wiring.
    #[test]
    fn same_member_count_but_different_wiring_does_not_canonicalize_equal() {
        let alternating = WasmModule {
            types: vec![
                FuncType { params: vec![ValueType::ConcreteFuncRef(1)], results: vec![] },
                FuncType { params: vec![ValueType::ConcreteFuncRef(0)], results: vec![] },
            ],
            type_subtyping: group_subtyping(2, &[(None, true), (None, true)]),
            ..Default::default()
        };
        let both_point_at_first = WasmModule {
            types: vec![
                FuncType { params: vec![ValueType::ConcreteFuncRef(0)], results: vec![] }, // $p -> $p (Rec(0))
                FuncType { params: vec![ValueType::ConcreteFuncRef(0)], results: vec![] }, // $q -> $p (Rec(0))
            ],
            type_subtyping: group_subtyping(2, &[(None, true), (None, true)]),
            ..Default::default()
        };
        let canonical_alt = canonicalize_types(&alternating);
        let canonical_both = canonicalize_types(&both_point_at_first);
        assert_ne!(canonical_alt[0], canonical_both[0]);
        assert_ne!(canonical_alt[1], canonical_both[1]);
        // Sanity: the two members WITHIN `both_point_at_first` also differ
        // from each other (`$p` self-references, `$q` doesn't) -- confirms
        // the mismatch isn't an artifact of comparing the wrong indices.
        assert_ne!(canonical_both[0], canonical_both[1]);
    }

    /// Composition: a group referencing an EARLIER multi-member group
    /// (`Outer`) whose OWN internal numbering is multi-member `Rec`. A
    /// singleton `$caller` (flat index 2) references `$h` (flat index 0,
    /// position 0 of the earlier 2-member `$h`/`$k` group) -- must tie to
    /// `Outer(<the $h/$k group>, 0)`, embedding the WHOLE 2-member group,
    /// not just a copy of `$h` alone.
    #[test]
    fn a_later_type_referencing_an_earlier_multi_member_group_composes_outer_with_multi_rec() {
        let m = WasmModule {
            types: vec![
                FuncType { params: vec![ValueType::ConcreteFuncRef(1)], results: vec![] }, // $h -> $k
                FuncType { params: vec![], results: vec![ValueType::ConcreteFuncRef(0)] }, // $k -> $h
                FuncType { params: vec![ValueType::ConcreteFuncRef(0)], results: vec![] }, // $caller -> $h
            ],
            type_subtyping: {
                let mut ts = group_subtyping(2, &[(None, true), (None, true)]);
                ts.push(TypeSubtyping::default());
                ts
            },
            ..Default::default()
        };
        let canonical = canonicalize_types(&m);
        let (hk_group, _) = canonical[0].as_ref().unwrap();
        let (caller_group, caller_pos) = canonical[2].as_ref().expect("$caller must canonicalize");
        assert_eq!(*caller_pos, 0);
        assert_eq!(caller_group.members.len(), 1);
        match &caller_group.members[0].comp {
            CanonicalCompType::Func(params, _) => match &params[0] {
                CanonicalValType::Ref(true, CanonicalHeapRef::Outer(embedded, pos)) => {
                    assert_eq!(*pos, 0);
                    // The embedded group is the WHOLE $h/$k group (2
                    // members), byte-identical to it -- not a partial or
                    // re-derived copy.
                    assert_eq!(**embedded, **hk_group);
                    assert_eq!(embedded.members.len(), 2);
                }
                other => panic!("expected an Outer(2-member group, 0) reference, got {other:?}"),
            },
            other => panic!("expected a Func comp type, got {other:?}"),
        }
    }

    /// Composition the other direction: a LATER multi-member group whose
    /// members mix an intra-group `Rec` reference with an `Outer`
    /// reference into an EARLIER multi-member group, within the SAME
    /// member. `$c` (flat index 2, group 2) references both `$b` (flat
    /// index 1, group 1's second member -- `Outer(group1, 1)`) and `$c`
    /// itself (flat index 2, in-group -- `Rec(0)`); `$d` (flat index 3) is
    /// a plain, non-referencing sibling.
    #[test]
    fn a_later_multi_member_group_mixes_outer_and_rec_within_one_member() {
        let m = WasmModule {
            types: vec![
                FuncType { params: vec![], results: vec![] },                                                    // $a (group1[0])
                FuncType { params: vec![ValueType::ConcreteFuncRef(0)], results: vec![] },                        // $b (group1[1]) -> $a
                FuncType { params: vec![ValueType::ConcreteFuncRef(1), ValueType::ConcreteFuncRef(2)], results: vec![] }, // $c (group2[0]) -> $b, $c
                FuncType { params: vec![], results: vec![] },                                                    // $d (group2[1])
            ],
            type_subtyping: {
                let mut ts = group_subtyping(2, &[(None, true), (None, true)]);
                ts.extend(group_subtyping(2, &[(None, true), (None, true)]));
                ts
            },
            ..Default::default()
        };
        let canonical = canonicalize_types(&m);
        let (group1, _) = canonical[0].as_ref().unwrap();
        let (group2, c_pos) = canonical[2].as_ref().expect("$c must canonicalize");
        assert_eq!(*c_pos, 0);
        match &group2.members[0].comp {
            CanonicalCompType::Func(params, _) => {
                match &params[0] {
                    CanonicalValType::Ref(true, CanonicalHeapRef::Outer(embedded, 1)) => {
                        assert_eq!(**embedded, **group1);
                    }
                    other => panic!("expected Outer(group1, 1), got {other:?}"),
                }
                match &params[1] {
                    CanonicalValType::Ref(true, CanonicalHeapRef::Rec(0)) => {}
                    other => panic!("expected Rec(0) (self-reference within group2), got {other:?}"),
                }
            }
            other => panic!("expected a Func comp type, got {other:?}"),
        }
    }

    /// The W33/W34 addenda's own worked "3-cycle" example
    /// (`type-subtyping.wast` lines 68-87, re-verified fresh against the
    /// vendored corpus file): a 3-member group where each member's body
    /// references a DIFFERENT sibling (`$t1` -> `$t3`, `$t2` -> `$t2`
    /// itself, `$t3` -> `$t1`), AND a declared `sub` chain threading
    /// through the same group (`$t3 <: $t2 <: $t1`, `$t1` itself
    /// declaring no supertype). Wiring this into `nominal_subtype_chain`'s
    /// own termination check is slice 3's job (this slice does not touch
    /// `is_assignable`/`nominal_subtype_chain` at all) -- what this test
    /// confirms is that the canonical FORMS themselves, including the
    /// supertype links, tie correctly for this exact corpus example.
    #[test]
    fn the_three_cycle_worked_example_canonicalizes_correctly() {
        let m = WasmModule {
            types: vec![
                FuncType { params: vec![ValueType::I32, ValueType::ConcreteFuncRef(2)], results: vec![] }, // $t1 -> $t3
                FuncType { params: vec![ValueType::I32, ValueType::ConcreteFuncRef(1)], results: vec![] }, // $t2 -> $t2
                FuncType { params: vec![ValueType::I32, ValueType::ConcreteFuncRef(0)], results: vec![] }, // $t3 -> $t1
            ],
            type_subtyping: group_subtyping(3, &[(None, false), (Some(0), false), (Some(1), false)]),
            ..Default::default()
        };
        let canonical = canonicalize_types(&m);
        let (group, _) = canonical[0].as_ref().expect("the 3-cycle must canonicalize");
        assert_eq!(group.members.len(), 3);
        assert_eq!(
            group.members,
            vec![
                CanonicalSubtype {
                    is_final: false,
                    supertype: None,
                    comp: CanonicalCompType::Func(vec![CanonicalValType::I32, CanonicalValType::Ref(true, CanonicalHeapRef::Rec(2))], vec![]),
                },
                CanonicalSubtype {
                    is_final: false,
                    supertype: Some(CanonicalHeapRef::Rec(0)),
                    comp: CanonicalCompType::Func(vec![CanonicalValType::I32, CanonicalValType::Ref(true, CanonicalHeapRef::Rec(1))], vec![]),
                },
                CanonicalSubtype {
                    is_final: false,
                    supertype: Some(CanonicalHeapRef::Rec(1)),
                    comp: CanonicalCompType::Func(vec![CanonicalValType::I32, CanonicalValType::Ref(true, CanonicalHeapRef::Rec(0))], vec![]),
                },
            ]
        );
        // All three flat indices share the identical `Rc` allocation
        // (one tied group, three positions into it).
        assert_eq!(canonical[0].as_ref().unwrap().0, canonical[1].as_ref().unwrap().0);
        assert_eq!(canonical[0].as_ref().unwrap().0, canonical[2].as_ref().unwrap().0);
        assert_eq!(canonical[0].as_ref().unwrap().1, 0);
        assert_eq!(canonical[1].as_ref().unwrap().1, 1);
        assert_eq!(canonical[2].as_ref().unwrap().1, 2);
    }

    /// `type-canon.wast`'s own second module (5-member group, `$t0..$t4`,
    /// several members referencing more than one sibling each) -- a real
    /// corpus fixture, not a hand-simplified one, exercised as a smoke +
    /// correctness test: it must canonicalize (not `None`), and two
    /// members with genuinely different bodies must not collide.
    #[test]
    fn type_canon_wast_five_member_group_canonicalizes() {
        // (rec
        //   (type $t0 (func (param i32 (ref $t2) (ref $t3))))
        //   (type $t1 (func (param i32 (ref $t0) i32 (ref $t4))))
        //   (type $t2 (func (param i32 (ref $t2) (ref $t1))))
        //   (type $t3 (func (param i32 (ref $t2) i32 (ref $t4))))
        //   (type $t4 (func (param (ref $t0) (ref $t2))))
        // )
        use ValueType::{ConcreteFuncRef as R, I32};
        let m = WasmModule {
            types: vec![
                FuncType { params: vec![I32, R(2), R(3)], results: vec![] },
                FuncType { params: vec![I32, R(0), I32, R(4)], results: vec![] },
                FuncType { params: vec![I32, R(2), R(1)], results: vec![] },
                FuncType { params: vec![I32, R(2), I32, R(4)], results: vec![] },
                FuncType { params: vec![R(0), R(2)], results: vec![] },
            ],
            type_subtyping: group_subtyping(5, &[(None, true); 5]),
            ..Default::default()
        };
        let canonical = canonicalize_types(&m);
        for (i, entry) in canonical.iter().enumerate() {
            assert!(entry.is_some(), "member {i} of type-canon.wast's 5-member group must canonicalize");
        }
        // $t0 and $t2 have genuinely different bodies (different param
        // counts/wiring) and must not collide.
        assert_ne!(canonical[0], canonical[2]);
    }

    /// Defensive/security: a `rec_group_size > 1` claim that ISN'T
    /// internally consistent (here, the two members disagree about the
    /// group's own size) must canonicalize to `None` at every position it
    /// touches, never panic, and never silently guess which member's
    /// claim to believe. This replaces the W34 first slice's "multi-member
    /// groups are always `None`" test, updated for the second slice's real
    /// reality: a CONSISTENT multi-member group now canonicalizes fine
    /// (see the tests above); only a genuinely malformed one still can't.
    #[test]
    fn inconsistent_multi_member_group_metadata_canonicalizes_to_none_without_panicking() {
        let m = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }, FuncType { params: vec![], results: vec![] }],
            type_subtyping: vec![
                TypeSubtyping { rec_group_size: 2, rec_group_position: 0, ..Default::default() },
                // Disagrees with index 0's own claimed group size.
                TypeSubtyping { rec_group_size: 3, rec_group_position: 1, ..Default::default() },
            ],
            ..Default::default()
        };
        let canonical = canonicalize_types(&m);
        assert_eq!(canonical, vec![None, None]);
    }

    /// A type referencing a genuinely-failed (inconsistent-metadata)
    /// multi-member group can't tie either -- the failure propagates, it
    /// never gets silently skipped over.
    #[test]
    fn a_type_referencing_an_inconsistent_multi_member_group_is_also_none() {
        let m = WasmModule {
            types: vec![
                FuncType { params: vec![], results: vec![] },
                FuncType { params: vec![], results: vec![] },
                FuncType { params: vec![ValueType::ConcreteFuncRef(0)], results: vec![] },
            ],
            type_subtyping: vec![
                TypeSubtyping { rec_group_size: 2, rec_group_position: 0, ..Default::default() },
                TypeSubtyping { rec_group_size: 3, rec_group_position: 1, ..Default::default() },
                TypeSubtyping::default(),
            ],
            ..Default::default()
        };
        let canonical = canonicalize_types(&m);
        assert_eq!(canonical[0], None);
        assert_eq!(canonical[1], None);
        assert_eq!(canonical[2], None);
    }

    /// Security review concern distinct from stack depth (see
    /// `CanonicalCost`'s own doc comment): a chain of groups where each
    /// level references the one immediately before it from TWO sibling
    /// positions at once doubles `weight` at every level, while `depth`
    /// only grows by 1 -- so a naive bound on `depth` alone would let this
    /// through even though a full structural `PartialEq`/`Hash`/`Drop`
    /// traversal of the resulting (memory-cheap, thanks to `Rc` sharing)
    /// tree would need to visit an EXPONENTIAL number of nodes. This test
    /// builds such a chain far past where `2^level` would exceed
    /// `MAX_CANONICAL_TREE_WEIGHT`, and confirms (a) early levels (still
    /// within the weight budget) canonicalize normally, (b) levels past
    /// the point where doubling exceeds the cap become `None` rather than
    /// an ever-larger tree, and (c) this all completes fast and drops
    /// cleanly -- if the weight cap regressed to "unbounded" (or to
    /// tracking `depth` alone), this test would hang or take
    /// astronomically long rather than fail quickly.
    #[test]
    fn outer_embedding_weight_is_capped_for_branching_reference_chains() {
        // level 0: plain, no references (weight 1).
        // level i (i >= 1): func(param (ref level[i-1]) (ref level[i-1])).
        // weight(level[i]) ~ 2 * weight(level[i-1]) + O(1), so weight
        // roughly doubles every level -- `MAX_CANONICAL_TREE_WEIGHT` is
        // 1_000_000, comfortably exceeded well before level 25.
        let levels = 40usize;
        let mut types = Vec::with_capacity(levels);
        types.push(FuncType { params: vec![], results: vec![] });
        for i in 1..levels {
            let prev = (i - 1) as u32;
            types.push(FuncType { params: vec![ValueType::ConcreteFuncRef(prev), ValueType::ConcreteFuncRef(prev)], results: vec![] });
        }
        let type_subtyping = vec![TypeSubtyping::default(); levels];
        let m = WasmModule { types, type_subtyping, ..Default::default() };

        let canonical = canonicalize_types(&m); // must return promptly, not hang or blow up memory
        assert_eq!(canonical.len(), levels);
        // Early levels, well within the weight budget, still canonicalize.
        assert!(canonical[0].is_some());
        assert!(canonical[1].is_some());
        assert!(canonical[5].is_some());
        // By the last level, doubling 39 times from a base weight of 1
        // (2^39, astronomically past 1_000_000) must have been rejected
        // somewhere along the chain, so it and everything after the
        // rejection point must be `None`.
        assert!(canonical[levels - 1].is_none(), "a doubling reference chain must stop canonicalizing once total weight exceeds the cap, not keep branching forever");
        // `canonical` (holding whatever `Rc<CanonicalGroup>` chain was
        // built up to the rejection point) drops cleanly here.
    }

    /// Declared `sub`/finality metadata is part of a type's real canonical
    /// identity (MVP.md: "their finality flag matches") -- two otherwise
    /// byte-identical bodies with different `is_final` must NOT
    /// canonicalize equal, and a declared supertype must tie the same way
    /// the body does.
    #[test]
    fn finality_and_declared_supertype_are_part_of_canonical_identity() {
        let base = FuncType { params: vec![], results: vec![] };
        let child = FuncType { params: vec![ValueType::NonNullConcreteFuncRef(0)], results: vec![] };
        let open = WasmModule {
            types: vec![base.clone(), child.clone()],
            type_subtyping: vec![
                TypeSubtyping { is_final: false, ..Default::default() },
                TypeSubtyping { supertype: Some(0), is_final: true, ..Default::default() },
            ],
            ..Default::default()
        };
        let mut final_base = open.clone();
        final_base.type_subtyping[0].is_final = true;

        let canonical_open = canonicalize_types(&open);
        let canonical_final = canonicalize_types(&final_base);
        // The supertype (index 0) differs only in `is_final` -- must not
        // canonicalize equal.
        assert_ne!(canonical_open[0], canonical_final[0]);
        // The child (index 1) embeds index 0 wholesale via `Outer` -- since
        // index 0's OWN canonical form differs between the two modules,
        // the child's canonical form (which contains it) must differ too.
        assert_ne!(canonical_open[1], canonical_final[1]);
        // The child's own declared supertype really did tie to an `Outer`
        // reference, not get dropped.
        let (child_group, _) = canonical_open[1].as_ref().unwrap();
        assert!(matches!(child_group.members[0].supertype, Some(CanonicalHeapRef::Outer(_, 0))));
    }

    /// Defensive/security: a malformed module (out-of-range supertype
    /// index) canonicalizes that one entry to `None` rather than panicking
    /// -- `canonicalize_types` never assumes its input was validated.
    #[test]
    fn out_of_range_supertype_canonicalizes_to_none_without_panicking() {
        let m = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            type_subtyping: vec![TypeSubtyping { supertype: Some(999), is_final: false, ..Default::default() }],
            ..Default::default()
        };
        assert_eq!(canonicalize_types(&m), vec![None]);
    }

    /// Defensive/security: a self-referential declared supertype (`(sub
    /// $self (func))`, nonsensical and rejected elsewhere by
    /// `wasm-validator`'s acyclicity check, but `canonicalize_types` itself
    /// must never assume that check already ran) ties to `Rec(0)` rather
    /// than looping or panicking -- there is no recursion in this
    /// function at all, so this can't loop regardless.
    #[test]
    fn self_referential_supertype_ties_to_rec_zero_without_looping() {
        let m = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            type_subtyping: vec![TypeSubtyping { supertype: Some(0), is_final: false, ..Default::default() }],
            ..Default::default()
        };
        let canonical = canonicalize_types(&m);
        let (group, _) = canonical[0].as_ref().expect("self-referential supertype still resolves");
        assert_eq!(group.members[0].supertype, Some(CanonicalHeapRef::Rec(0)));
    }

    /// A module with struct/array composite kinds (W33 fourth slice)
    /// canonicalizes those bodies too, not just `FuncType` ones --
    /// canonical equivalence is not a func-only concept.
    #[test]
    fn struct_and_array_bodies_canonicalize_and_compare_by_content() {
        let struct_ty = StructType { fields: vec![FieldType::plain(ValueType::I32, true), FieldType { storage: StorageType::I8, mutable: false }] };
        let array_ty = ArrayType { element: FieldType::plain(ValueType::F64, false) };
        let m = WasmModule {
            types: vec![
                FuncType { params: vec![], results: vec![] }, // dummy slot for the struct
                FuncType { params: vec![], results: vec![] }, // dummy slot for the array
            ],
            type_kinds: vec![TypeKind::Struct(0), TypeKind::Array(0)],
            struct_types: vec![struct_ty.clone()],
            array_types: vec![array_ty.clone()],
            type_subtyping: vec![TypeSubtyping::default(); 2],
            ..Default::default()
        };
        let canonical = canonicalize_types(&m);
        let (struct_group, _) = canonical[0].as_ref().expect("struct body canonicalizes");
        assert_eq!(
            struct_group.members[0].comp,
            CanonicalCompType::Struct(vec![
                CanonicalFieldType { storage: CanonicalStorageType::Val(CanonicalValType::I32), mutable: true },
                CanonicalFieldType { storage: CanonicalStorageType::I8, mutable: false },
            ])
        );
        let (array_group, _) = canonical[1].as_ref().expect("array body canonicalizes");
        assert_eq!(
            array_group.members[0].comp,
            CanonicalCompType::Array(CanonicalFieldType { storage: CanonicalStorageType::Val(CanonicalValType::F64), mutable: false })
        );
    }

    /// Security review finding (W34 first slice): a long CHAIN of singleton
    /// groups, each referencing only the immediately preceding one (no
    /// cycle at all -- indices strictly decrease), builds a genuinely
    /// nested `Outer`-embedding tree `N` links deep. An empirical repro
    /// during review confirmed a real process-aborting stack overflow in
    /// the compiler-derived `Drop` glue for such a tree at tens of
    /// thousands of links -- comfortably reachable from a small, realistic
    /// module. `MAX_CANONICAL_OUTER_DEPTH` must cut this off FAR below
    /// that threshold: this test builds a chain well past the cap and
    /// confirms (a) entries within the cap still canonicalize normally,
    /// (b) every entry beyond the cap is `None` rather than an
    /// ever-deeper tree, and (c) dropping the whole result (implicitly, at
    /// the end of this test) does not crash -- if the cap regressed to
    /// "unbounded" this test would be the one to catch it, and its own
    /// chain length (a few thousand) is deliberately far short of the
    /// tens-of-thousands threshold that actually crashes an unbounded
    /// build, so it stays fast and reliable as a regression guard rather
    /// than a slow stress test.
    #[test]
    fn outer_embedding_depth_is_capped_and_a_long_chain_does_not_crash() {
        let chain_len = (MAX_CANONICAL_OUTER_DEPTH as usize) + 50;
        let mut types = Vec::with_capacity(chain_len);
        types.push(FuncType { params: vec![], results: vec![] });
        for i in 1..chain_len {
            types.push(FuncType { params: vec![ValueType::ConcreteFuncRef((i - 1) as u32)], results: vec![] });
        }
        let type_subtyping = vec![TypeSubtyping::default(); chain_len];
        let m = WasmModule { types, type_subtyping, ..Default::default() };

        let canonical = canonicalize_types(&m); // must not crash to reach this line at all
        assert_eq!(canonical.len(), chain_len);

        // The root of the chain (depth 0) and everything within the cap
        // must still canonicalize -- the cap must not be so aggressive it
        // rejects ordinary, well-within-bounds chains.
        assert!(canonical[0].is_some());
        assert!(canonical[MAX_CANONICAL_OUTER_DEPTH as usize - 1].is_some());
        // Somewhere past the cap, entries must start reporting `None`
        // rather than building an ever-deeper tree.
        assert!(canonical[chain_len - 1].is_none(), "a chain past the depth cap must stop canonicalizing, not keep nesting forever");
        // `canonical` (holding potentially-deep `Rc<CanonicalGroup>` chains
        // up to the cap) is dropped here, at the end of the test -- if the
        // cap regressed to "unbounded" and this chain were long enough to
        // matter, THIS is where a stack overflow would abort the test
        // process rather than report a normal failure.
    }

    /// Every abstract (non-index-carrying) `ValueType` variant this crate
    /// has must canonicalize without panicking and must roundtrip through
    /// the SAME `AbstractHeapKind`/nullability pair every time (no
    /// `resolve_heap_index` call involved at all for these, so this also
    /// exercises the exhaustive match in isolation from the `Rec`/`Outer`
    /// machinery above).
    #[test]
    fn every_abstract_heap_type_canonicalizes_deterministically() {
        let abstracts = [
            ValueType::Anyref,
            ValueType::I31ref,
            ValueType::Funcref,
            ValueType::Externref,
            ValueType::Exnref,
            ValueType::NullFuncref,
            ValueType::NullExternref,
            ValueType::NullExnref,
            ValueType::NullRef,
            ValueType::NonNullArrayAny,
        ];
        for vt in abstracts {
            let m = WasmModule {
                types: vec![FuncType { params: vec![vt], results: vec![] }],
                type_subtyping: vec![TypeSubtyping::default()],
                ..Default::default()
            };
            let once = canonicalize_types(&m);
            let twice = canonicalize_types(&m);
            assert_eq!(once, twice, "canonicalization must be deterministic for {vt:?}");
            assert!(once[0].is_some(), "{vt:?} must canonicalize to Some");
        }
    }
}
