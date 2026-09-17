## Unreleased — the nuqta, and a note whose lead example was wrong

**Two inventory points close: `HI-A1-SCR-16` and `HI-A1-PH-05`.** SCR-16 was
recorded as *the single sharpest finding in the exam inventory*.

| metric | before → after |
|---|---|
| exam-point coverage | 232/282 → **234/282 (83%)** |
| atoms taught | 522 → 524 |
| measurable lessons | 467 → 469 |
| writing-practice lessons | 113 → 115 |
| reinforcement window misses | 1226 → 1231 |
| never-taught glyphs | unchanged (2) |
| script-closure violations | unchanged (24) |
| `forward-language` | unchanged (22) |
| `atomsNeverRevisited` | unchanged |
| `durationViolations` | unchanged (0) |

### The size of the hole

The mark was never taught. The corpus carries it in **36 headwords**, the first
of them in the second chapter:

| carrier | headwords | examples |
|---|---|---|
| ड़ | 15 | बड़ा, सड़क, घोड़ा, खिड़की |
| ज़ | 7 | ज़रूर, मेज़, दरवाज़ा, बाज़ार |
| फ़ | 6 | सफ़ेद, सफ़र, तोहफ़ा, फ़सल |
| ख़ | 5 | ख़ुशी, ख़ुशबू, बुख़ार |
| ढ़ | 2 | पढ़ना, सीढ़ी |
| क़ | 1 | मुलाक़ात |

**A correction caught before this entry shipped.** The draft claimed a reader
who had done every page could not read **खाना** against **ख़ाना**. That is
false: `HI-C37-khaana` names the nuqta for exactly that pair, and says honestly
that the dot is often left off in print. What no lesson did was **generalise
it** — the mark was a fact about two words rather than a mark, and the other 34
headwords carrying it were unreadable shapes. This is the failure this entry's
own `lessons.d` note is about, found in my own draft on the way out.

### Two lessons, and why not one

`HI-S140-nuqta` draws the dot and the two flaps it makes. `HI-S141-nuqta-borrowed`
draws the four that carry borrowed sounds. The split follows the language: on
**ड** and **ढ** the dot makes a native Hindi flap; on **ज फ ख क** it lets the
letter carry a sound Hindi took in from outside.

### Why it could not come sooner

A lesson that teaches a mark has to be able to show it on letters the reader can
already draw. **ड** was not drawn until chapter 59, so chapter 60 is the first
honest slot. Every carrier — क, ख, ग, ज, ड, ढ, फ — is now drawn, and the two
that were missing until the entry before this one were **ग** and **फ**.

### A correction to the inventory note's own evidence

The SCR-16 note led with **शुक्रिया**, which carries **no nuqta at all**. It is
श + ु + क् + र + ि + य + ा. Fourteen examples offered as proof, one of them
wrong, and nobody had checked — the same *half true* failure catalogued in
`lessons.d`: precise about the phenomenon, wrong about the vocabulary. The note
now lists what the corpus actually contains, counted rather than recalled.

### ग takes the dot in no word this book has

The script data inventories seven carriers. **The corpus uses six.** ग़ appears
in zero headwords, so neither lesson teaches it, and the entry before this one
claimed ग was needed *for the nuqta* when it was needed only on its own account
— a letter nobody had drawn. That justification was half right, and this is the
half that was wrong.

### Neither lesson reaches forward

Every illustration comes from a headword already taught: बड़ा, सड़क, खिड़की,
पढ़ना, सीढ़ी, ज़रूर, मेज़, दरवाज़ा, सफ़ेद, सफ़र, तोहफ़ा, ख़ुशी, ख़ुशबू,
मुलाक़ात. `forward-language` did not move.

### The source says less than a stroke order

The reference for U+093C describes where the dot sits and what it combines with,
and states that it specifies composition and placement rather than a handwriting
sequence. **Carrier-first is therefore presented as a convention of this book,
not a rule handed down**, and the lesson says so in those words.

### Payoffs

| atom | payoff |
|---|---|
| `HI-SCRIPT-RECOG-140` | `HI-C62-wood` — **लकड़ी** |
| `HI-SCRIPT-RECOG-141` | `HI-C63-fever` — **बुख़ार** |

Both chosen for a headword whose spelling needs the dot to be read. Neither
required new prose, so no duration moved.

### What the metrics could not see

`scriptClosureViolations` and `never-taught glyphs` both held still, for the
third entry running. `measureScriptClosure` credits a glyph to any script lesson
whose **body** contains it, and the nuqta is in 81 lesson bodies. By the
corrected check, which credits only a lesson's **headword**, never-drawn goes
**5 → 4**: ञ, ट, ठ and the vocalic-ṛ sign.

`reinforcementWindowMisses` rose by 5, as it has for every pair of letter
lessons in this run: a newly introduced atom has no prior revisits inside the
measurement windows.

### Closing `HI-A1-PH-05`

PH-05 asks for z, f and q **as sounds, not just as spellings**. It closes on a
script atom, the way `HI-A1-PH-01` through `PH-04` already do. `HI-S141` names
each sound, places **ख़** and **क़** further back in the mouth than their plain
letters, says plainly that many speakers merge them, and picks up the one pair
`HI-C37-khaana` had already named. If that is judged too thin for a
pronunciation point, the probe to remove is PH-05's, not SCR-16's.
