## [0.1.0] - 2026-11-24

### Added

- PIF-003a (VER32 3.7, 5.3 R-IMP-1/2, E1 to E5): the new crate `pif-check` (library plus the `pif-check` binary), trusted-base id `tcb.pif.gate-binding`.
- `resolver`: a digest-verifying import resolver. `Source` is untrusted (a directory of `<64 hex>.pif` files, or memory); `Resolver::resolve` hashes every file with the repository's own SHA-256 and refuses a mismatch (`DigestMismatch`). No network access; symbolic links and non-regular files are refused; the size is capped before the file is read.
- `control` and `chain`: the strict chain, sign-off and allow-list file formats, role and shape rules (prelude first, roles in order, once-only roles, claim last: `PreludeNotFirst`, `RoleOrder`, `DuplicateRole`, `ClaimNotLast`, at most 65 files), `load_chain` (each file read with the right `FileClass` and the digests of all earlier files as its expected imports), the base and full environment digests (8.3), the allow-list check for base files (E4) and the sign-off base-chain and base-environment checks (E1, E6).
- `cli`, `report`, `json`: `pif-check --store <dir> --chain <file> [--signoff <file>] [--allow <file>]`, a one-line ASCII JSON report, exit 0/1/2.
- Tests: the prelude alone; the prelude plus the worked example's theorem as a claim file; digest mismatch; wrong import list; missing file; reordered chain; claim file in a base position; allow-list; sign-off; store hardening; CLI exit statuses (in process and through the binary).
