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
    package, its closure or its reads, else `escape:<link>`.
  All are hard reasons (class `uncacheable`); keys are still computed. The scan
  shares the single read that hashes each file.

### Changed

- `m1-incomplete:scans` is gone; `m1-incomplete:tools-env` remains, so still no
  package is `cacheable`. The histogram groups `escape:`, `indirect:` and
  `unscanned:` reasons by prefix.

### Fixed

- The absolute-path indirection rule no longer treats non-paths as paths:
  `///` comment leaders, XML closing tags (`</p>`), and MSVC flags (`/W4`) are
  ignored; a one-slash token needs a well-known root (`/tmp`) or two segments
  (`/a/b`), and a two-slash token needs two segments (`//server/share`).
