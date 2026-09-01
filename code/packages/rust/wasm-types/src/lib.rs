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
            (ValueType::NonNullStructRef(_), ValueType::Anyref)
                | (ValueType::NonNullConcreteFuncRef(_), ValueType::Funcref)
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldType {
    /// The type stored in this field.
    pub val_type: ValueType,
    /// Whether this field can be modified after the struct is created.
    /// `true` → mutable (heap write is legal via `struct.set`).
    /// `false` → immutable (write-once, set during `struct.new`).
    pub mutable: bool,
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
    /// Struct type `k` is at type-section index `types.len() + k`.
    /// When this vec is empty, the type section contains only function types
    /// and the encoding is identical to WASM 1.0.
    pub struct_types: Vec<StructType>,

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
        if sub_idx == super_idx {
            return true;
        }
        let mut cur = sub_idx;
        // Security review finding (W33 first slice): this used to bound
        // the walk to `self.types.len()` hops -- correct for
        // TERMINATION (paired with `wasm-validator`'s own
        // `check_type_subtyping_is_acyclic`, a cyclic chain can no
        // longer reach here at all), but NOT for algorithmic complexity.
        // This method is called from `wasm-validator::is_assignable`,
        // which runs at every `pop_expect` call site -- i.e. roughly once
        // per instruction operand, across every function body in a
        // module. A module declaring one very long, entirely spec-legal
        // `sub` chain (N types) plus M call sites checking assignability
        // against a type near the chain's root forces O(N*M) total
        // validation work -- confirmed to scale linearly per query,
        // verified directly rather than assumed. `MAX_SUBTYPE_CHAIN_HOPS`
        // caps the PER-QUERY cost to a constant instead: a chain longer
        // than this bound simply reports "not a nominal subtype" beyond
        // the cutoff (a false negative -- SAFE, since it can only make
        // this method reject something a deeper walk might have
        // accepted, never wrongly accept). The real corpus's own longest
        // chain is a handful of hops; this bound is generous by several
        // orders of magnitude while keeping worst-case per-query cost
        // bounded.
        for _ in 0..Self::MAX_SUBTYPE_CHAIN_HOPS {
            match self.type_subtyping_at(cur).supertype {
                Some(parent) if parent == super_idx => return true,
                Some(parent) => cur = parent,
                None => return false,
            }
        }
        false
    }

    /// The hop cap [`Self::func_type_is_nominal_subtype`] enforces -- see
    /// that method's own doc comment for why a bounded walk (rather than
    /// one scaled to `types.len()`) matters for algorithmic complexity,
    /// not just termination.
    const MAX_SUBTYPE_CHAIN_HOPS: u32 = 1_000;

    /// `(rec_group_size, rec_group_position)` for type `idx` -- see
    /// [`TypeSubtyping::rec_group_position`]'s own doc comment for why
    /// this exists and how `wasm-runtime`'s cross-module import/tag
    /// type-compatibility check uses it.
    pub fn type_group_shape(&self, idx: u32) -> (u32, u32) {
        let st = self.type_subtyping_at(idx);
        (st.rec_group_size, st.rec_group_position)
    }
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
        let f = FieldType { val_type: ValueType::Anyref, mutable: true };
        assert_eq!(f.val_type, ValueType::Anyref);
        assert!(f.mutable);

        let g = FieldType { val_type: ValueType::I32, mutable: false };
        assert_eq!(g.val_type, ValueType::I32);
        assert!(!g.mutable);
    }

    // Test 22: StructType for LispyPair has two anyref fields
    #[test]
    fn struct_type_lispy_pair() {
        let lispy_pair = StructType {
            fields: vec![
                FieldType { val_type: ValueType::Anyref, mutable: true }, // $head
                FieldType { val_type: ValueType::Anyref, mutable: true }, // $tail
            ],
        };
        assert_eq!(lispy_pair.fields.len(), 2);
        assert_eq!(lispy_pair.fields[0].val_type, ValueType::Anyref);
        assert!(lispy_pair.fields[0].mutable);
        assert_eq!(lispy_pair.fields[1].val_type, ValueType::Anyref);
        assert!(lispy_pair.fields[1].mutable);
    }

    // Test 23: WasmModule with struct_types carries the GC definition
    #[test]
    fn wasm_module_with_struct_types() {
        let m = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            struct_types: vec![StructType {
                fields: vec![
                    FieldType { val_type: ValueType::Anyref, mutable: true },
                    FieldType { val_type: ValueType::Anyref, mutable: true },
                ],
            }],
            ..Default::default()
        };
        assert_eq!(m.types.len(), 1);
        assert_eq!(m.struct_types.len(), 1);
        // The struct type index in the type section is types.len() + 0 = 1.
        assert_eq!(m.types.len(), 1);
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
        // structurally identical (that would need canonical equivalence,
        // W33's own explicitly out-of-scope piece).
        assert!(!m.func_type_is_nominal_subtype(0, 1));
        assert!(!m.func_type_is_nominal_subtype(1, 0));
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
        let n = (WasmModule::MAX_SUBTYPE_CHAIN_HOPS + 10) as usize;
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
}
