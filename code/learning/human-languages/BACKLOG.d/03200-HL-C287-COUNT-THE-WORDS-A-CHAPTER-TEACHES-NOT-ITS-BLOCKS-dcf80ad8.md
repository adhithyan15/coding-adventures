## HL-C287 — count the WORDS a chapter teaches, not its blocks

Urdu chapter 1 reported a parity gap of **23**: `checkpoint`x6, `rootweb`x4,
`scriptstep`x6, `usage`x7. The real loss was **one paragraph** — the chapter's
own introduction, which the generator replaces with a `chapteropening` built
from `canDo` and the payoff. Every other block's prose was already in the six
lessons, under headings that classify to `script`, `input`, `guided-production`
and `recall`.

**Why the number was 23.** Those four environments are not in `BLOCKS` at all,
so `unportable_blocks` counts them straight off the `.tex` with **no markdown
side to subtract**. HL-C134 introduced that arm for a good reason — an
allowlist of what the TARGET can emit undercounts what the SOURCE would lose —
but it charges full price for prose that survives under a plainer wrapper. A
`\begin{usage}` whose sentences now sit in a `\subsection*{Your turn}` is not
lost writing; it is the same writing in the environment the generator has.

**This is now the third direction the same instrument has failed in.**

  * HL-C134: counting only what the target can emit **undercounts** the loss
    (corpus 140 -> 217).
  * HL-C286 (Arabic ch2): counting only the headings you thought of
    **invents** debt — a gap of 5 that was never there.
  * Here: counting environments with no markdown side **inflates** the loss
    23-fold.

And a fourth, found on German chapter 3 in the same tranche and pointing the
other way again: a gap of **8 blocks** whose real deficit was **ten missing
lessons**, because a word taught in a paragraph inside a surviving `cousinweb`
costs zero blocks and a whole lesson. The gap can be too big or too small, and
the report alone never says which.

**The instrument that actually sizes the work.** Every one of these failures is
the same mistake: block counts measure FORMAT, and the thing at risk is
TEACHING. Two cheap censuses answered it directly for Urdu and should be run
before planning any remaining chapter:

  1. **Taught-token census.** Enumerate every source-script token in the `.tex`
     and name the lesson that owns it. Urdu ch1: 17 distinct tokens, **0**
     owned by nobody, 6 `\section`s against 6 lessons — so the deficit was one
     schema migration, not ten lessons. On German ch3 the same census is what
     would have shown the shortfall the block gap hid.
  2. **Content-word census, both sides.** Normalise LaTeX accent macros and
     Unicode combining marks to a common skeleton, drop stopwords, stem, and
     list the words the hand-written chapter says that the generated one does
     not. Urdu ch1 came back with 72; reading them found **five** real losses
     (the name *nūn ghunna*; that *shukriyā* is an ordinary modern expression;
     that the safe reply to *salām* is *salām*; the instruction to cover the
     romanization before speaking; the Persian-Arabic/Indo-Aryan contrast that
     made *nahī̃*'s etymology a point). Those five were authored into the
     lessons before the flip. The rest were re-wordings and PDF-bookmark
     strings, which is exactly what HL-C134 predicts: the prose was edited on
     its way into LaTeX, so a plain text diff sees everything and says nothing.

Neither census is wired into the repo, deliberately: they are ten-line scripts
whose value is in being read alongside the artifacts by whoever is flipping the
chapter, and a gate that turned 48 stemmed word deltas into a red build would
be the fifth version of this same mistake.

**What to do about the script.** Unchanged from HL-C286: teach `BLOCKS` every
alias `classifyBlock` knows, and report markdown blocks it could not classify
as unknown rather than as a silent zero. Add to that list: give
`unportable_blocks` a markdown side, or stop presenting its output in the same
column as a gap that has one. Still not done here, for the same reason — it
moves every track's number at once while chapters are mid-flight.
