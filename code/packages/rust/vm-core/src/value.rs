//! [`Value`] — the dynamic value type stored in VM registers.
//!
//! Python's `Any` registry is replaced in Rust by a closed enum.  The seven
//! variants cover every type the standard IIR opcode handlers produce:
//!
//! | Variant | IIR type strings | Notes |
//! |---------|-----------------|-------|
//! | `Int(i64)` | `"u8"` .. `"i64"` | Includes all unsigned widths |
//! | `Float(f64)` | `"f64"`, `"f32"` | Single 64-bit slot |
//! | `Bool(bool)` | `"bool"` | |
//! | `Str(Arc<[u8]>)` | `"str"` | Immutable bytes, reference-counted (not GC-managed) |
//! | `HeapRef(gc_core::HeapRef)` | `"ref<T>"` | A `gc_alloc`'d object on the shared `FlatHeap` collector |
//! | `Symbol(u32)` | none (`dyn_symbol`'s result) | An interned atom: equal iff the ids are (#18470) |
//! | `Null` | `"void"`, none | Default register value; ret_void result; the dynamic nil (`dyn_nil`) |
//!
//! `HeapRef` is the one variant vm-core itself roots precisely at a
//! `safepoint` (see `dispatch::run_safepoint`): every `Value::HeapRef` found
//! in a register, global, memory slot, or array element is a live root into
//! `gc-core`'s `FlatHeap`, the same collector engine the native-AOT backends
//! share via `gc-core-capi`. E6d records/unions/closures/cons cells
//! (`alloc`/`field_store`/`field_load`) allocate here — `alloc` is a direct
//! alias for `gc_alloc`, so every Twig program that builds lisp/record/union
//! data gets a real, collected heap. Only `alloc_array`/`array_get`/
//! `array_set` (E5 arrays) still allocate on `ctx.arrays`, a separate, plain
//! Rust bump arena that is never collected — arrays have no analogous real
//! collector to reuse yet, a deliberate, separate gap, not an oversight.
//!
//! ## A string is bytes (LANG83 PR5b)
//!
//! An IIR `str` is an immutable sequence of bytes with no encoding: `str_len`
//! counts bytes and `str_index` reads one. Until PR5b this VM held a Rust
//! `String`, which must be valid UTF-8, so `str_slice "é", 1, 2` — the lone
//! byte `A9`, a perfectly good IIR string — was refused, and a literal could
//! never hold the byte `FF`. `Str` now holds `Arc<[u8]>`:
//!
//! | need | how |
//! |------|-----|
//! | build one | [`Value::str`] from `&str`, `String`, `&[u8]` or `Vec<u8>` |
//! | read the bytes | [`Value::as_bytes`] |
//! | read UTF-8 text (an embedder printing a name) | [`Value::as_str`], `None` if the bytes are not UTF-8 |
//! | copy a register | `clone()` bumps a count; the bytes are shared, never copied |
//!
//! Sharing is safe because no op mutates a string: `str_concat` and
//! `str_slice` build a fresh one.
//!
//! Why `Arc` and not the `Rc` LANG83 first named: a `Value` must stay `Send`
//! and `Sync`. Embedders record values from inside builtins, which must be
//! `Send + Sync` (`BuiltinRegistry::register`, the JIT's `register_builtin`),
//! for example into an `Arc<Mutex<Vec<Value>>>`; with `Rc` inside, that no
//! longer compiled. A string's count changes only when a register is copied,
//! so the atomic increment is not on any hot path that `Rc` would have saved.
//!
//! ## Dynamic values: nil, booleans and symbols are kinds of their own (#18470, #18493)
//!
//! A dynamically-typed frontend (the Lisp family: Twig today) needs every
//! value to say what *kind* it is at run time: `number?`, `symbol?`,
//! `boolean?`, `null?` and its truthiness rule all ask. Until #18470 two kinds
//! had no spelling here, and borrowed `Int`'s:
//!
//! | value | before | now |
//! |-------|--------|-----|
//! | nil | `Int(0)` — the same value as the integer `0` | `Null` (from `dyn_nil`) |
//! | a symbol `'a` | `Int(2^29 + id)` — an integer in a reserved range | `Symbol(id)` (from `dyn_symbol`) |
//!
//! So `(null? 0)` answered `#t`, `(if 0 1 2)` could not tell `0` from nil, and
//! `(= 'a 536870912)` compared a symbol equal to a perfectly good integer.
//! `Symbol` is the one new variant. It is the LANG83 `Dynamic` family's
//! representation of an interned atom: language-neutral (Ruby, Erlang and
//! JavaScript have the same thing), and carrying only an identity — which
//! *name* an id stands for is the business of whoever interned it
//! ([`crate::builtins`]'s `dyn_symbol`). What a symbol *means* to a language,
//! like everything else here, is the compiler's to emit (LANG83 PR7).
//!
//! Language frontends that need additional value kinds beyond these may
//! extend this in a wrapper enum without modifying vm-core itself.
//!
//! # Arithmetic helpers
//!
//! The `as_i64`, `as_f64`, `as_bool`, and `as_str` extractors return `None`
//! for a type mismatch so the dispatch loop can surface a clean
//! [`VMError::TypeError`] instead of a Rust panic.
//!
//! # Example
//!
//! ```
//! use vm_core::value::Value;
//!
//! let v = Value::Int(42);
//! assert_eq!(v.as_i64(), Some(42));
//! assert!(v.as_f64().is_none());
//!
//! let b = Value::Bool(true);
//! assert!(b.is_truthy());
//! ```

/// The dynamic value stored in a VM register.
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub enum Value {
    /// A signed integer covering all of u8 / u16 / u32 / u64 / i8..i64.
    ///
    /// The VM's `u8_wrap` flag masks the result with `& 0xFF` after every
    /// arithmetic operation when Tetrad 8-bit semantics are required.
    Int(i64),
    /// An IEEE 754 double-precision float.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// An immutable byte string (LANG83 PR5b: bytes, not UTF-8 text).
    /// Build one with [`Value::str`]; read it with [`Value::as_bytes`].
    Str(std::sync::Arc<[u8]>),
    /// A reference to an object allocated with `gc_alloc`, managed by the
    /// shared `FlatHeap` collector (`gc-core`) — traced, reclaimed, and
    /// relocated under compaction. `HeapRef::NULL` (address `0`) represents
    /// the null reference; `is_truthy` and `gc_field_load`/`gc_field_store`
    /// (see `dispatch.rs`) both treat it as such.
    HeapRef(gc_core::HeapRef),
    /// An interned atom (LANG83 PR7f `dyn_symbol`, #18470). Two symbols are
    /// the same value iff their ids are equal; an id is meaningful only inside
    /// the VM that interned it. A symbol is not a number: every numeric
    /// extractor ([`Value::as_i64`], [`Value::as_f64`]) answers `None`, so an
    /// arithmetic or ordering op on one is a clean type error, never a silent
    /// integer. In a GC field it is stored as `(id << 32) | 0b010`, the tagged
    /// word the native backends use for a symbol (`dispatch.rs`, field tags).
    Symbol(u32),
    /// The absence of a value — default register contents and `ret_void` result.
    #[default]
    Null,
}

impl Value {
    // ------------------------------------------------------------------
    // Type extractors
    // ------------------------------------------------------------------

    /// Return the integer value if this is `Value::Int`, else `None`.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            Value::Bool(b) => Some(*b as i64),
            _ => None,
        }
    }

    /// Return the float value if this is `Value::Float`, else `None`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Return the bool value if this is `Value::Bool`, else `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Build a `Value::Str` holding exactly `bytes` (LANG83 PR5b).
    ///
    /// ```
    /// use vm_core::value::Value;
    /// assert_eq!(Value::str("hé").as_bytes(), Some(&b"h\xC3\xA9"[..]));
    /// assert_eq!(Value::str([0xFFu8]).as_bytes(), Some(&[0xFF][..]));
    /// ```
    pub fn str(bytes: impl AsRef<[u8]>) -> Value {
        Value::Str(std::sync::Arc::from(bytes.as_ref()))
    }

    /// Return the string's bytes if this is `Value::Str`, else `None`.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Return the string as UTF-8 text if this is a `Value::Str` whose bytes
    /// are valid UTF-8, else `None`.
    ///
    /// For an embedder that wants text (a symbol name, a diagnostic). A
    /// string's *contents* are bytes: write them with [`Value::as_bytes`],
    /// or a lone `0xA9` disappears here.
    ///
    /// ```
    /// use vm_core::value::Value;
    /// assert_eq!(Value::str("hé").as_str(), Some("hé"));
    /// assert_eq!(Value::str([0xA9u8]).as_str(), None);
    /// assert_eq!(Value::Int(1).as_str(), None);
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(self.as_bytes()?).ok()
    }

    /// Return the heap reference if this is `Value::HeapRef`, else `None`.
    pub fn as_heap_ref(&self) -> Option<gc_core::HeapRef> {
        match self {
            Value::HeapRef(r) => Some(*r),
            _ => None,
        }
    }

    // ------------------------------------------------------------------
    // Truth test (used by conditional branches)
    // ------------------------------------------------------------------

    /// Return whether the value is "truthy".
    ///
    /// - `Bool(false)` and `Int(0)` are falsy.
    /// - `Null` is falsy.
    /// - Everything else is truthy.
    ///
    /// This mirrors Python's `bool()` semantics and the behaviour of
    /// `jmp_if_true` / `jmp_if_false` in the Python vm-core.
    ///
    /// It is the *machine's* branch rule, not any source language's. A
    /// language whose truthiness differs writes its own rule as IIR before the
    /// branch, so the branch only ever sees a `0` / `1`. Twig's rule — only
    /// `#f` and nil are false, so `0` and `""` are true (TW00) — is emitted by
    /// `iir-builtin-lowering::vm_lisp` (#18493); the VM never learns it.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b)    => *b,
            Value::Int(n)     => *n != 0,
            Value::Float(f)   => *f != 0.0,
            Value::Str(s)     => !s.is_empty(),
            Value::HeapRef(r) => !r.is_null(),
            Value::Symbol(_)  => true,
            Value::Null       => false,
        }
    }

    // ------------------------------------------------------------------
    // Type-name helper (for profiler)
    // ------------------------------------------------------------------

    /// Map this value to the IIR type string the profiler should record.
    ///
    /// Matches the `default_type_mapper` from the Python vm-core:
    ///
    /// | Python type | IIR type |
    /// |-------------|----------|
    /// | `bool`      | `"bool"` |
    /// | `int` 0..255 | `"u8"` |
    /// | `int` 0..65535 | `"u16"` |
    /// | `int` 0..2³²-1 | `"u32"` |
    /// | any other int | `"u64"` |
    /// | `float`     | `"f64"` |
    /// | `str`       | `"str"` |
    /// | other       | `"any"` |
    pub fn iir_type_name(&self) -> &'static str {
        match self {
            Value::Bool(_)  => "bool",
            Value::Int(n) => {
                let n = *n;
                if (0..=255).contains(&n) {
                    "u8"
                } else if (0..=65535).contains(&n) {
                    "u16"
                } else if (0..=0xFFFF_FFFF).contains(&n) {
                    "u32"
                } else {
                    "u64"
                }
            }
            Value::Float(_)   => "f64",
            Value::Str(_)     => "str",
            Value::HeapRef(_) => "ref",
            // No IIR type names a symbol's machine shape, so the profiler
            // records it as "any" and the JIT never specialises on it.
            Value::Symbol(_)  => "any",
            Value::Null       => "any",
        }
    }
}


impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n)     => write!(f, "{n}"),
            Value::Float(v)   => write!(f, "{v}"),
            Value::Bool(b)    => write!(f, "{b}"),
            // UTF-8 text displays as a Rust string literal, as before PR5b;
            // other bytes display quoted with each byte that is not printable
            // ASCII as `\xNN`, so nothing is lost (LANG83 PR5b).
            Value::Str(s)     => match std::str::from_utf8(s) {
                Ok(text) => write!(f, "{text:?}"),
                Err(_) => write!(f, "\"{}\"", s.escape_ascii()),
            },
            Value::HeapRef(r) => write!(f, "{r}"),
            Value::Symbol(id) => write!(f, "symbol#{id}"),
            Value::Null       => write!(f, "null"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_i64_int() {
        assert_eq!(Value::Int(42).as_i64(), Some(42));
        assert_eq!(Value::Bool(true).as_i64(), Some(1));
        assert!(Value::Float(1.0).as_i64().is_none());
    }

    #[test]
    fn is_truthy_cases() {
        assert!(Value::Int(1).is_truthy());
        assert!(!Value::Int(0).is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(!Value::Null.is_truthy());
        assert!(Value::str("hi").is_truthy());
        assert!(!Value::str("").is_truthy());
        assert!(Value::HeapRef(gc_core::HeapRef::new(0x1000)).is_truthy());
        assert!(!Value::HeapRef(gc_core::HeapRef::NULL).is_truthy());
        assert!(Value::Symbol(0).is_truthy());
    }

    /// #18470: a symbol is an atom of its own — never a number, equal only to
    /// the same id.
    #[test]
    fn symbol_is_not_a_number_and_compares_by_id() {
        assert_eq!(Value::Symbol(3), Value::Symbol(3));
        assert_ne!(Value::Symbol(3), Value::Symbol(4));
        assert_ne!(Value::Symbol(0), Value::Int(0));
        assert!(Value::Symbol(7).as_i64().is_none());
        assert!(Value::Symbol(7).as_f64().is_none());
        assert!(Value::Symbol(7).as_heap_ref().is_none());
        assert_eq!(Value::Symbol(7).iir_type_name(), "any");
        assert_eq!(Value::Symbol(7).to_string(), "symbol#7");
    }

    #[test]
    fn as_heap_ref_extracts_and_rejects_other_kinds() {
        let r = gc_core::HeapRef::new(0x2000);
        assert_eq!(Value::HeapRef(r).as_heap_ref(), Some(r));
        assert_eq!(Value::Int(0x2000).as_heap_ref(), None);
        assert_eq!(Value::Null.as_heap_ref(), None);
    }

    /// LANG83 PR5b: a string holds any bytes, not only UTF-8.
    #[test]
    fn a_string_holds_bytes_and_offers_text_only_when_it_is_utf8() {
        let lone = Value::str([0xA9u8]);
        assert_eq!(lone.as_bytes(), Some(&[0xA9][..]));
        assert_eq!(lone.as_str(), None, "a lone continuation byte is not UTF-8");
        assert!(lone.is_truthy());
        assert_eq!(lone.iir_type_name(), "str");
        assert_eq!(lone.to_string(), r#""\xa9""#);
        let text = Value::str(String::from("hé"));
        assert_eq!(text.as_str(), Some("hé"));
        assert_eq!(text.as_bytes(), Some(&b"h\xC3\xA9"[..]));
        assert_eq!(text.to_string(), r#""hé""#);
        // The same bytes are the same string, however they were spelled.
        assert_eq!(text, Value::str(vec![b'h', 0xC3, 0xA9]));
        assert_eq!(Value::Int(1).as_bytes(), None);
        // A clone shares the bytes rather than copying them.
        let Value::Str(a) = &text else { unreachable!() };
        let copy = text.clone();
        let Value::Str(b) = &copy else { unreachable!() };
        assert!(std::sync::Arc::ptr_eq(a, b));
    }

    #[test]
    fn iir_type_name_ranges() {
        assert_eq!(Value::Int(0).iir_type_name(), "u8");
        assert_eq!(Value::Int(255).iir_type_name(), "u8");
        assert_eq!(Value::Int(256).iir_type_name(), "u16");
        assert_eq!(Value::Int(65536).iir_type_name(), "u32");
        assert_eq!(Value::Int(-1).iir_type_name(), "u64");
        assert_eq!(Value::Float(1.0).iir_type_name(), "f64");
        assert_eq!(Value::Bool(true).iir_type_name(), "bool");
        assert_eq!(Value::str("hi").iir_type_name(), "str");
        assert_eq!(Value::HeapRef(gc_core::HeapRef::new(0x1000)).iir_type_name(), "ref");
        assert_eq!(Value::Null.iir_type_name(), "any");
    }
}
