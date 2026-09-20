## Six characters the book was already using and had never taught

`TE-A1-L-04` (the vowel signs) and `TE-A1-L-12` (the retroflex row) close.
Telugu A1 coverage **214/326 → 216/326**.

| metric | before → after |
|---|---|
| exam-point coverage | 214/326 → **216/326 (66%)** |
| characters used but untaught | **15 → 9** |
| uses of an untaught character | **~2,450 → 754** |
| atoms taught | 464 → 470 |
| measurable lessons | 353 → 360 |
| `forwardReferences` | unchanged |
| `scriptClosureViolations` | unchanged |
| `durationViolations` | unchanged (0) |
| `atomsNeverRevisited` | unchanged (8) |
| `payoffSurprises` | unchanged (0) |
| reinforcement-window misses | 844 → 864 |

### What was actually wrong

Counted over every Telugu lesson body, **fifteen characters appeared on this
track's own pages with nothing teaching them**:

> మ 756 · ట 362 · అ 282 · ో 224 · బ 180 · ొ 175 · శ 126 · ఆ 97 · ధ 92 ·
> ష 72 · ఏ 57 · ఐ 13 · ఒ 12 · ఫ 4 · ఠ 3

**మ is the most-used character in the track.** It sits in the second syllable of
**నమస్కారం**, the very first word the book teaches, and `TE-S109-letter-na` prints
that word and **ధన్యవాదములు** as its own examples — showing both మ and ధ on the
page while teaching neither.

**The gate said none of this was happening.** `measureScriptClosure` reported
`neverTaughtGlyphs: 0` and `taughtGlyphs: 64`, because it credits a glyph to any
script lesson whose *body* contains it. Under the rule that a glyph counts as
taught only when a script lesson's **headword** is a glyph inventory, 49 were
taught. The metric understated the debt by fifteen characters.

### Two of the three exam notes were wrong

- `TE-A1-L-10` counted **"ta 292 times"** among untaught consonants. **Dental త
  has been taught since chapter 5** by `TE-S01-copy-in-a-word`. The 292-count
  letter is *retroflex* ట — the claim had been duplicated from `TE-A1-L-12`.
- `TE-A1-L-04` called **ూ** the *oo* sign. That is the **uu** sign, taught at
  chapter 7. The genuinely untaught pair was ొ and ో.

Both notes are corrected in place.

### The six, and where each sits

**This track teaches a letter just after the word that needs it**, not before —
its script lessons are titled *"recognised inside words you already say"*. Each
of the six is placed one slot after its own first use, wedged at
content-sequence + 1 exactly as `TE-S136` through `TE-S139` already are:

| character | placed after | because |
|---|---|---|
| మ, ధ | నమస్కారం, ధన్యవాదములు | the book's first two words |
| ట, ో | ఏమిటి, సంతోషం | chapter two's question and its answer |
| ొ | తొమ్మిది | the number nine |
| ఠ | జ్యేష్ఠం | a month name |

**ఠ is rare — three uses — and is taught anyway.** A reader who meets an
unfamiliar shape inside a familiar list has no way to tell whether the word or
the printing is at fault.

### `TE-A1-L-10` stays open, deliberately

మ and ధ are taught because they cost a reader most. The point itself asks for
the consonant set as *something a reader can finish a page with*, and **eleven
consonants are still untaught** — బ (180 uses), శ (126), ష (72) and ఫ (4) among
them. Probing it now would claim a range the corpus does not teach.

### Two regressions the snapshot diff caught

- **`payoffSurprises` 0 → 1.** A fifth introduced atom took chapter 16's payoff
  to 2/5, under the 0.5 floor. **ఠ moved to chapter 17** — one slot later than
  its first use, which this track's design already allows.
- **`atomsNeverRevisited` 8 → 12.** Four of the six atoms had nothing later to
  revisit them. Each lesson now practises the one before it, and
  `TE-S156-script-recall` introduces nothing and closes the chain.

Neither was visible in any of the twelve gates.

110 exam points remain open. Coverage means the teaching exists, not that a
reader scores.


