### Added — Malayalam chapter 105, places, where a gate caught the stale note before a reader did

- `ML-A1-LEX-50` closes. Malayalam A1 coverage 211/243 -> **212/243 (87%)**, 31
  points unmapped. The percentage holds — 212/243 is 87.2 — recomputed, not
  carried.

#### The note was stale in the domain mode again, and this time a gate found it

> **naadu** ('home country') **only** — see `ML-A1-N-02`.

**നാട്** is `ML-C55-homeland` — and `ML-C80-uuru` (2760) **also** teaches **ഊര്**,
*"a town, a village, the settled place somebody is from"*, and already builds a
scale out of the two: **ഊര്** is one settlement, **നാട്** floats from a district
to a country.

I verified the five words the note implies are missing — രാജ്യം, നഗരം, തലസ്ഥാനം,
ഗ്രാമം, പട്ടണം, all correctly absent — and **did not check what else the domain
already held**. The duplicate-concept gate did: the draft's **പട്ടണം** collided
with `ML-NOUN-TOWN`, which is `ML-C80-uuru`'s tag.

**That is the fourth stale-note mode in four tranches, and the first one a gate
caught rather than a reader.**

| tranche | how the note failed |
|---|---|
| `ML-A1-TIME-10` | named the wrong **construction** (locative for dative) |
| `ML-A1-LEX-33` | right about its words, implied an empty **domain** |
| `ML-A1-V-20` | simply **false** about what is taught |
| `ML-A1-LEX-50` | right about its words, implied an empty **domain** — again |

#### The collision improved the chapter

**പട്ടണം is dropped, not renamed.** *A town* is a concept this corpus already
teaches, and a second word for it is not a gap. What was genuinely missing is the
**top** of the scale — a word that does *not* float, where നാട് does — the
**large settlement**, and the **functional** word.

| | | |
|---|---|---|
| **രാജ്യം** | *rājyaṁ* | a country — new |
| **നഗരം** | *nagaraṁ* | a city — new |
| **ഊര്** | *ūrŭ* | a settled place — **since chapter 80** |
| **തലസ്ഥാനം** | *talasthānaṁ* | a capital — new |

Three new words instead of four, and the point closes on those plus
`ML-LEX-C80-ORIGIN-01`. The label's four halves are all delivered; one of them was
delivered in chapter 80. `ML-C105-nagaram` now draws its contrast against a word
the learner **owns** rather than one introduced three lessons earlier — better
teaching as well as a smaller chapter.

#### What the city lesson refuses to claim

**That നഗരം and ഊര് divide by size.** They overlap. നഗരം is Sanskrit and formal,
ഊര് is Dravidian and everyday, and the same place can take either. The lesson says
the reliable difference is **register**, not population, rather than teaching a
tidy rule Malayalam does not keep.

#### തലസ്ഥാനം was half-built already

It opens with *tala*, the **head** from `ML-C13`'s body-part bundle. A capital is
a **head-place** — and English says the same thing and hides it, since *capital*
is Latin *caput*. The second half, *sthānaṁ*, is given in **romanization only**,
because no lesson teaches it: the treatment **പള്ളി** and **കൂടം** got in chapter
103.

#### One invented example, caught before the first validate

The draft wrote **ഇന്ത്യ ഒരു രാജ്യം**. **ഒരു is taught nowhere** — it is the open
point `ML-A1-ART-02`, whose note records that this corpus already uses ഒരു
untaught in **six** lessons. Writing a seventh would have deepened a debt the
inventory is actively tracking.

#### The process change worked

The three corpus-wide ceilings were checked **in draft rather than after** —
zero chapter-number references, zero `RULE_PATTERNS` hits, zero banned words,
run as greps before the suite, per the `lessons.d` entry written last tranche.
Every glyph was confirmed script-taught and already in use in headwords before
drafting, so there was no repeat of chapter 103's U+0D20.

**The full suite passed on the first run — the first time in four chapters that
it has.**
