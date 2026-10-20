## Fixed — drivable lessons stop telling a driver to write

The modality manifest marks 102 lessons in chapters 7-18 and 131-137, and
their review lessons, `drivable: true`, but each still asked for writing in
bare prose ("Write **い**, **ち**, and **と**.", "Copy **あね**, hide it, and
write it from the meaning.", "4. **Write:** hear the word, wait ten seconds,
and write all five signs."). Narration reads bare prose unhedged, so the
audio edition told a driver to write (issue #12070). Each writing task is now
a `[YOU WRITE: …]` cue, as the Chinese track did in #17014: the narration
defers it ("[once you have stopped driving — write: …]") and the book prints
it as "*Write it:* …". The cue does not create a writing block, so every
lesson stays drivable.

- **Lessons:** JA-C01-practice; JA-C08-hear-sayounara, -sayounara;
  JA-C09-ichido, -mou, -mou-ichido-onegaishimasu, -onegaishimasu,
  -sumimasen; JA-C10-itte-kudasai, -koko, -mou-sukoshi, -slower-please,
  -sukoshi, -wakarimashita, -yukkuri; JA-C11-ashi, -body-map, -hana, -kao,
  -kuchi, -me-eye, -mimi, -te-hand; JA-C12-atama, -body-map-two, -ha-tooth,
  -kami, -kata, -koshi, -onaka, -senaka; JA-C13-ane, -ani, -chichi, -haha,
  -imouto, -kodomo, -otouto, -otto, -tsuma; JA-C131-chotto, -isha, -ocha,
  -ocha-o-kudasai, -toshokan; JA-C132-kore, -kuruma, -soko, -sore, -soto;
  JA-C133-eki, -heya, -ike, -inu, -kesa, -kinou, -nuno; JA-C14-go, -ichi,
  -ni, -san, -yon; JA-C15-juu, -ku, -nana, -roku; JA-C16-futatsu, -hitotsu,
  -itsutsu, -mittsu; JA-C17-kokonotsu, -muttsu, -nanatsu, -too, -yattsu;
  JA-C18-hitori-futari, -hon, -nin; JA-R12-body-01, -doorway, -farewell,
  -foundation-01, -foundation-02, -mixed-scripts, -repair-01 to -04,
  -writing-01, -writing-02; JA-R13-family-a to -d, -tsu-contrast;
  JA-R131-tea-please, JA-R132-this-and-that, JA-R133-last-four,
  JA-R134-phone-and-train, JA-R135-hospital-and-bank,
  JA-R136-family-and-water, JA-R137-pencil-and-ticket.
- A spoken half stays prose ("Say *sumimasen, yoku wakarimasen*. [YOU WRITE:
  **め** and **ど**, once each]"). A sentence that followed the old
  instruction and is not about the writing ("Retrieve **くち** — mouth.",
  "Then say *a station* — **R2**, five lessons back.") is now its own
  paragraph. Where a warm-up interleaved writing and speaking ("Say **はい**,
  write **語**, recall its Chinese-derived bridge, and write **もうすこし**."),
  each writing step is its own cue in the original order.
- The chained "Hear, point, say, read, then write **X**." of chapter 12 keeps
  its four spoken steps on **X** and ends with the cue; "hide and write"
  becomes "with the word hidden" inside the cue, as in chapter 11's
  "3. Hide it and write both signs." steps.
- "Copy **X**, hide it, and write it from the meaning." (chapters 13 and 14)
  becomes `[YOU WRITE: one copy of **X**; then hide it and write it from the
  meaning]`, so the narrated cue still says the learner copies a model first.
- The four-skill blocks of JA-C01-practice, JA-C08-sayounara,
  JA-C09-mou-ichido-onegaishimasu and JA-C11-body-map labelled their writing
  step `**Write:**`. That label was itself the bare imperative, so the step is
  now just the cue, which the book prints as "*Write it:* …" and which keeps
  the Listen / Speak / Read / Write order. The dictation instructions ("hear
  the word, wait ten seconds") move inside the cue after a dash.
- The numbered steps of JA-R131 to JA-R137 stay one item each, as
  ZH-R21-close's did in #17014. A spoken half that is about the written words
  goes inside the cue ("2. [YOU WRITE: **きのう** and **けさ** from memory,
  and say which one is earlier]"), and so does circling a sign the learner
  has just written ("circle the two signs said *o*"). A spoken half that
  stands alone comes first ("3. Say *heya*. [YOU WRITE: **へや**]"), so a
  driver can still do it.
- Two circling tasks were really questions a listener can answer aloud, so
  they became spoken prompts instead of cues: JA-C09-ichido's "Circle the
  dakuten." now asks which mora carries the dakuten (the last, *do*), and
  JA-C10-mou-sukoshi's "Circle **う**, the second beat of *mō*." now asks the
  learner to name that sign.
- Steps the detector does not match but that still ask for a pen are cues
  too: JA-C10-slower-please's "3. **Read/write:** …" is now a **Read:** step
  and a cue; JA-R12-foundation-02's "Build the five and mouth components of
  **語**" and JA-R13-family-a's "Build distant **語** once" are inside cues;
  JA-R12-writing-01's "Add dakuten to a known base" and JA-R13-family-d's
  "Add distant **ー** after **コ**" open their cues.
- Warm-ups that wrote a sign and then said it ("Before the new word: write
  **お**, then **ち**, and say each one.", JA-C131-ocha; also JA-C132-soko and
  JA-C133-eki) now say the sounds first and end with the cue, so the
  speaking can happen while driving.
- JA-C17-nanatsu's Grammar Lens said "Write the two rows out and the pattern
  is exact:" before a table; it now reads "Set the two rows side by side".
- JA-C01-practice's review pulse had lost its kana when it was authored
  ("From memory, write ??, ???, ?????, ?????, ???, and ????."). The cue now
  names the six words that fit both the six lengths and the block's
  `assesses` list (hiragana, kanji bridge, katakana loan, dakuten):
  **はい**, **いいえ**, **こんにちは**, **ありがとう**, **日本語** and
  **コーヒー**. The same `?` damage in JA-C01-iie, JA-W01-ko, -n, -wa,
  JA-W03-sa, -ka, -dakuten, JA-W05-mouth-component and -nichi-kanji is not
  drivable debt and is left for its own change.
- The lessons leave `tests/drivable-writing-debt/` in human-language-data;
  this track has no debt left, so its ledger file is deleted.
- Regenerated: book chapters 7-18 and 131-137, narration (`.json` and
  `.txt`), their generated book and narration hashes, and each lesson's
  `core/lesson-modality` owner (source hash only; all still `drivable: true`).

