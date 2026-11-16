## Unreleased

- Runs the shared `warrant-corpus-conformance-v1` fixture (VER04i 9.1, WD-100):
  397 cases across `json`, `tagged`, `containment` and `label-digest`, in
  `tests/conformance-fixture.test.ts`.
- **Fix:** `renderJson` sorted keys by UTF-16 code unit (wrong for U+E000..U+FFFF
  against astral keys), emitted integer-like keys in numeric order (JavaScript
  enumerates them first), and left DEL raw. It now renders by hand: code point
  order, DEL as `\u007f`.
- **Fix:** the nesting limit counted scalars, so 257 nested arrays were accepted;
  the limit is now 256 nested containers, as in every lane.
- `relativeInside` treats a device-namespace path (`\\?\`, `\\.\`) as never
  inside anything, on either side.
- New `manifestLabelDigest` in `labels.ts`, the one definition of the label
  digest (toolchain name bound, version ignored).
- `runner.src_sha256` changed, so the `ct-compare` manifests are regenerated
  separately.
