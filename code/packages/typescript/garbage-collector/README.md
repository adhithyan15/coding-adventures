# Garbage Collector (TypeScript)

A language-agnostic mark-and-sweep garbage collector for educational virtual machines.

The package provides a small heap-object contract, a concrete `MarkAndSweepGC`, Lisp-flavored heap objects, and a symbol table that interns names on the managed heap.

## Example

```ts
import { ConsCell, MarkAndSweepGC, Symbol } from "@coding-adventures/garbage-collector";

const gc = new MarkAndSweepGC();
const tail = gc.allocate(new Symbol("end"));
const head = gc.allocate(new ConsCell(1, tail));

gc.collect([head]);
gc.heapSize(); // 2
```
