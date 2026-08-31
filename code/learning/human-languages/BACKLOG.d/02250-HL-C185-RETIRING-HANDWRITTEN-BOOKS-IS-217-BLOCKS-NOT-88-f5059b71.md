## HL-C185 — Retiring handwritten books is 217 blocks, not 88

The standing directive is that no hand-written books remain. Measuring the job
first changed its shape three times, and each correction is worth keeping.

`handwritten_parity.py` reported **88 blocks at risk across six Indic tracks**.
Two things were wrong with that number.

**The track list was hardcoded.** `TRACKS = [tamil, telugu, kannada, malayalam,
hindi, sanskrit]`, written when those were the only tracks anyone meant to
migrate. The ledger holds **69 handwritten chapters across 14 tracks**, so 44
were never measured, including all **32 French and German** ones. The list now
comes from the ledger and cannot drift from the corpus again.

**The block list was an allowlist of what the GENERATOR emits.** It counted
`sounds`, `cousinweb`, `grammarlens`, `culture` and nothing else. But a chapter
is handwritten precisely because it is not limited to what the generator can
emit. `cognates` (18), `morphologybox` (17), `usage` (11), `scriptstep` (9),
`checkpoint` (9), `etymology` (7) and `rootweb` (6) appear in these chapters and
in **no generator source path at all** — 77 blocks that vanish on flip without
even a heading to carry across.

Corpus-wide, counting those, the real figure is **217 blocks**.

The generalisable rule: **an allowlist of what the target format supports is the
wrong instrument for measuring what the source would lose.** It can only ever
under-report, and it under-reports hardest exactly where a chapter is most
custom — which is where the risk is.

The correction moved two tracks from safe to blocked. **persian (16) and urdu
(19)** were reported as "no prose blocks at all, nothing would be lost" and
passed `--check`; their chapters are built almost entirely from
`usage`/`scriptstep`/`checkpoint`/`rootweb`. Wiring the gate in as it stood would
have made a green check certify their deletion.

Two further false-green paths were closed at the same time: a missing `.tex`
`continue`d without recording anything, so a renamed chapter or a drifted ledger
path landed the track in the clean bucket with `--check` exiting 0; and the
markdown side counted a heading substring anywhere in the file, so prose
mentioning "taken apart" scored an etymology block (live on French ch2), which
shrinks the gap and reports a chapter safer than it is.

Sequencing this implies:

- **Retire first:** italian, portuguese, punjabi, sanskrit — prose already
  carried into the lessons, `--check` passes. Sanskrit at 5 chapters is the
  natural first real retirement.
- **Carry the prose across first:** german (78), french (60), urdu (19),
  persian (16), arabic (10), marathi (9), telugu (7), malayalam (7), kannada (6),
  tamil (5).

`--check [track ...]` is the blocking form, and a track with no handwritten
chapters left passes trivially, so the gate survives its own success rather than
going red the moment a track is retired.

What the measure still does NOT say: whether the surviving prose says the same
thing. It counts blocks, because the prose was rewrapped and rephrased on its way
into LaTeX and a text diff would call every block different. The side-by-side
read stays a human step, per chapter.
