## HL-C446-f9052df3 — Filmstrip generated artifacts now have independent track and script owners

**Status: CLOSED — implemented by #16094.** The post-#16090 contention audit
found the filmstrip aggregate set at 10 recent touches per file. Every track's
figure generation rewrote one 100 KB hash manifest, and every new cited glyph
rewrote one 3 MB geometry ledger. The corpus count ratchets lived together in
the same TypeScript assertion.

Figure hashes now have one generated owner per track, filmstrip geometry has
one generated owner per script, and filmstrip count ratchets have one strict
owner per track. Loaders deterministically rebuild the public aggregate,
reject malformed or duplicate owners, and generation checks the exact owner
set so stale files cannot survive a removal.

After removing already-sharded level-gate and letter-anchoring files from the
same audit window, the next shared hotspot is
`tests/integration.test.ts` at seven recent touches. Its real-corpus assertions
mix unrelated track facts in one module; the next bounded concurrency pass
should move track-specific assertions behind independent evidence modules.
