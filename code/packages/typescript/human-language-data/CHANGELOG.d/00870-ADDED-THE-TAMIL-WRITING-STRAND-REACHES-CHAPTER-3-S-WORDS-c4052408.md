### Added — the Tamil writing strand reaches chapter 3's words

The chapter 2 pass left six lessons still teaching script inline (`TA-C02-nii-niingal`
plus all five of chapter 3), and the same rule applied: the glyphs they explained —
ங, ள, ீ, ா, ட and ு — were taught nowhere in the strand, so deleting the sections
would have deleted the only explanation those letters had. Four new reading lessons
close that:

- **`TA-W10-read-naan`** (chapter 25) — the **ா** sign, spelling **நான்**. This is the
  one sign with a sourced description in `data/scripts/tamil.json` — *"a vertical
  stroke with a small top hook, written after the consonant"* — and it completes the
  picture of where a vowel sign can sit: after (ா), above-right (ி), before (ை, ெ).
- **`TA-W11-read-niingal`** (chapter 27) — **ங**, **ள** and the **ீ** sign, spelling
  **நீங்கள்**. It pays two debts the corpus had been carrying: `TA-W01` explained that
  **க** sounds like *g* after a nasal and printed **ங்க** without ever saying what **ங**
  was, and `TA-W06` promised three *l*-letters while teaching only **ல**.
- **`TA-W12-read-eppadi`** (chapter 29) — **ட**, spelling **எப்படி**, held against the
  **ண** the learner already has: the same retroflex curl, stopped rather than nasal.
- **`TA-W13-read-irukkirirgal`** (chapter 31) — the **ு** sign, spelling
  **இருக்கிறீர்கள்** and then the whole chapter 3 question off the page.

All four are reading-only. Of the six glyphs, only **ா** appears in
`data/scripts/tamil.json` at all (under `marks`, not `letters`), so it is the only one
described from a source. The other five say plainly that the book has no entry for
them, and the hedge covers the **sound** descriptions as well as the missing stroke
orders: **ங**, **ள** and **ட** lean on **ண**'s sourced "tongue curled back" and **க**'s
sourced position-picks-the-sound note, and say so rather than asserting their own
phonetics flat. `TA-W11` marks the same attestation gap for **ீ** that `TA-W09` marked
for **ெ**.

This started as three lessons and became four because the corpus caps a lesson under
300 effective seconds. The first draft put **எப்படி** and **இருக்கிறீர்கள்** in one
lesson and computed at 380s; splitting them is what the cap is for, and the result is
gentler than the draft was.

