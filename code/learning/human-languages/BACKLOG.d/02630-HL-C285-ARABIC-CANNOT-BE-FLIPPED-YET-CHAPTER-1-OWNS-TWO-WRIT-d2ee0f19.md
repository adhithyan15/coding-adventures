## HL-C285 — Arabic cannot be flipped yet: chapter 1 owns two writing ladders, and only one of them is in the book

Arabic is the last of the four small tracks in this retirement wave and the only
one that did not ship. The blocker is not prose and not schema: it is that
Chapter 1 contains **two complete writing ladders for the same five letters**,
and the hand-written `.tex` shows only one of them. Generating the chapter puts
both in front of the reader, in sequence order:

    seq  12  AR-W00-sin-lam             teach س ل
    seq  14  AR-W00-alif-mim-salam      teach ا م, then write سلام
    seq  22  AR-W00-ra-ha               teach ر ح
    seq  24  AR-W00-ba-family-marhaba   teach ب ت ث
    seq  42  AR-W00-ayn-ya              teach ع ي
    seq  44  AR-W00-kaf-waw-alaykum     teach ك و
    seq  46  AR-W00-full-greeting-recall  dictation, no new shape
    ---- and then, in the same chapter ----
    seq  90  AR-W01-direction-and-alif  teach ا            (again)
    seq 100  AR-W01-abjad-short-vowels  short-vowel marks
    seq 110  AR-W02-joining-sin-lam     teach س ل          (again)
    seq 120  AR-W02-lam-alif-joins      the لا ligature
    seq 130  AR-W03-dots-mim-ba-salam   teach م ب          (again)
    seq 140  AR-W03-write-salam         write سلام         (again)

`AR-W03-write-salam` is titled *"سلام — your first whole written word"*. It
arrives 126 sequence steps after `AR-W00-alif-mim-salam` has already written it.
Five letters -- ا س ل م ب -- are each taught twice in one chapter.

**How it got here, and why no gate saw it.** `AR-W01`-`AR-W03` are the original
ladder. PR #12315 ("Chapter 1 speaks first, then writes three shapes at a time")
replaced it with the interleaved `AR-W00` micro-lessons and, in its own words,
*"mirrored the new ramp in the hand-authored book"*. The book was updated; the
superseded lessons were left in the corpus, still filed under `chapter: 1`,
still attached to `AR-PATH-006` and `AR-EXT-006-SCRIPT`.

Where do they render today? **Nowhere in the book.** This was checked rather
than assumed, and the first answer was wrong: the parity script's docstring
describes this condition as writing lessons that "render only in the answer
key", and that is true of Hindi but not of Arabic. Grepping the materialized
compile inputs and the committed `book/` tree for `AR-W01-direction`,
`AR-W02-joining`, `AR-W03-write` and their siblings returns nothing --
`appendix-answer-key.tex` is 31 lines and does not mention them. They survive
only in the lesson corpus and in the generated narration (`arabic/narration/
ch01.json` scripts them for audio). So the reader of the book has never seen
them, and the listener of the narration has heard them twice.

No lesson-level gate reads a `.tex` that is not built from lessons, so nothing
reported any of this.

Chapter 2 has the same shape and is less clear-cut: `AR-W04-dots-family-nun-ta`
re-covers the ب/ت/ث skeleton from `AR-W00-ba-family-marhaba` while adding ن;
`AR-W05-ya-and-my-name` re-covers ي from `AR-W00-ayn-ya`; and
`AR-W06-harakat-and-hamza` appears in the `.tex` as an unmarked hand-written
section rather than a canonical insertion, so it is nearly-live rather than
dead.

**Why this was not resolved here.** Which ladder survives is an authorial call
with a large blast radius -- six to ten lesson files, two curriculum path nodes,
an extension node, narration, modality, and the lesson counts `language-ladder`
pins. The evidence says `AR-W00` is the live one and `AR-W01`-`AR-W03` are
superseded, but "the book currently shows X" is not the same as "the owner wants
X kept", and deleting curriculum content on that inference, inside a PR whose
subject is a config flip, is the wrong place to decide it. It wants its own PR.

**What Arabic also needs, once that is settled.** Twenty-two schema-v1 lessons
across chapters 1-2 must be migrated before `generate:books` will accept them,
and many carry headings the renderer classifies as `unknown` (`## The letters in
this word` is fine; `## The bowl family — a truth-table`, `## Its three jobs`,
`## Build the word`, `## The catch in "uh-oh"` are not).

**A measurement note that changes what the remaining gap means.** Parity reports
Arabic at 9 blocks after this change, all `sounds`. Nearly all of them are an
ENVIRONMENT mismatch rather than missing prose: the hand-written chapters put
letter-shape descriptions (*"Two new letters: ش (shīn) ... and ك (kāf)"*) inside
a `sounds` box, while the lessons file the same sentences under `## The letters
in this word`, which the renderer classifies as `script`. The lessons have it
right. Manufacturing `## Sounds you'll need` sections to drive the counter to
zero would move real script content into a pronunciation box and invent prose to
satisfy a gate -- so the count was left honest instead.

The one block that WAS genuinely misfiled is fixed here: `AR-C01-salam` kept
pure pronunciation prose under `## You'll want to know`, while its two siblings
in the same chapter, `AR-C01-marhaba` and `AR-C01-as-salamu-alaykum`, file
identical content under `## Sounds you'll need`. Gap 10 -> 9.

So `handwritten_parity.py --check arabic` should be expected to keep failing
until the environment question is settled, and the failure is not a queue of
nine paragraphs waiting to be typed. This is the third track in this wave whose
parity number needed a human to interpret it -- see HL-C283 (Italian and
Portuguese resolved the same measurement oppositely, both correctly) and
HL-C284 (Persian, where 4 of 16 orphaned blocks were the defect rather than the
loss).
