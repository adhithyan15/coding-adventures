# HOS-003c: Elixir and Haskell library ports

This item provides reusable Elixir and Haskell ports of the HOS00 First Edition
Unix teaching model. Each package keeps filesystem, descriptor-table, kernel,
and command semantics in a library API that needs no fixture files, process
arguments, terminal output, or other host I/O.

## Component contract

The two ports preserve the canonical path resolution, byte-range reads,
descriptor allocation and offsets, kernel error codes, and `cat`/`ls` traces.
The canonical nine scenarios remain solely in
`fixtures/historical-unix-v1/file-read.json`. Package component tests do not
read that file. External fixture adapters import each library, compare its
output and events with every canonical case, and keep JSON and host I/O outside
the reusable runtime packages.

The Elixir library remains at `elixir/unix_history_simulator`; repository
selection and caching use that exact underscore package name. Its fixture
adapter is a separate Elixir script loaded only by the package BUILD recipe.
The Haskell package has a standalone Cabal project that includes the reusable
library and a fixture-adjacent adapter executable. The adapter depends on the
package API and delegates scenario iteration and comparison to
`replay_cli.py`.

## Validation boundary

Linux and Windows BUILD fronts run package component checks plus the shared
nine-case adapter. Haskell's Unix build front sets the Command Line Tools
developer directory only on Darwin for every Cabal invocation, matching GHC's
linker requirement. Repository-input routing names the canonical fixture,
shared replay helper, and exact adapter sources and project files consumed by
each lane. It does not use `build_native.py` or a directory-wide fixture glob.

The model is a teaching contract, not a historical executable or syscall ABI.
No claim is made about historical binary compatibility or unimplemented Unix
features.
