## HL-C383 — the never-taught-glyph metric counts a letter as taught when it only appears inside another letter lesson, and the real Hindi alphabet debt is sixteen glyphs rather than six

`measureScriptClosure` credits a glyph to any lesson with `type: writing` or
`delivery: script` whose **body** contains it. Every letter lesson ends with a
list of words the learner already says so the new shape can be found inside
them, and those words are full of **other** letters — each silently marked
taught by a lesson that was teaching something else.

`HI-S117-letter-ca` teaches **च** and lists **एक दो तीन चार पाँच**. From that
line the measure concludes **ए** is taught. Nobody has drawn **ए** for a
learner.

**The 29 Hindi script lessons have exactly these subjects:**

```
म न अ आ ् ा त स र क े प ह य ी च ँ ष ु ल ख द ध ऋ थ उ ऊ ओ छ
```

**Reported never-taught (6):** ः इ औ झ ढ ौ

**Never taught, but credited anyway (10):**

| glyph | what it is | why it matters |
|---|---|---|
| ए | independent vowel *e* | `HI-A1-V-21`'s note says exactly this, and it is right |
| ट | retroflex *ṭa* | **ट्रेन**, ठीक-adjacent forms, every English loan |
| ग घ ज ञ ठ फ | six more consonants | ordinary A1 text |
| ़ | the **nuqta** | `HI-A1-SCR-16`, the sharpest finding in the inventory |
| ृ | the vocalic-r **sign** | ऋ is taught as a letter; the sign never is |

**True figure: 16, not 6.** `scriptClosureViolations` (30) understates the same
way, because a lesson using one of these ten is never counted in debt for it.

**This reconciles a disagreement nobody had resolved.** `HI-A1-SCR-15` says
fifteen consonants ordinary A1 text needs are untaught; `HI-A1-SCR-16` calls
the nuqta the single sharpest finding in the file, because the corpus's own
headwords are full of *shukriyā*, *zarūr*, *darvāzā*, *mez*, *sabzī*, *safed*.
The metric said six. **The inventory was right and the metric was wrong**, and
the two had been contradicting each other unnoticed for the whole campaign.

### What to do

1. **Change what "taught" means in the audit helper**, not in the gate. Build
   the taught set from **headwords**; keep the body-based set as *exposure*.
   Do not change `measureScriptClosure` itself in the same change as a content
   chapter — a metric that moves 6 → 16 and 30 → higher will look like a
   regression in every snapshot diff, so it wants its own PR with the jump
   explained.
2. **Re-prioritise the alphabet work.** The cheap-first order by violations
   actually cleared, with the earliest corpus use that constrains placement:

   | glyph | clears | earliest use | source needed |
   |---|---|---|---|
   | ौ | **3** (`HI-C34-padhna`, `HI-C40-where`, `HI-C76-tenth`) | ch16 | no — it is a mātrā |
   | ः | **1** (`HI-C01-namaste`) | ch1 | no, but ch1 leaves nowhere to precede it |
   | औ + झ + ढ | **1** (`HI-C78-pehla-paath`) together | ch85 | ढ and झ sourced |
   | ट ग घ ज ठ फ ए ़ ृ | not counted today | ch1–ch13 | mostly sourced |

   **ौ is still the best single move** and needs no source data. It does **not**
   unblock **और**, which begins with the independent **औ** — that was wrong in
   HL-C381 and is corrected here.
3. **Expect the ten to be expensive to place.** Their earliest uses are in
   chapters 1–13, where the track has the least room, so several will need a
   host chapter earlier than any existing script lesson. That is the real
   reason they were never written, and it was hidden by a metric that said
   they were done.

**Do not treat the jump as a regression.** Nothing got worse; the count got
honest.
