## 2026-10-26 — Mosaic manifest dependencies and repository-wide Mosaic guards

- The resolver reads `mosaic-package.toml` for language-`mosaic` packages
  (`code/packages/mosaic/*`, `code/programs/mosaic/*`). Keys of the root
  `[dependencies]` table
  resolve within the Mosaic scope by directory basename or `[package].name`
  (each name owned by exactly one package). Before this, Mosaic packages had no edges,
  so bumping or editing `mosaic-pkg-toolkit` never re-ran task-app, engram-app,
  journal-app or the packages built on it. Requirement strings, comments,
  `[components]` exports, `[host_assets].dependencies` third-party
  coordinates, external names and self-references never create edges;
  multi-line values are tracked by bracket depth (`internal/resolver/mosaic.go`).
- The Mosaic reader fails closed (`MosaicManifestError`), because a dropped
  edge silently skips a dependent's tests on every later PR. It reads only a
  plain subset: one-part headers from a fixed list (`[package]`,
  `[dependencies]`, `[components]`, `[kernel]`, `[styles]`, `[app]`,
  `[host_assets]`, `[host_effects]` — every table the repository uses), and
  bare keys with one-line string values in `[package]` and `[dependencies]`.
  Refused with an error: any other header (`[package.meta]`, dotted, quoted,
  `[[...]]`), any root-level key or inline table (`package.name = ...`), a multi-line string, any backslash
  escape (keys, headers and `[package].name` alike), a manifest that is not a
  regular file or exceeds 1 MiB, and two Mosaic packages claiming one name —
  including a library taking an app's name (`checkMosaicNameCollisions`) — all
  stop `ResolveDependencies` with an error. None occurs in the repository. A
  leading UTF-8 byte-order mark is stripped, as `toml_edit` does. A
  dependency key without the `mosaic-pkg-` prefix also depends on
  `mosaic-pkg-<key>`, as the resolver's directory locator tries that first.
- New `repository_inputs.go`: a table of *repository input consumers*,
  packages whose tests walk the whole tree. Its first entry selects
  `rust/mosaic-package-resolver` (`every_repository_requirement_is_satisfied`
  and the layout-binding guard) for any changed `mosaic-package.toml`,
  `.mil`, `.mll` or `.msl` under `code/`. Both the dry-run/build path and
  every per-platform plan add the consumer and its prerequisites but not its
  dependents; seeding it as a changed package instead measured 72 affected
  packages for a one-line `.mll` edit. Raw Git paths, no stat (deletions
  select), `-language` honored, and a missing consumer fails the plan.
- Measured on the repository with `-dry-run -diff-base origin/main`: a
  toolkit version bump went from 1 to 35 affected packages, an app `.mll`
  edit from 1 to 19, both now including the guard and neither including the
  guard's dependents (artifact builder, emitters); a Python/Go-only change is
  identical (168 → 168). Mosaic-only changes now also request the Rust
  toolchain in the plan, because the guard is a Rust crate.
