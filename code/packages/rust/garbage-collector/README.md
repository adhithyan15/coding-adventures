# garbage-collector

Mark-and-sweep garbage collector simulation for educational use.

## Architecture

1. **HeapObject trait** -- the contract for anything on the managed heap
2. **ConsCell, Symbol, LispClosure** -- concrete heap object types
3. **GarbageCollector trait** -- abstract interface (allocate, deref, collect, stats)
4. **MarkAndSweepGC** -- the classic algorithm (mark reachable from roots, sweep the rest)
5. **SymbolTable** -- intern symbols for identity-based equality

## How Mark-and-Sweep Works

1. **Mark**: Starting from roots (stack, globals), follow all references and mark each reachable object.
2. **Sweep**: Walk the entire heap. Delete any object that wasn't marked.
3. **Reset**: Clear all marks for the next cycle.

## Usage

```rust
use garbage_collector::*;

let mut gc = MarkAndSweepGC::new();
let addr1 = gc.allocate(Box::new(ConsCell::new(42, -1)));
let addr2 = gc.allocate(Box::new(Symbol::new("x")));

// Collect with addr1 as root -- addr2 will be freed
let freed = gc.collect(&[Value::Address(addr1)]);
assert_eq!(freed, 1);
```
