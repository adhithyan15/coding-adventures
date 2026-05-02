# PR68 - Prolog Cleanup Control

## Goal

Add the next practical Prolog control layer on top of the Logic VM:
deterministic cleanup predicates that let source programs model resource-style
setup, guarded execution, and cleanup without leaving the VM path.

## Scope

- `logic-builtins` exposes `call_cleanupo/2`.
- `logic-builtins` exposes `setup_call_cleanupo/3`.
- `prolog-loader` rewrites source `call_cleanup/2` and
  `setup_call_cleanup/3` to those builtins.
- `prolog-vm-compiler` stress coverage proves the predicates survive
  source parsing, loading, VM compilation, and execution.

## Semantics

`call_cleanupo(Goal, Cleanup)` runs `Goal` and then runs `Cleanup` for the
first cleanup proof before yielding the result state. Ordinary cleanup failure
is ignored, which keeps cleanup useful for side-effect style dynamic database
operations.

If `Goal` throws a Prolog exception with `throw/1`, cleanup runs against the
thrown state and the exception is rethrown with that cleaned-up state. This
allows an enclosing `catch/3` recovery branch to observe cleanup side effects.

`setup_call_cleanupo(Setup, Goal, Cleanup)` runs `Setup` once and then runs
`Goal` under `call_cleanupo/2`.

## Deferred Semantics

Full Prolog cleanup around open choicepoints and cut-driven pruning is subtler
than this first VM layer. This PR intentionally documents and tests the
deterministic subset first. A later PR can add choicepoint-close hooks in the
engine if we need exact `setup_call_cleanup/3` behavior for nondeterministic
resource lifetimes.
