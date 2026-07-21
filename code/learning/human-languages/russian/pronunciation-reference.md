# Russian / Русский — Pronunciation & Cyrillic Reference

A **reference**, not a chapter — a place to look things up. You are *not* meant
to read this before the lessons: the lessons teach reading through real words,
introducing each letter as a word needs it. This page gathers the system in one
spot. Full letter-by-letter decomposition (components + stroke order) lives in
the machine-readable [`data/scripts/cyrillic.json`](../data/scripts/cyrillic.json).

## The one thing that trips everyone: false friends

Cyrillic descends, like the Latin and Greek alphabets, from the Greek alphabet —
so some letters are shared, but others look identical to Latin letters and say
something completely different. These four cause 90% of beginner misreadings:

| Cyrillic | looks like Latin… | actually says | mnemonic |
|---|---|---|---|
| **в** | B | **v** | Greek *beta* drifted to a "v" sound |
| **р** | P | **r** | Greek *rho* |
| **с** | C | **s** | Greek *sigma* (a "c" that hisses) |
| **н** | H | **n** | Greek *eta* / Latin N with a level bar |

Two vowels also mislead: **у** looks like Latin *y* but says **"oo"**; **х**
looks like Latin *x* but says a raspy **"kh."**

## The letters, grouped by how they behave

- **Same look, same sound**: **а** (a), **е** (ye), **к** (k), **м** (m),
  **о** (o), **т** (t).
- **False friends** (look Latin, sound different): **в** (v), **р** (r),
  **с** (s), **н** (n), **у** (oo), **х** (kh).
- **Greek-shaped**: **г** (g, gamma Γ), **д** (d, delta Δ), **л** (l, lambda Λ),
  **п** (p, pi Π), **ф** (f, phi Φ).
- **Sounds English writes with two letters**: **ж** (zh, as in *measure*),
  **ч** (ch), **ш** (sh), **щ** (shch), **ц** (ts), **ю** (yu), **я** (ya),
  **ё** (yo).
- **The vowel English hasn't got**: **ы** (`yery-vowel`) — not the *ee* of **и**,
  but a tighter sound made with the tongue **drawn back**. Say English *ill*, then
  say it again with your tongue pulled towards your throat. It is the single
  hardest Russian vowel for an English speaker, and it carries real meaning:
  **ты** (*you*) and **вы** (*you*, formal) both turn on it.
- **The two signs** — **ъ** (hard sign) and **ь** (soft sign) — spell *no sound
  of their own*; they only tell you how to pronounce the consonant before them
  (ь softens it).

## Two rules that make you sound native

- **Stress is unmarked and it matters.** Russian never prints its stress
  accent, but stress governs the vowels — so you learn each word *with* its
  stress. (This reference marks it with an acute, *privét*, but real text won't.)
- **Unstressed о → "a" (akanye).** An **о** not under stress reduces toward an
  *a*-ish sound: *спасибо* is said *"spa-SEE-ba,"* *хорошо* as *"kha-ra-SHO."*

## Sound ids used in the lessons

The lessons' `sounds:` frontmatter points here: `cyrillic-false-friends`
(the table above), `stress-unmarked` and `o-reduction` (the two rules),
`e-ye` (е carries a *y*-glide), `zh-sound` (ж), `b-vs-v` (б vs в), `silent-v`
(the dropped в in *здравствуйте*), `syllable-drop` (spoken contractions like
*pa-ZHAL-sta*), `a-clear` / `a-father` (the pure *a*).

## Russian in the Slavic family

**Slavic** is a branch of **Indo-European**, so Russian is a distant cousin of
English, Latin, and Sanskrit — which is why its deepest words (*нет* ← *\*ne*,
*есть* "is" ← the same root as English *is*) line up with words you already own.
Cyrillic itself was built in the 9th century from Greek letters (plus a few
invented for Slavic sounds Greek lacked) to write Old Church Slavonic — which is
why so many letters are Greek in disguise.
