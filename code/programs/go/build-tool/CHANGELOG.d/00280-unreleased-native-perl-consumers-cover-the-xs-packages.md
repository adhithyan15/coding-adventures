## Unreleased — Native Perl repository-input consumers cover every Rust XS package

- **New repository-input consumers `perl/gf256_native`, `perl/polynomial_native`
  and `perl/paint_codec_png_native`.** These three packages now build their Rust
  cdylib on Windows through `code/scripts/check-perl-extension-windows.ps1`, the
  same helper as `perl/conduit` and `perl/irc-server-native`. A change that
  edits only that helper, or the shared `rust/perl-bridge` crate, therefore
  selects all six XS consumers (and `perl/programs/conduit-hello`); a plan that
  lacks one of them still fails closed. The three packages are also listed in
  the `build-windows-os-suites` gate in `code/specs/data/ci-gates.json`.
