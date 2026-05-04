//! # HeapKind — layout descriptors for managed heap objects.
//!
//! The LANG16 spec (§ "kind tags") introduces a 16-bit `kind` operand on
//! the `alloc` IIR opcode.  A kind is a **compile-time label** that tells
//! the runtime how large the object is, which byte offsets inside it hold
//! other heap references (so the GC can trace them), and whether the object
//! has a finalizer that must run before it is freed.
//!
//! ## Why not just RTTI?
//!
//! Runtime type information couples every object to a vtable pointer, adding
//! 8–16 bytes of overhead per object.  Many short-lived small objects (e.g.
//! cons cells) would pay more for the tag than for their actual data.
//!
//! The kind-id approach is the same one used by the LuaJIT garbage collector
//! and MRuby: a compact integer index into a process-wide layout table.  The
//! table is populated at program startup; the GC never touches it again
//! during a collection.
//!
//! ## Example
//!
//! A Lisp cons cell has two ref-typed slots (`car` and `cdr`) at offsets 0
//! and 8 on a 64-bit machine:
//!
//! ```
//! use gc_core::kind::{HeapKind, KindRegistry};
//!
//! let mut reg = KindRegistry::new();
//! let cons_id = reg.register(HeapKind {
//!     kind_id: 0,          // placeholder; register() fills this in
//!     size: 16,
//!     field_offsets: vec![0, 8],
//!     type_name: "ConsCell".to_string(),
//!     finalizer: false,
//! });
//! assert_eq!(cons_id, 0);
//! assert_eq!(reg.lookup(0).unwrap().type_name, "ConsCell");
//! ```

/// Layout descriptor for one class of heap-allocated object.
///
/// Registered at VM startup; the GC queries this during the mark phase
/// to find child refs, and during the sweep phase to run the finalizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeapKind {
    /// 16-bit identifier assigned by `KindRegistry::register`.
    ///
    /// The compiler emits this value as the second operand to `alloc`.
    /// The linker (LANG10) merges per-module tables into a process-wide one
    /// and re-numbers ids to avoid collisions.
    pub kind_id: u16,

    /// Object size in bytes.  The allocator requests exactly this many bytes.
    ///
    /// Generational collectors use this for their size-class selection; the
    /// mark-and-sweep collector ignores it (it works through trait objects).
    pub size: usize,

    /// Byte offsets of all `ref`-typed fields within the object.
    ///
    /// The GC calls `obj.references()` on `HeapObject` implementations; for
    /// kinds where the language frontend defines its own layout the field
    /// offsets let a generic tracer work without a vtable dispatch.
    ///
    /// An empty vec means "this object has no ref fields" (e.g. a boxed u8
    /// or a raw byte buffer).
    pub field_offsets: Vec<usize>,

    /// Human-readable name for debugging and profiling output.
    pub type_name: String,

    /// Whether objects of this kind have a finalizer that must run before
    /// the object's memory is reclaimed.
    ///
    /// Setting this to `true` causes the GC to defer freeing to a finalizer
    /// queue instead of immediately reclaiming the object.  Mark-and-sweep
    /// today calls the finalizer synchronously during the sweep phase;
    /// concurrent collectors may run it on a dedicated thread.
    pub finalizer: bool,
}

impl HeapKind {
    /// A sentinel kind for opaque untraced memory (no ref fields, no finalizer).
    ///
    /// This is the default when the language frontend does not register a
    /// specific kind.  The GC can still collect objects of this kind; it just
    /// won't find any child refs inside them.
    pub fn opaque(size: usize, name: &str) -> Self {
        HeapKind {
            kind_id: u16::MAX,  // placeholder; overwritten by register()
            size,
            field_offsets: vec![],
            type_name: name.to_string(),
            finalizer: false,
        }
    }
}

/// Process-wide registry of all registered `HeapKind` descriptors.
///
/// The registry is built during VM startup (before execution begins) and is
/// read-only during collection.  `GcCore` holds one registry for the entire
/// lifetime of the VM instance.
#[derive(Debug, Default)]
pub struct KindRegistry {
    kinds: Vec<HeapKind>,
}

impl KindRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        KindRegistry { kinds: Vec::new() }
    }

    /// Register a new kind and return its assigned `kind_id`.
    ///
    /// The returned id is what the compiler should emit as the second
    /// operand to `alloc`.  Kind ids are assigned sequentially from 0.
    ///
    /// # Panics
    ///
    /// Panics if more than `u16::MAX` kinds are registered (65535 kinds is
    /// far beyond any realistic use case; an MLton-style whole-program
    /// compiler rarely exceeds a few hundred).
    pub fn register(&mut self, mut kind: HeapKind) -> u16 {
        let id = u16::try_from(self.kinds.len())
            .expect("kind registry overflow: more than 65535 heap kinds registered");
        kind.kind_id = id;
        self.kinds.push(kind);
        id
    }

    /// Look up a registered kind by its id.
    pub fn lookup(&self, kind_id: u16) -> Option<&HeapKind> {
        self.kinds.get(kind_id as usize)
    }

    /// Number of registered kinds.
    pub fn len(&self) -> usize {
        self.kinds.len()
    }

    /// `true` if no kinds have been registered yet.
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    /// Iterate over all registered kinds.
    pub fn iter(&self) -> impl Iterator<Item = &HeapKind> {
        self.kinds.iter()
    }
}
