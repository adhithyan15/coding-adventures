# sir-mixins — modules, `include`, and `extend` (Ruby mixins)

## Status

New. Design/spec PR (specs-first). The largest remaining OOP gap toward the
north star (any Ruby → correct same-result output): real programs use
`module M; …; end` + `include M` to share behaviour. Builds directly on the
merged runtime-method-table OOP design ([[sir-classes-oop]]) and the typed-error
/ collection / exception cascades — same additive, runtime-table approach, **no
core-IR change**.

## Current state (2026-07-01 survey)

- **Parsing + lowering of `module`:** DONE. The grammar has
  `module_statement = "module" NAME { !"end" statement } "end"`, and the Ruby
  frontend lowers it to `Stmt::ModuleDef { name, body }` (Phase 14d), which
  triggers `Feature::Modules`. A module body's `def`s can therefore be lowered
  the same way class bodies are (a `__def_method__`-style registration keyed by
  the *module* name).
- **`include` / `extend`: NOT handled.** There is no lowering of `include M` /
  `extend M` (no `__include__`/`__extend__` builtin), and the OOP runtimes'
  method-resolution walk (`resolveMethod`/`call_method` → `methods[(class,
  name)]` walking `superclass`) never consults modules. So a method defined in a
  module and mixed into a class is **not found** — the call falls through to
  `NoMethodError`. This is the gap.

## Design — runtime method-table extension (NO core-IR change)

Reuse the OOP method table `methods[(owner, name)] = fn` — an *owner* is now
either a class OR a module name (modules register their `def`s exactly like
classes, via the existing `__def_method__` builtin keyed by the module name). Add
two things:

1. **Frontend lowering (one new builtin each, via `BuiltinCall` — no core-IR
   change):**
   - `include M` (in a class/module body) → `__include__("CurrentOwner", "M")`.
   - `extend M` → `__extend__("CurrentOwner", "M")` (mixes M's methods as
     *class/singleton* methods of the owner).
   - A module body's `def foo` → `__def_method__("M", "foo", closure)` (same
     builtin classes already use; the owner string is just a module name).
   - `Feature::Modules` already gates module defs; `include`/`extend` ride the
     same feature (or a sibling `Feature::Mixins` if the validator prefers an
     explicit flag — decide in MX1).

2. **Runtime (per backend, explicit tables — NEVER reflection
   [[dynamic-dispatch-rce]]):**
   - A per-owner list `included_modules[owner] = [moduleName, …]`, appended by
     `__include__` in **include order** (Ruby searches the *most recently
     included* module first, so the resolution walk iterates this list in
     **reverse**).
   - Extend the method-resolution walk to Ruby's MRO: for a receiver of class
     `C`, search **C** → **C's included modules (reverse)** → **C's superclass**
     → **its modules** → … → `Object`. The existing `superclass` walk gains a
     "check this owner's included modules before ascending" step.
   - `__extend__(owner, M)` registers M's methods into the owner's
     **class-method** table (`class_methods[(owner, name)]`), so they become
     callable as `Owner.method` / on the singleton.
   - **Cycle guard:** the walk already carries (or must carry) a `seen` set of
     owners; a module that (transitively) includes itself must terminate, not
     loop. Reuse the exception-ancestry `seen`-set discipline.

Ruby's real MRO also linearises diamond includes (a module included via two
paths appears once, at its earliest position). MX1's runtime walk implements the
**depth-first, most-recent-first, de-duplicated** order; document the exact
linearisation in the spec's truth table and prove it with a diamond test.

## Out of scope (note, defer)

- `prepend` (inserts the module *ahead* of the class — before its own methods).
- `Module#included`/`extended` hooks, `Comparable`/`Enumerable` as *mixed-in
  stdlib* modules (those need the stdlib method bodies; this cascade delivers the
  mixin *mechanism*, stdlib modules come with stdlib breadth).
- `refine`/refinements; `Module.new`; anonymous modules.

## Milestones (one PR per crate — MX1 frontend first, then backends parallel)

| # | Crate(s) | Content |
|---|---|---|
| MX0 | `code/specs/` | this spec |
| MX1 | `ruby-to-semantic-ir` (+ validator if a new feature flag) | lower `include`/`extend` → `__include__`/`__extend__`; module-body `def` → `__def_method__` keyed by module; unit + validation tests |
| MX2 | `sir-runtime-oop` (Python) + `semantic-ir-to-python` | `included_modules` table + MRO walk + `__extend__`; emit arms for the new builtins |
| MX3 | `sir-runtime-oop` (TS) + `semantic-ir-to-typescript` | same |
| MX4 | `semantic-ir-to-javascript` (inline runtime) | same |
| MX5 | `semantic-ir-to-go` (inline runtime) | same |
| MX6 | `semantic-ir-to-rust` (inline runtime) | same |

Each backend milestone: **execution-proof** a real Ruby program through the
native toolchain — a module with an instance method `include`d into a class,
called on an instance (finds the mixed-in method); an `include` that *overrides
nothing* but adds behaviour; a method the class defines itself **shadows** the
module's (class-first MRO); a diamond include resolves once; `extend` makes the
method a class method — each matching the reference backend. Security-review gate
(all dispatch stays explicit-table, cycle-guarded). Cross-backend parity: one
golden mixin suite through all 5.

Sequencing: MX2–MX6 touch the same OOP-runtime / backend-runtime files as the
[[sir-classes-oop]] and [[sir-typed-runtime-errors]] work — fan out only after
those merge, one in-flight PR per crate.
