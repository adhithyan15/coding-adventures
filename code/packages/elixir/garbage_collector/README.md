# Garbage Collector (Elixir)

A language-agnostic mark-and-sweep garbage collector for educational virtual machines.

The package provides heap object structs, a functional mark-and-sweep GC state, Lisp-flavored heap objects, and a symbol table that interns names on the managed heap.

## Example

```elixir
alias CodingAdventures.GarbageCollector

gc = GarbageCollector.new()
{gc, tail} = GarbageCollector.allocate(gc, GarbageCollector.symbol("end"))
{gc, head} = GarbageCollector.allocate(gc, GarbageCollector.cons_cell(1, tail))
{gc, 0} = GarbageCollector.collect(gc, [head])

GarbageCollector.heap_size(gc)
# 2
```
