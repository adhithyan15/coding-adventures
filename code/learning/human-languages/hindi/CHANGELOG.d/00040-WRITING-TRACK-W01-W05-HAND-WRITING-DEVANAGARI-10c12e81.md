## Writing track W01–W05 — hand-writing Devanagari

The first writing lessons for **any Indic track**. Ten Indic/Dravidian tracks had
reached Chapter 5 with **zero** writing lessons; this opens the first of them,
modelled on Arabic `AR-W01–12` and Russian `RU-W01–05`.

The arc is built so every lesson assembles vocabulary the learner already has,
and the last one produces the **first word of the course**:

- **`HI-W01-shirorekha-na-ma`** — the **shirorekhā**, the line Devanagari hangs
  from. शिरोरेखा is literally "**head-line**" (*śiras* + *rekhā*), and *śiras*
  descends from PIE \**ḱerh₂-* "head, horn" — the root behind Latin *cornu*,
  Latin *cerebrum* and English **horn**. Two counter-intuitive habits taken
  straight from the data: it is drawn **last**, and across a word it is **one
  bar** — both flagged in the lesson as the **common convention, not a rule**,
  since plenty of writers cap each letter as they go. Plus न and म and the
  commonest frame — **spine on the right, character on the left, bar across the
  top** — stated as *commonest* rather than universal, with the spineless
  minority (**द**, **र**) named up front.
- **`HI-W02-abugida-ka-ta`** — the **inherent vowel**, met head-on: **क is "ka",
  not "k"**. Which retroactively upgrades W01: नम was never "nm", it was *nama*,
  the first half of *namaste*. Names the script type — an **abugida** — and gets
  the coinage right, which is more interesting than the version usually told:
  *ʾä-bu-gi-da* names the first four **consonants of the old Semitic order** *and*
  the first four **Ge'ez vowel series** simultaneously, so unlike *alphabet*
  (which merely recites *alpha-beta*) the term **demonstrates the system it
  names**. Noted too that Ge'ez's own recitation order is *hä-lä-ḥä-mä…*, so this
  is a scholar's coinage, not the Ethiopian schoolroom sequence. Plus क, त, and
  the **five stop families** sorted by place of articulation, soft palate forward
  to the lips (क → च → ट → त → प), analysed by Indian phoneticians — Pāṇini is
  roughly 4th c. BCE — millennia before Europe attempted the same.
- **`HI-W03-matras-naam`** — **mātrās**. मात्रा means "a **measure**", from *mā-*
  "to measure" (PIE \**meh₁-*, whence *meter*, *measure*, *immense*, and *month*
  — the moon as the original measuring instrument). The load-bearing point is
  that a mātrā **replaces** the inherent vowel rather than adding to it. Teaches
  ा and े, builds **नाम** (Ch. 2), and closes on **ि** — *the only Hindi mātrā*
  written **before** the consonant and pronounced **after**. The lesson declines
  to explain why: the placement is an inherited Brahmi quirk, and an invented
  rationalisation would be worse than none.
- **`HI-W04-ra-sa-mera-naam`** — र, the **first spineless letter the learner
  writes** (with **द** named so it doesn't read as unique — द has in fact already
  been *read*, in Ch. 1's *dhanyavād*), and स. Carries a long-range
  etymology told with its uncertainty intact: *šin* → **Σ** → **S** going west is
  certain; **Brahmi**'s descent from the same Semitic family via **Aramaic** is
  the **leading view, not consensus**, so the lesson says "probably cousins," not
  "cousins." Builds **मेरा नाम**, and gets the word-boundary story the right way
  round: modern Hindi uses an **ordinary space** like English, and the break in
  the bar is the **consequence** of that space, not the device that marks it.
  (Continuous bars across a whole line belong to spaceless Sanskrit manuscripts.)
- **`HI-W05-virama-namaste`** — the **virama** ्, which kills the inherent vowel.
  विराम is "a **stopping**" (*vi-* + *ram-*) — and rather than the loose claim
  that it *is* the Hindi full stop, the lesson shows the word **grading Hindi's
  punctuation**: *pūrṇ virām* (complete stop) = full stop, *alp virām* (slight
  stop) = comma, *ardh virām* (half stop) = semicolon. Also names the mark
  honestly: *virāma* is the Sanskrit/Unicode term, **halant** is what everyday
  Hindi calls it. Then what actually happens in handwriting: the bare consonant
  **fuses** with the next into a **conjunct** (स् + त → स्त), a **spine-bearing**
  first consonant surrendering its spine — scoped that way because र, taught one
  lesson earlier, has no spine to surrender and uses **repha**/**ra-kāra**
  instead (र् + क → र्क, क + ्र → क्र), while क्ष/त्र/ज्ञ are simply irregular.
  Assembles **नमस्ते**, then reveals
  what the learner has been saying since lesson one: *namas* ("a **bow**", ←
  *nam-* "to bend") + *te* ("to you"). The greeting **is** the bow. *Namaskār* is
  the same *namas* + *kāra*, "making".

### Data honesty

Everything the learner is asked to **hand-write** comes from
`data/scripts/devanagari.json` — 28 letters and 12 marks. Every such letter,
mātrā and mark has a real entry with real `components` and `strokeOrder`;
**nothing was invented**.

**Six** letters appear somewhere in the text without entries — **ख ज ञ ट ण ष** —
and none of them is ever presented as something to draw. Two are cited as letters
(**ट** in W02's articulation chart, **ख** inside the word *shirorekhā*) and each
carries an explicit "read it now, draw it when its entry is written" note. The
other four only occur *inside quoted Hindi words* — ण in *pūrṇ virām*, ष in क्ष,
ज and ञ in ज्ञ — where the word is the point and the glyph is incidental.

To be precise about the guarantee, since a vaguer version of it was wrong in an
earlier draft of this entry: **every letter this track asks you to hand-write has
a real entry with real `components` and `strokeOrder`.** It is not true, and is
not claimed, that every Devanagari character appearing anywhere in the prose has
one.

The file is marked `complete: false` (its inventory covers the
greeting/self-introduction vocabulary), and its stroke orders are flagged as the
common handwriting convention rather than a standard — so **W01 says the same in
the lesson text**, since it would otherwise state "draw the bar last, one bar per
word" as flat rules when many writers cap each letter as they go.

### Scope note

`devanagari.json` serves **three** tracks — Hindi, **Marathi** and **Sanskrit**.
This PR opens only the Hindi one; the other two can mirror this arc against the
same data, and **नमस्ते is identical in all three**.

**Still blocked, stated rather than skipped:** among Indic scripts only
`devanagari.json` and `gujarati.json` exist. **Tamil, Telugu, Kannada, Malayalam,
Bengali and Gurmukhi have no letter data at all**, so no writing track is
possible for them until that data is authored — a real piece of work, not an
oversight.

