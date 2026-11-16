## Unreleased

- Runs the shared `warrant-corpus-conformance-v1` fixture (VER04i 9.1, WD-100):
  397 cases across `json`, `tagged`, `containment` and `label-digest`, in
  `conformance_test.go`; the Go lane now passes the kit.
- **Fix:** the canonical writer escapes DEL (U+007F) as `\u007f` instead of
  writing it raw.
- **Fix:** the reader allows at most 256 nested containers; it used to admit 257,
  and scalars no longer count as a level.
- **Fix (containment):** `relativeInside(root, child, flavor)` is lexical (anchor
  plus whole components) and takes a `posix` or `win32` flavour, so the win32,
  device-path and exactly-two-leading-slashes rules run on any host; production
  passes `hostFlavor`. It replaces the `filepath.Rel` based check and its
  `pathOps` seam.
- New `manifestLabelDigest`, the one place the label digest is defined.
- **Fix (security review, low):** `check` refuses a committed `toolchain.version`
  that is not text; `sabotage` children run under a 10-minute timeout and are
  killed (a timed-out child is reported `not ok`). An unknown or inapplicable
  op after `<fault>:` still exits 0, as in the Python and TypeScript lanes.
- `runner.src_sha256` changed, so the `go/ct-compare` manifest and labels are
  regenerated.
