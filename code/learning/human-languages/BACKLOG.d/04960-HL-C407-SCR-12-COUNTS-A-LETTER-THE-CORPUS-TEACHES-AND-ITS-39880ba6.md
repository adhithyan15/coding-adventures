## HL-C407 — SCR-12 names a letter the corpus teaches, and its count is one of three different numbers

**Status: CLOSED (2026-09-19).** Found while scoping chapter 103, which needed
**പരീക്ഷ** and therefore needed to know whether the **ക്ഷ** conjunct was taught.

`ML-A1-SCR-12` now names glyphs and codepoints instead of ambiguous English
letter names, states its counting rule, and distinguishes direct script-lesson
headword ownership from the broader script-closure gate's example-based credit.
The current census is **68** Malayalam headword characters across **471**
lessons, **59** with direct owners and **nine** without. A focused test derives
the nine glyphs, their headword-field counts, and their distinct-token counts
from the corpus so the prose cannot drift silently again.

**`ML-A1-SCR-12`'s note** lists nine characters "the corpus uses in headwords and
never teaches", opening with *"sha (14)"* and adding that *"sha alone blocks
fourteen taught words including sari, the first-chapter 'okay'"*.

**Malayalam has two letters that get called *sha*, and the note's identity is
right while its wording invites the wrong one.**

| letter | codepoint | taught? |
|---|---|---|
| **ശ** *śa* | U+0D36 | **no** — no script lesson has it as a headword |
| **ഷ** *ṣa* | U+0D37 | **yes** — `ML-S121-letter-ssa`, sequence 471 |

**ശരി** (*śari*, the "okay") uses U+0D36, so the note means **ശ** and is correct
about which letter is open. But a reader scoping a word containing **ക്ഷ** —
which is **ക** + virama + **ഷ**, the *taught* one — will read "sha is untaught"
and conclude they are adding script debt when they are not. Chapter 103 nearly
did.

**The count is three different numbers, and nobody has re-derived it.**

- the note says **14**
- **12** distinct tokens in headword position contain **ശ**
- those sit across **15** headword fields

**The first draft of this shard explained that gap wrongly, and the wrong
explanation aimed the remedy at the wrong variable.** It said the two numbers
differ because **ശനി** and **വൃശ്ചികം** sit inside multi-word headwords. They do
sit inside multi-word headwords — and that is not what causes it. A token inside
a multi-word headword contributes **one token and one field**, so multi-word
headwords cannot produce a token/field discrepancy at all.

**The entire three-field gap is ശുഭ**, which occupies **four** headword fields:
`ML-C25-shubha-rathri`, `ML-C30-shubha-sayaahnam`, `ML-C31-shubha-madhyaahnam`
and `ML-C31-afternoon-convergence`. Every other token, **ശനി** and **വൃശ്ചികം**
included, sits in exactly one. The variable is **a token repeated across sibling
headwords**, not a token sharing a headword with other tokens.

Which number the note *meant* is not recoverable from the note.

**What closing this needs.** Re-derive the whole nine-character list against the
data, saying explicitly which counting rule is used: distinct tokens, or headword
fields, and — the rule that actually moves this number — whether a token
appearing in several sibling headwords counts once or once per headword. The
multi-word-headword question is a separate one worth settling too (`continuity.ts`
keeps such headwords whole, which argues a token inside one is not a taught token
at all), but it does not explain the 12-against-15 gap and must not be mistaken
for it. Check each of the other eight the way **ഷ**
was checked here: by looking for a script lesson whose headword *is* that
character, not by trusting the list. The note's *"58 of 67 are taught"* headline
moves with whatever the recount finds.

**Do not close this by fixing the count alone.** The nine-character list is the
useful part; if even one entry names a letter that has since been taught, the
list's value is in the re-derivation, not in the number.
