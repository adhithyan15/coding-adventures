# PR57: Prolog Numbered Variables And Write Options

## Goal

Extend the Prolog-on-Logic-VM term metaprogramming layer with numbered
variables. This gives source-level programs a practical way to freeze open
variables into printable placeholders, then render those placeholders through
the existing term text I/O bridge.

## Scope

The builtin and loader layers expose:

```text
numbervars/3
write_term_to_atom/3 option numbervars(true)
```

The VM path supports both through the parser, loader adapter, compiler, and
runtime pipeline.

## Semantics

This batch implements the finite core of Prolog numbered-variable handling:

- `numbervars(Term, Start, End)` walks the reified `Term` left-to-right and
  binds each still-open variable to `'$VAR'(N)`, starting at `Start`.
- Repeated occurrences of the same variable receive the same `'$VAR'(N)`
  placeholder.
- `End` is unified with the first unused number.
- Non-negative integer starts are supported; negative, float, and open starts
  fail cleanly for now.
- Ground terms do not change and unify `End` with `Start`.
- `write_term_to_atom(Term, Atom, [numbervars(true)])` renders `'$VAR'(0)` as
  `A`, `'$VAR'(1)` as `B`, through `Z`, then continues as `A1`, `B1`, and so
  on.
- `numbervars(false)` keeps canonical rendering such as `'$VAR'(0)`.

## Verification

- `logic-builtins` tests cover first-occurrence numbering, repeated variables,
  ground terms, and invalid starts.
- `prolog-loader` tests cover source-level `numbervars/3` adaptation and both
  numbered and canonical write rendering.
- `prolog-vm-compiler` stress coverage runs numbered variables end-to-end
  through parser, loader, compiler, VM, and named-answer extraction.
