# Changelog

## [Unreleased]

### Changed

- Declared the real dependency on `CodingAdventures::NeuralNetwork` in `cpanfile` (it `use`s it and its `BUILD` already puts `../neural-network/lib` on `PERL5LIB`), so the build-tool validator can see the dependency edge for the Windows recipe.
- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

### Fixed

- `BUILD_windows` passes dependency library paths with `-I` instead of `set PERL5LIB=...`, which MSYS perl on CI splits on `:` and so never found.
