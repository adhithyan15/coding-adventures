## [0.1.0] - UNRELEASED

### Added

- `kiln record prepare` (KILN04 stage 4 PR 2b-1, section 4.8, decisions P1 to P10):
  the whole pre-BUILD phase of a recording shard, in `internal/record.Prepare`.
  It checks trust and platform (exit 3, nothing written, unless a trusted GitHub
  run on linux/amd64), pins the commit with `VerifyTree`, hashes the Kiln and
  build-tool binaries and requires them outside the checkout, `tmpdir`, the state
  roots and the `ro_roots`, refuses an untracked `BUILD*` path anywhere on disk
  and any regular file with `nlink > 1`, runs the out-of-tree build tool for the
  graph, steps and sharded plan, writes `host-facts.json`, computes the weak key
  of every package the shard runs, generates and validates `sandbox.json`, scans
  every `ro_roots` entry for sockets and FIFOs, runs the sandbox's own
  `CheckHost`, and writes `binding.json` and `SEALED` last, byte for byte as
  `recfmt` encodes them, then makes `pre/` read-only.
- `kiln record step` and `kiln record collect` as stubs that refuse (PR 2b-2), so
  the command surface and the `-exec-wrapper` argv contract
  (`pre/exec-wrapper.json`) are fixed now.
- `record.ScanTree`, `CheckInventory`, `HostScan`, `ResolveBinary`, `CheckOutside`,
  `CheckRoRoots`, `BuildSandboxConfig`, `MarshalConfig`, `ReadPartition`, and the
  `kiln-record-action/1` file (`EncodeAction`, `DecodeAction`).
- `legacy.RecordExecutorArgs`, `legacy.RecordEnviron`, `Traced.RecordEnv` (a
  package whose key is refused still runs, so it still gets an environment) and
  `Traced.HostRoots`; `recfmt.CheckPath`.
- The sandbox configuration declares `~/.cargo/registry` and `~/.cargo/git` as
  state roots when a tool root lies under `~/.cargo` (a closed anchor table: any
  other anchor, such as `~/.local`, is refused), requires every state root empty
  (`unscrubbed:state-preloaded`), and refuses a symlink root beneath home.

### Specification

- KILN04 section 4.8: every item of the two "Pending for PR 2b" lists is marked
  2b-1 (done by `prepare`) or 2b-2, and decisions P1 to P10 are added. Divergence
  from the design document: gates and toolchain probes, and the in-sandbox socket
  probe, are 2b-2 (they need `legacy.Gate` and the supervisor); `graph_sha256` and
  `steps_sha256` are digests of the file bytes.

### Tests

- A fixture repository and a fake build tool drive about 45 refusal cases (binary
  inside the checkout, dirty tree, untracked `BUILD_linux`, hard-linked tracked
  file, socket or FIFO in an `ro_roots` entry, symlink root beneath home, home under
  `/tmp` with a root beneath it, symlinked `--dir`, record directory inside a read-only
  root, and more) and the byte-exact, deterministic success path (decoded by
  `recfmt`, weak keys recomputed from the host facts as the merge will).
- `TestRealBuildToolExportsAreWhatPrepareReads` builds the real build tool and
  reads its three exports with the readers `prepare` uses (about a minute; skipped
  with `-short` and in a sparse checkout).
