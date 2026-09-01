## HL-C217 — Sanskrit's last nine untaught Devanagari characters are blocked on shared files and on vocabulary

Nine recognition segments (`SA-S115`–`SA-S123`) took Sanskrit's never-taught
Devanagari set from **18 characters to 9**. What remains is six letters —
**इ ई ऋ घ ङ ड** — and three marks — **ँ ू ◌ै**. Each is blocked for a reason
worth writing down rather than retrying blindly.

**Blocked on a shared data file (do not attempt this from a single-track
branch).** `data/scripts/devanagari.json` carries a cited stroke order for every
one of the 42 letters it lists, and **ऋ and ङ are not among them**. There is
therefore no sourced pen path for either, and HL11 §5 binds: no citation, no pen
path. Adding them means editing a file that Hindi, Marathi and Marwadi also
read. The same holds for `data/scripts/devanagari-ledger.json`, whose 24
positions Sanskrit has now overrun — this tranche taught ट, ज, उ, ओ, फ and ◌ौ,
none of which the shared ledger contains. The ledger is authored intent and it
is now behind the corpus it describes. Extend it from a branch that owns the
shared file.

**Blocked on vocabulary, not on data.** इ, ई, घ, ड, ँ, ू and ◌ै all have
committed stroke or attachment data, and every one of them fails the same test:
the track has **no headword containing it**. They occur only inside
metalinguistic notes — the bare letter इ in `SA-C09-sahayyam-karoti`, ई in
`SA-C11-bhagini`, ऋ wherever the vocalic ṛ is discussed. A recognition segment
whose "you already say these" list is empty teaches a shape for nothing, so the
honest order is **vocabulary first**: schedule words that carry these characters
(इदानीम् already sits in ch. 28 and would anchor इ), then teach the shape.

**What the remaining 46 closure violations actually are.** They are dominated by
chapters 1–13 — lessons EARLIER than any script lesson, so no amount of teaching
later can reach them. Two levers exist and neither is another script lesson: add
`romanization` to the four Sanskrit headwords still missing it, which flips each
headword from load-bearing to exposure and is a real gain for the learner; or
move script segments earlier in their chapters, which HL08's own placement
measurement says costs drivable prefix. Measure both before choosing.
