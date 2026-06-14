# PR38: Prolog `call/N` Meta-Call

This batch adds Prolog-style meta-call argument extension on top of the existing
callable-term runtime.

## Scope

- Extend `calltermo(term_goal, *extra_args)` so library callers can append
  arguments to atoms, compound callable terms, and module-qualified goals.
- Adapt parsed SWI-style `call/2` through `call/8` into the same builtin path.
- Preserve extra arguments when loader module-linking rewrites the first
  meta-call argument.
- Prove the behavior through direct builtins tests, loader tests, and a Logic VM
  stress test.

## Examples

```prolog
?- call(member, Item, [tea, cake]).
Item = tea ;
Item = cake.
```

```prolog
?- call(pair, Name, Flavor).
```

where `pair/2` may itself use nested meta-calls:

```prolog
pair(Name, Flavor) :-
    call(pick, Name),
    call(member, Flavor, [sweet, savory]).
```

## Non-goals

- Full `meta_predicate/1` declaration semantics.
- Higher-order list predicates such as `maplist/N` and `foldl/N`.
- Module-sensitive closures beyond explicit `Module:Callable` terms.
