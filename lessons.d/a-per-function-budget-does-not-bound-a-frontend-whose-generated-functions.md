---
category: Compiler / VM / language pipeline
---

# A per-function budget does not bound a frontend whose generated functions grow with the square of the source

**What went wrong.** #18474 asked for Twig budgets on source size, tree
size, registers per function and instructions per function. Those four
look complete: the tree bounds the input, and the per-function caps bound
each output function. They are not. `(record P (f0 : int) … (fN : int))`
lowers to one accessor *per field*, and the accessor for field *i* walks
*i* `cdr`s, so the module holds about N²/2 instructions while every single
function stays tiny. A 25 KB record of 2,000 fields compiled to 2,009,006
instructions in 7 seconds; a 250 KB one, well inside a 1 MiB source budget
and a 2²⁰-item tree budget, asks for 200 million. No per-function check
ever fires. It is the same trap as *depth limits do not bound breadth*, one
level up: a bound on each piece says nothing about how many pieces there
are.

**Fix.** A module-wide instruction budget (2²⁰), checked as each function
is *added* (`Compiler::admit_function`), not once at the end, so a hostile
record stops after at most the budget plus one accessor of work. Every
function enters the module through that one method; the old
`self.functions.push` sites are gone, so none can skip it.

**Next time.** When sizing a compiler's budgets, look for every place the
output can grow faster than the input: one generated function per declared
item, per field, per variant, per pattern; helpers that walk a list per
element. Measure the expansion on a few sizes (n = 500, 2,000) rather than
reading it off the code, and add a whole-output budget that is checked
incrementally. A test past that budget should be a small input with a big
expansion, not a big input.
