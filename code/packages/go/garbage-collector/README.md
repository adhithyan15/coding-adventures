# Garbage Collector (Go)

A language-agnostic mark-and-sweep garbage collector for educational virtual machines.

The package provides a heap object interface, a concrete `MarkAndSweepGC`, Lisp-flavored heap objects, and a symbol table that interns names on the managed heap.

## Example

```go
gc := garbagecollector.NewMarkAndSweepGC()
tail := gc.Allocate(garbagecollector.NewSymbol("end"))
head := gc.Allocate(garbagecollector.NewConsCell(1, tail))

gc.Collect([]any{head})
// gc.HeapSize() == 2
```
