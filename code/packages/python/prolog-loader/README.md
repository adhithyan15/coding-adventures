# prolog-loader

`prolog-loader` is the first explicit loading layer above the Prolog dialect
parsers.

It keeps parsing side-effect free, then exposes helpers to:

- normalize dialect-specific parsed sources into one shared loaded shape
- collect `initialization/1` directives in source order
- run those initialization goals explicitly against the loaded `Program`
- adapt parsed Prolog builtin calls like `call/1`, `dynamic/1`, `assertz/1`,
  and `predicate_property/2` into runtime goals before execution
- expose a convenience runner for executing initialization goals with the shared
  Prolog builtin adapter enabled

This package is the bridge between “we parsed a Prolog file” and “we loaded a
Prolog file and are ready to run its startup behavior.”
