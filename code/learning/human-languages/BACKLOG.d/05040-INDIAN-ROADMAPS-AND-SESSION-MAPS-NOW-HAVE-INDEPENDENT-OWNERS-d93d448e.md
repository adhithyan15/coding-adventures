## HL-C415-d4e1ff4b — Indian roadmaps and session maps now have independent owners

**Status: CLOSED (2026-09-26). Tracks #15743.** The contention audit found
chapter agents repeatedly editing the same per-track planning aggregates even
after lesson, curriculum, changelog, and regression ownership had been split.

All twelve Indian tracks now keep roadmap sections and numbered session rows in
strict document shards. The committed owners reconstruct the original bytes,
use deterministic stable identities, reject unsafe or duplicate fragments, and
leave the generated `roadmap.md` and `session-map.md` views untracked. Ordinary
chapter work can now change its planning owner without rewriting a track-wide
document.
