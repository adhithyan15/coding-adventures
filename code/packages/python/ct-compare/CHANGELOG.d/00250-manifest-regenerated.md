## Unreleased

- `evidence/test-manifest.json` regenerated with this lane's own `run --out` command: the shared `warrant-corpus` runner
  gained `ir.fixed` and `ir.tuple` (CV2 S10e), which moved `runner.src_sha256`. Nothing else changed; this is not an
  adopter change. This lane still runs `warrant-corpus-v1`: dropping `not_live` to 0 for the `eq_u64` control (S10e's
  last item) needs the v2 corpus reader of S6b and S6c and is deferred to them.
