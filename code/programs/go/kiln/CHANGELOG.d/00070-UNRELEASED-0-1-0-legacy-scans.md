## [0.1.0] - UNRELEASED

### Added

- Escape, indirection and unscanned rules for `kiln legacy-plan` (KILN03 section
  6, loop item 7c-2), in `internal/legacy/scan.go` and `links.go`:
  - every path token with a `..` segment in a package's own text files is
    resolved against the file's directory; a target outside the package, its
    closure and its reads gives `escape:<path>` (an unresolvable `$VAR/../x`
    token is an escape too);
  - indirection markers (`dirname`, `realpath`, `readlink`, `git rev-parse`,
    `git ls-files`, `$PWD`, `$0`, `$HOME`, `~/`, a bare `cd ..`, ...) and
    absolute paths outside `/dev/null`, `/usr/bin/env`, `/bin/sh` give
    `indirect:<file>`;
  - text over 256 KiB and executable non-text files give `unscanned:<file>`;
  - every symlink in a package's inputs and in every read is fully resolved
    (chains, directory links, 40 hops, loops refused) and must end inside the
    package, its closure or its reads, else the hard reason `symlink:<link>`.
  The text scans (`escape:`, `indirect:`, `unscanned:`) are advisory hints, not
  reasons (see Changed); keys are always computed. The scan shares the single
  read that hashes each file.

### Changed

- Scans are advisory hints, not soundness (KILN03 sections 6, 11.2 and 12
  items 13 to 21): `escape:`, `indirect:` and `unscanned:` findings no longer
  contribute to `reasons` or change a class. They go to a new per-package
  `hints` array in `kiln-legacy-plan/1` (sorted, de-duplicated, 16 per kind)
  and to a separate `histogram.hints`; member and read findings still reach
  dependents, as hints only. Soundness will come from traced read-sets under
  `strace` (KILN04, forthcoming).
- Symlink findings stay hard reasons but are now spelled `symlink:<link>`
  (and `symlink:<build-file>` for a symlinked BUILD), so `escape:` means one
  thing only: a hint.
- `m1-incomplete:scans` is gone; `m1-incomplete:trace` (no traced read-set
  manifest yet) is added beside `m1-incomplete:tools-env`, so no package is
  `cacheable`. A test asserts it for every fixture plan.
- `kiln explain` prints hints in their own `advisory_hints` section, labelled
  advisory.

### Fixed

- The absolute-path indirection rule no longer treats non-paths as paths:
  `///` comment leaders, XML closing tags (`</p>`), and MSVC flags (`/W4`) are
  ignored; a one-slash token needs a well-known root (`/tmp`) or two segments
  (`/a/b`), and a two-slash token needs two segments (`//server/share`).
- Security review of the scan rules (KILN03 section 12 items 13 to 20): the
  rules now apply to every closure member and every read file, so a dependency's
  indirection, unscanned file or escape reaches its dependents; non-text files
  are `unscanned` unless a known inert image, PDF or font format; UTF-16 files
  are decoded and scanned; invalid UTF-8 is scanned as bytes; markers cover
  non-shell location idioms (`__file__`, `__dir__`, `CARGO_MANIFEST_DIR`,
  `getcwd`, ...), PowerShell variables case-insensitively, and `cd..`,
  `pushd`, `chdir` and friends; a bare `..` token is the parent directory; the
  absolute-path rule sees `${X:-/tmp}`, `-isystem/usr/include`, drive and UNC
  paths, `//tmp` and `~user`; `../` targets are walked through symlinks; a
  read glob admits a directory only when every file under it matches; link
  targets with a backslash are refused; link resolution reuses the target
  text that was hashed.
