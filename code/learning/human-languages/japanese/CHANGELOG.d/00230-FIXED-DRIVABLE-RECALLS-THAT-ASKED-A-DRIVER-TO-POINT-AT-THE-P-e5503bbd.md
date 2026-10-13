## Fixed — drivable recalls that asked a driver to point at the page are said by ear

The previous entry left a question open: 24 spaced recalls in drivable
lessons of chapters 131-137 asked the learner to put a finger on a printed
sign ("point to the sign in **おちゃ** you can already write, and the one you
cannot"). RECALL is a spoken cue action, so the narration read each one to a
driver as an ordinary turn. Each now asks for the same retrieval in a form
the ear can do, keeping its target word and what it asks about the sign:

- "point to the sign in **X** you can already write, and the one you cannot"
  → "name the sign in **X** you can already write, and the one you cannot"
  (JA-C131-ocha, JA-C132-kore, -soko, JA-C133-eki, -heya, -ike).
- "point to the sign in **X** you cannot write yet, and say which sign it
  looks like" → "name the sign in **X** …" (JA-C132-kuruma, JA-C133-inu).
- "point to the two small signs in **ちょっと** …" → "name the two small
  signs …" (JA-C131-chotto).
- "point to the sign in **X** that carries the two-stroke mark, and name the
  sign under it" → "say which sign in **X** carries the two-stroke mark, and
  name the sign under it" (JA-C134-denwa, -kaban, -shinbun, -taberu;
  JA-C135-byouin, -getsuyoubi, -ginkou, -iriguchi; JA-C136-kaze, -kazoku,
  -mizu), and the same for "the small circle" (JA-C137-enpitsu, -ippai,
  -kippu, -perapera).
- None of these cues carried a spacing tag, so there was none to keep.
- JA-C09-sumimasen: "[YOU HEAR: *sumimasen* → point to **repair**, not
  **farewell**]" becomes "→ choose **repair** …", the verb its chapter
  siblings already use.
- A new check in human-language-data fails on any drivable recall that says
  "point to" or "point at", so the shape cannot come back.
- Separately, the narration now defers `[YOU READ: …]`, `[YOU LOOK: …]`,
  `[YOU FIND: …]`, `[YOU COPY: …]`, `[YOU TAP: …]` and the other hands-on
  cue verbs, which regenerates this track's narration for every chapter that
  uses them; the lessons themselves are unchanged by that.
- Regenerated: book chapters 9 and 131-137 and their hashes, the narration
  (`.json` and `.txt`) and narration hashes, and the 25 edited lessons'
  `core/lesson-modality` owners (source hash only; all still
  `drivable: true`).

