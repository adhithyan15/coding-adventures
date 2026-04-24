# prolog-loader

`prolog-loader` is the first explicit loading layer above the Prolog dialect
parsers.

It keeps parsing side-effect free, then exposes helpers to:

- normalize dialect-specific parsed sources into one shared loaded shape
- collect `initialization/1` directives in source order
- run those initialization goals explicitly against the loaded `Program`
- optionally adapt parsed goals into richer runtime or builtin goals before
  execution

This package is the bridge between “we parsed a Prolog file” and “we loaded a
Prolog file and are ready to run its startup behavior.”
