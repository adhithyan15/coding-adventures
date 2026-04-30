# prolog-loader

`prolog-loader` is the first explicit loading layer above the Prolog dialect
parsers.

It keeps parsing side-effect free, then exposes helpers to:

- normalize dialect-specific parsed sources into one shared loaded shape
- retain structured module/import metadata such as `module/2` and
  `use_module/1,2`
- collect `initialization/1` directives in source order
- run those initialization goals explicitly against the loaded `Program`
- adapt parsed Prolog builtin calls like `call/1..8`, `dynamic/1`, `assertz/1`,
  and `predicate_property/2` into runtime goals before execution
- adapt finite integer builtins such as `integer/1`, `between/3`, and `succ/2`
- adapt callable CLP(FD) forms such as `in/2`, `ins/2`, `#=/2`,
  `all_different/1`, and `labeling/2`
- flatten nested additive CLP(FD) equality expressions such as
  `Z #= X + Y + 1`
- preserve supported `labeling/2` options such as `down` and `leftmost`
- adapt `phrase/2` and `phrase/3` into executable DCG runtime calls
- link multiple loaded sources into one namespace-aware runnable project with
  module-local predicates and weak imports
- rewrite explicit `module:goal` qualification during linking, including common
  meta-goal forms like `call/1..8`, apply-family closures, `once/1`, `not/1`,
  `\\+/1`, and `phrase/2,3`
- adapt Prolog control constructs such as `->/2` and
  `(If -> Then ; Else)` into executable builtin goals
- adapt common list predicates such as `member/2`, `append/3`, `select/3`,
  `permutation/2`, `reverse/2`, `last/2`, `length/2`, `sort/2`, `msort/2`,
  `nth0/3`, `nth1/3`, `nth0/4`, `nth1/4`, and `is_list/1` into relational
  standard-library goals
- adapt higher-order list predicates such as `maplist/2..5`, `convlist/3`,
  `include/3`, `exclude/3`, `partition/4`, `foldl/4..7`, and `scanl/4..7`
  into callable-term-backed builtin goals
- load SWI-Prolog source graphs from real `.pl` files through relative
  `consult/1`, `ensure_loaded/1`, and file-backed `use_module/1,2`
- splice `include/1` targets into the including source before project linking
- accept an optional `SourceResolver` hook so callers can resolve non-file
  source references like `library(...)` without hard-coding one search policy
- apply explicit `term_expansion/2` and `goal_expansion/2` passes during load
  without making parsing itself stateful or magical
- rewrite ad-hoc parsed queries through a linked project's module/import
  context
- expose a convenience runner for executing initialization goals with the shared
  Prolog builtin adapter enabled

This package is the bridge between “we parsed a Prolog file” and “we loaded a
Prolog file and are ready to run its startup behavior.”
