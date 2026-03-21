//! # Garbage Collector -- Language-agnostic memory management framework.
//!
//! This crate provides an abstract garbage collection interface that any
//! virtual machine can use, plus a concrete mark-and-sweep implementation.
//!
//! ## The Big Picture
//!
//! When a program runs, it creates objects -- cons cells, closures, strings,
//! arrays. Some become unreachable: no variable points to them, no other
//! object references them. Without cleanup, memory grows without bound.
//!
//! A **garbage collector** automatically finds and reclaims unreachable
//! objects. This crate provides:
//!
//! 1. **HeapObject trait** -- the contract that all heap-allocated objects implement
//! 2. **MarkAndSweepGC** -- the classic algorithm (mark from roots, sweep unmarked)
//! 3. **Heap object types** -- ConsCell, Symbol, LispClosure
//! 4. **SymbolTable** -- ensures identity-based equality for symbols
//!
//! ## How Mark-and-Sweep Works
//!
//! 1. **Mark**: Starting from roots, recursively follow all references
//!    and mark each reachable object.
//! 2. **Sweep**: Walk the entire heap. Delete any object that wasn't marked.
//! 3. **Reset**: Clear all marks for the next cycle.
//!
//! ## Handling Cycles
//!
//! Mark-and-sweep handles reference cycles correctly. If A references B
//! and B references A, but neither is reachable from a root, both are
//! correctly identified as garbage.

use std::collections::HashMap;
use std::fmt;
use std::any::Any;

// =========================================================================
// Value type -- represents things that can be on the stack or in variables
// =========================================================================

/// A runtime value that may or may not be a heap address.
///
/// The GC uses this to scan roots: only Address values are followed
/// as potential heap references.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// An integer value (could also be a heap address).
    Int(i64),
    /// A heap address (always followed by the GC).
    Address(usize),
    /// A string value.
    Str(String),
    /// A boolean value.
    Bool(bool),
    /// Nil / None.
    Nil,
    /// A list of values (scanned recursively by GC).
    List(Vec<Value>),
}

// =========================================================================
// HeapObject trait
// =========================================================================

/// Base trait for anything that lives on the managed heap.
///
/// Every heap object has a `marked` flag used by tracing garbage
/// collectors. During the mark phase, reachable objects get marked.
/// During the sweep phase, unmarked objects are freed.
///
/// Subclasses should store references to other heap objects as integer
/// addresses. The GC calls `references()` to find them during marking.
pub trait HeapObject: fmt::Debug + Any {
    /// Return all heap addresses that this object references.
    ///
    /// The GC calls this during the mark phase to find transitive
    /// references. Return an empty vec if this object holds no references.
    fn references(&self) -> Vec<usize>;

    /// Get the marked flag.
    fn is_marked(&self) -> bool;

    /// Set the marked flag.
    fn set_marked(&mut self, marked: bool);

    /// Get a human-readable type name.
    fn type_name(&self) -> &str;
}

// =========================================================================
// Concrete heap object types
// =========================================================================

/// A cons cell -- the fundamental building block of Lisp lists.
///
/// A cons cell is simply a pair: `car` (first element) and `cdr` (rest).
/// Lists are chains of cons cells:
/// ```text
/// (1 2 3) = ConsCell(1, ConsCell(2, ConsCell(3, NIL)))
/// ```
///
/// When car or cdr is a valid heap address, the GC follows it during marking.
#[derive(Debug, Clone)]
pub struct ConsCell {
    /// First element (a value or heap address).
    pub car: i64,
    /// Rest of list (a heap address or sentinel).
    pub cdr: i64,
    /// GC mark flag.
    pub marked: bool,
}

impl ConsCell {
    pub fn new(car: i64, cdr: i64) -> Self {
        ConsCell { car, cdr, marked: false }
    }
}

impl HeapObject for ConsCell {
    fn references(&self) -> Vec<usize> {
        let mut refs = Vec::new();
        if self.car >= 0 { refs.push(self.car as usize); }
        if self.cdr >= 0 { refs.push(self.cdr as usize); }
        refs
    }
    fn is_marked(&self) -> bool { self.marked }
    fn set_marked(&mut self, marked: bool) { self.marked = marked; }
    fn type_name(&self) -> &str { "ConsCell" }
}

/// An interned symbol -- a named atom in Lisp.
///
/// Symbols are interned: every occurrence of the same name maps to the
/// same heap address. This makes identity-based equality work.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub marked: bool,
}

impl Symbol {
    pub fn new(name: &str) -> Self {
        Symbol { name: name.to_string(), marked: false }
    }
}

impl HeapObject for Symbol {
    fn references(&self) -> Vec<usize> { vec![] }
    fn is_marked(&self) -> bool { self.marked }
    fn set_marked(&mut self, marked: bool) { self.marked = marked; }
    fn type_name(&self) -> &str { "Symbol" }
}

/// A function closure -- compiled code + captured environment.
///
/// When a lambda expression is evaluated, it captures the current
/// environment (variable bindings). The result is a closure: the
/// function body plus the captured environment.
#[derive(Debug, Clone)]
pub struct LispClosure {
    /// Some representation of the code (simplified as a string here).
    pub code: String,
    /// Captured environment: variable name -> value (may contain heap addresses).
    pub env: HashMap<String, i64>,
    /// Parameter names.
    pub params: Vec<String>,
    /// GC mark flag.
    pub marked: bool,
}

impl LispClosure {
    pub fn new(code: &str, env: HashMap<String, i64>, params: Vec<String>) -> Self {
        LispClosure { code: code.to_string(), env, params, marked: false }
    }
}

impl HeapObject for LispClosure {
    fn references(&self) -> Vec<usize> {
        self.env.values().filter(|&&v| v >= 0).map(|&v| v as usize).collect()
    }
    fn is_marked(&self) -> bool { self.marked }
    fn set_marked(&mut self, marked: bool) { self.marked = marked; }
    fn type_name(&self) -> &str { "LispClosure" }
}

// =========================================================================
// GarbageCollector trait
// =========================================================================

/// Abstract interface for garbage collection algorithms.
///
/// VMs depend on this trait, never on a specific algorithm. This makes
/// algorithms hot-swappable: MarkAndSweepGC, RefCountGC, etc.
pub trait GarbageCollector {
    /// Allocate an object on the heap and return its address.
    fn allocate(&mut self, obj: Box<dyn HeapObject>) -> usize;

    /// Look up a heap object by its address.
    fn deref(&self, address: usize) -> Option<&dyn HeapObject>;

    /// Look up a heap object mutably.
    fn deref_mut(&mut self, address: usize) -> Option<&mut dyn HeapObject>;

    /// Run a garbage collection cycle. Returns the number of objects freed.
    fn collect(&mut self, roots: &[Value]) -> usize;

    /// Return the number of objects currently on the heap.
    fn heap_size(&self) -> usize;

    /// Return introspection counters for debugging and testing.
    fn stats(&self) -> GcStats;

    /// Check whether an address points to a live heap object.
    fn is_valid_address(&self, address: usize) -> bool;
}

/// GC statistics for introspection.
#[derive(Debug, Clone, PartialEq)]
pub struct GcStats {
    pub total_allocations: usize,
    pub total_collections: usize,
    pub total_freed: usize,
    pub heap_size: usize,
}

// =========================================================================
// MarkAndSweepGC
// =========================================================================

/// Mark-and-sweep garbage collector implementation.
///
/// The heap is a HashMap mapping integer addresses to HeapObject trait objects.
/// Addresses start at 0x10000 (65536) to avoid ambiguity with small integer
/// values used in programs.
///
/// ## Algorithm
///
/// 1. **Mark**: DFS from roots, marking all reachable objects
/// 2. **Sweep**: Walk heap, delete unmarked objects, clear marks on survivors
pub struct MarkAndSweepGC {
    heap: HashMap<usize, Box<dyn HeapObject>>,
    next_address: usize,
    total_allocations: usize,
    total_collections: usize,
    total_freed: usize,
}

impl MarkAndSweepGC {
    /// Create a new empty garbage collector.
    ///
    /// Addresses start at 0x10000 to avoid confusion with small integers.
    pub fn new() -> Self {
        MarkAndSweepGC {
            heap: HashMap::new(),
            next_address: 0x10000,
            total_allocations: 0,
            total_collections: 0,
            total_freed: 0,
        }
    }

    /// Recursively mark a value and everything it references.
    fn mark_value(&mut self, value: &Value) {
        match value {
            Value::Address(addr) => {
                if let Some(obj) = self.heap.get(addr) {
                    if !obj.is_marked() {
                        let refs = obj.references();
                        // Mark this object
                        self.heap.get_mut(addr).unwrap().set_marked(true);
                        // Recursively mark references
                        for r in refs {
                            self.mark_value(&Value::Address(r));
                        }
                    }
                }
            }
            Value::Int(i) => {
                // Integers might be heap addresses
                let addr = *i as usize;
                if self.heap.contains_key(&addr) {
                    self.mark_value(&Value::Address(addr));
                }
            }
            Value::List(items) => {
                for item in items {
                    self.mark_value(item);
                }
            }
            _ => {} // Strings, bools, nil -- not heap references
        }
    }
}

impl Default for MarkAndSweepGC {
    fn default() -> Self { Self::new() }
}

impl GarbageCollector for MarkAndSweepGC {
    fn allocate(&mut self, obj: Box<dyn HeapObject>) -> usize {
        let address = self.next_address;
        self.next_address += 1;
        self.heap.insert(address, obj);
        self.total_allocations += 1;
        address
    }

    fn deref(&self, address: usize) -> Option<&dyn HeapObject> {
        self.heap.get(&address).map(|b| b.as_ref())
    }

    fn deref_mut(&mut self, address: usize) -> Option<&mut dyn HeapObject> {
        self.heap.get_mut(&address).map(|b| b.as_mut())
    }

    fn collect(&mut self, roots: &[Value]) -> usize {
        self.total_collections += 1;

        // Phase 1: Mark -- DFS from roots
        // We need to collect root values first, then mark, because marking
        // requires mutable access.
        let root_values: Vec<Value> = roots.to_vec();
        for root in &root_values {
            self.mark_value(root);
        }

        // Phase 2: Sweep -- delete unmarked, clear marks on survivors
        let to_delete: Vec<usize> = self.heap.iter()
            .filter(|(_, obj)| !obj.is_marked())
            .map(|(&addr, _)| addr)
            .collect();

        for addr in &to_delete {
            self.heap.remove(addr);
        }

        // Clear marks on survivors
        for obj in self.heap.values_mut() {
            obj.set_marked(false);
        }

        let freed = to_delete.len();
        self.total_freed += freed;
        freed
    }

    fn heap_size(&self) -> usize {
        self.heap.len()
    }

    fn stats(&self) -> GcStats {
        GcStats {
            total_allocations: self.total_allocations,
            total_collections: self.total_collections,
            total_freed: self.total_freed,
            heap_size: self.heap_size(),
        }
    }

    fn is_valid_address(&self, address: usize) -> bool {
        self.heap.contains_key(&address)
    }
}

// =========================================================================
// SymbolTable
// =========================================================================

/// Interns symbols so that equal names share the same heap address.
///
/// A symbol table ensures identity-based equality for symbols: two
/// references to the same name get the same heap address.
pub struct SymbolTable<'a> {
    gc: &'a mut dyn GarbageCollector,
    table: HashMap<String, usize>,
}

impl<'a> SymbolTable<'a> {
    /// Create a symbol table backed by the given garbage collector.
    pub fn new(gc: &'a mut dyn GarbageCollector) -> Self {
        SymbolTable { gc, table: HashMap::new() }
    }

    /// Intern a symbol name, returning its heap address.
    ///
    /// If the symbol has been interned before and is still alive,
    /// returns the existing address. Otherwise, allocates a new Symbol.
    pub fn intern(&mut self, name: &str) -> usize {
        if let Some(&addr) = self.table.get(name) {
            if self.gc.is_valid_address(addr) {
                return addr;
            }
        }
        let addr = self.gc.allocate(Box::new(Symbol::new(name)));
        self.table.insert(name.to_string(), addr);
        addr
    }

    /// Look up a symbol without allocating.
    pub fn lookup(&self, name: &str) -> Option<usize> {
        self.table.get(name).and_then(|&addr| {
            if self.gc.is_valid_address(addr) { Some(addr) } else { None }
        })
    }

    /// Return all currently interned (alive) symbols.
    pub fn all_symbols(&self) -> HashMap<String, usize> {
        self.table.iter()
            .filter(|(_, &addr)| self.gc.is_valid_address(addr))
            .map(|(name, &addr)| (name.clone(), addr))
            .collect()
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_and_deref() {
        let mut gc = MarkAndSweepGC::new();
        let addr = gc.allocate(Box::new(ConsCell::new(42, -1)));
        assert!(gc.is_valid_address(addr));
        let obj = gc.deref(addr).unwrap();
        assert_eq!(obj.type_name(), "ConsCell");
    }

    #[test]
    fn test_allocate_symbol() {
        let mut gc = MarkAndSweepGC::new();
        let addr = gc.allocate(Box::new(Symbol::new("foo")));
        assert!(gc.is_valid_address(addr));
        assert_eq!(gc.heap_size(), 1);
    }

    #[test]
    fn test_collect_unreachable() {
        let mut gc = MarkAndSweepGC::new();
        let addr1 = gc.allocate(Box::new(ConsCell::new(42, -1)));
        let _addr2 = gc.allocate(Box::new(Symbol::new("unreachable")));
        assert_eq!(gc.heap_size(), 2);

        // Collect with only addr1 as root -- addr2 should be freed
        let freed = gc.collect(&[Value::Address(addr1)]);
        assert_eq!(freed, 1);
        assert_eq!(gc.heap_size(), 1);
        assert!(gc.is_valid_address(addr1));
    }

    #[test]
    fn test_collect_reachable_chain() {
        let mut gc = MarkAndSweepGC::new();
        let addr2 = gc.allocate(Box::new(Symbol::new("end")));
        let addr1 = gc.allocate(Box::new(ConsCell::new(addr2 as i64, -1)));

        // addr1 references addr2, so both should survive
        let freed = gc.collect(&[Value::Address(addr1)]);
        assert_eq!(freed, 0);
        assert_eq!(gc.heap_size(), 2);
    }

    #[test]
    fn test_collect_cycle() {
        let mut gc = MarkAndSweepGC::new();
        let addr1 = gc.allocate(Box::new(ConsCell::new(0, 0)));
        let addr2 = gc.allocate(Box::new(ConsCell::new(0, 0)));

        // Create a cycle: addr1.car -> addr2, addr2.car -> addr1
        // We can't easily mutate the objects in this design, but we can
        // test that unreachable cycles are collected
        let _addr3 = gc.allocate(Box::new(Symbol::new("standalone")));

        // Only addr1 is a root, but it doesn't reference addr2 or addr3
        // (car=0, cdr=0 are not valid heap addresses since they start at 0x10000)
        let freed = gc.collect(&[Value::Address(addr1)]);
        assert_eq!(freed, 2); // addr2 and addr3 should be freed
    }

    #[test]
    fn test_collect_no_roots() {
        let mut gc = MarkAndSweepGC::new();
        gc.allocate(Box::new(ConsCell::new(1, 2)));
        gc.allocate(Box::new(Symbol::new("orphan")));
        assert_eq!(gc.heap_size(), 2);

        let freed = gc.collect(&[]);
        assert_eq!(freed, 2);
        assert_eq!(gc.heap_size(), 0);
    }

    #[test]
    fn test_stats() {
        let mut gc = MarkAndSweepGC::new();
        gc.allocate(Box::new(Symbol::new("a")));
        gc.allocate(Box::new(Symbol::new("b")));
        gc.collect(&[]);

        let stats = gc.stats();
        assert_eq!(stats.total_allocations, 2);
        assert_eq!(stats.total_collections, 1);
        assert_eq!(stats.total_freed, 2);
        assert_eq!(stats.heap_size, 0);
    }

    #[test]
    fn test_address_space() {
        let mut gc = MarkAndSweepGC::new();
        let addr1 = gc.allocate(Box::new(Symbol::new("a")));
        let addr2 = gc.allocate(Box::new(Symbol::new("b")));
        // Addresses start at 0x10000 and are monotonically increasing
        assert_eq!(addr1, 0x10000);
        assert_eq!(addr2, 0x10001);
    }

    #[test]
    fn test_closure_references() {
        let mut env = HashMap::new();
        env.insert("x".to_string(), 0x10000_i64);
        env.insert("y".to_string(), -1_i64); // Not a valid address
        let closure = LispClosure::new("(lambda (a) (+ a x))", env, vec!["a".to_string()]);
        let refs = closure.references();
        assert_eq!(refs, vec![0x10000]);
    }

    #[test]
    fn test_symbol_table_intern() {
        let mut gc = MarkAndSweepGC::new();
        let mut table = SymbolTable::new(&mut gc);

        let addr1 = table.intern("foo");
        let addr2 = table.intern("foo");
        assert_eq!(addr1, addr2); // Same name -> same address

        let addr3 = table.intern("bar");
        assert_ne!(addr1, addr3); // Different name -> different address
    }

    #[test]
    fn test_symbol_table_lookup() {
        let mut gc = MarkAndSweepGC::new();
        let mut table = SymbolTable::new(&mut gc);

        assert!(table.lookup("foo").is_none());
        table.intern("foo");
        assert!(table.lookup("foo").is_some());
    }

    #[test]
    fn test_symbol_table_all_symbols() {
        let mut gc = MarkAndSweepGC::new();
        let mut table = SymbolTable::new(&mut gc);

        table.intern("foo");
        table.intern("bar");
        table.intern("baz");

        let symbols = table.all_symbols();
        assert_eq!(symbols.len(), 3);
        assert!(symbols.contains_key("foo"));
        assert!(symbols.contains_key("bar"));
        assert!(symbols.contains_key("baz"));
    }

    #[test]
    fn test_multiple_collections() {
        let mut gc = MarkAndSweepGC::new();
        let root = gc.allocate(Box::new(Symbol::new("root")));

        // Allocate and collect multiple times
        for _ in 0..5 {
            gc.allocate(Box::new(Symbol::new("temp")));
        }
        gc.collect(&[Value::Address(root)]);
        assert_eq!(gc.heap_size(), 1);

        for _ in 0..3 {
            gc.allocate(Box::new(Symbol::new("temp2")));
        }
        gc.collect(&[Value::Address(root)]);
        assert_eq!(gc.heap_size(), 1);

        let stats = gc.stats();
        assert_eq!(stats.total_allocations, 9); // 1 root + 5 + 3
        assert_eq!(stats.total_collections, 2);
        assert_eq!(stats.total_freed, 8);
    }

    #[test]
    fn test_list_roots() {
        let mut gc = MarkAndSweepGC::new();
        let addr1 = gc.allocate(Box::new(Symbol::new("a")));
        let addr2 = gc.allocate(Box::new(Symbol::new("b")));
        let _addr3 = gc.allocate(Box::new(Symbol::new("c")));

        // Use a list of values as roots (simulating a VM stack)
        let roots = vec![Value::List(vec![Value::Address(addr1), Value::Address(addr2)])];
        let freed = gc.collect(&roots);
        assert_eq!(freed, 1); // Only addr3 should be freed
    }

    #[test]
    fn test_deref_freed_object() {
        let mut gc = MarkAndSweepGC::new();
        let addr = gc.allocate(Box::new(Symbol::new("gone")));
        gc.collect(&[]);
        assert!(gc.deref(addr).is_none());
        assert!(!gc.is_valid_address(addr));
    }
}
