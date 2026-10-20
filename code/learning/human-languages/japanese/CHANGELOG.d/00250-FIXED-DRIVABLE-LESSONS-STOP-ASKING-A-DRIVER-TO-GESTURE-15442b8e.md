## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
clapping beats, tapping, pointing at body parts and places, touching, and
holding up a pen (issue #12070, ninth pass). Each ask is now said for the ear
and voice where that keeps the learning goal, or moved into a cue the
narration defers (`[YOU POINT: …]`, `[YOU READ: …]`: "once you have stopped
driving — …"). The new gesture check in human-language-data demands zero such
spoken cues in drivable lessons. Every edited lesson stays `drivable: true`
(only its `core/lesson-modality` source hash changes).

- **Count:** 30 spoken cues and 26 prose instructions in 55 drivable lessons.
- Mora drills: `[YOU SAY: *denwa*, clapping three beats]` → `[YOU SAY:
  *denwa*, then count its beats aloud — three]` in JA-C131-chotto,
  JA-C131-isha, JA-C131-ocha, JA-C131-toshokan, JA-C132-kuruma, JA-C133-eki,
  JA-C133-ike, JA-C133-inu, JA-C134-denwa, JA-C134-kaban, JA-C134-shinbun,
  JA-C134-taberu, JA-C135-byouin, JA-C135-doyoubi, JA-C135-getsuyoubi,
  JA-C135-ginkou, JA-C135-iriguchi, JA-C136-kaze, JA-C136-kazoku,
  JA-C136-mizu, JA-C137-enpitsu, JA-C137-ippai, JA-C137-kippu,
  JA-C137-perapera. JA-C131-chotto keeps its point as "three, and the middle
  one is a silent hold"; JA-C09-mou's "and tap twice" counts its two beats the
  same way. Judgement call: counting aloud after the word keeps the lesson's
  question — how many beats? — which the wrap-up recall asks in the same
  words.
- Demonstratives: JA-C132-kore, JA-C132-soko, JA-C132-sore, JA-C132-soto and
  JA-C133-heya picture what the word lands on ("*kore*, picturing a pen in
  your hand, then *koko*, picturing the spot where you stand") instead of
  holding up, touching or pointing; JA-C10-koko, JA-C11-te-hand, JA-R12-mixed-
  scripts and JA-R132-this-and-that do the same in prose.
- Body words: JA-C11-ashi, JA-C11-hana, JA-C11-kao, JA-C11-kuchi, JA-C11-me-
  eye, JA-C11-mimi and JA-C11-te-hand "Hear *hana*; point to the nose" → "Hear
  *hana*; say what it names — the nose"; JA-C11-body-map "point to the named
  body part" → "say the English for the named body part"; JA-C12-atama,
  JA-C12-ha-tooth, JA-C12-kami, JA-C12-kata, JA-C12-koshi, JA-C12-onaka and
  JA-C12-senaka "Hear, point, say, and read" → "Hear, picture the part, say,
  and read"; JA-C12-body-map-two "Listen and point" → "Listen and give each
  English meaning"; JA-C13-chichi and JA-R13-family-a "point to “my father”" →
  "say “my father”".
- Script: JA-C09-yoku "Point to **み** and **め** inside it" becomes `[YOU
  POINT: **み** and **め** inside it]`, deferred; JA-C14-ichi "point to the part
  of it that is the number" → "name the part …", which the ear can do.
- Beats in prose: JA-C08-hear-sayounara "Tap five even beats" → "Count five
  even beats aloud"; JA-R131-tea-please "clap its beats" → "count its beats".
- Left alone: the block and lesson titles "Guided Practice — point and write"
  and "Retrieval 6 — greet, repair, point" (labels), JA-C08-hear-sayounara's
  typed-activity prompt "how many morae do you tap?" (shown in the app, not
  narrated), and the non-drivable JA-W131-small-yo and JA-W132-ru recalls,
  which still clap. The reading steps in these lessons' prose ("read all
  seven") are a separate question from gestures and are unchanged.

