## Fixed — drivable lesson prose stops asking a driver to read or handle cards

Narration reads bare prose aloud as written, so a prose instruction to read
printed script, handle cards or cover the page reached a driver unhedged (issue
#12070, tenth pass): "Hear, picture the part, say, and read **おなか**." was
narrated word for word. Each such step now moves into a cue the narration
defers (`[YOU READ: …]`, `[YOU COVER: …]`, `[YOU CHECK: …]`, `[YOU FIND: …]`,
`[YOU WRITE: …]`: "once you have stopped driving — …"), in the authored order,
or is said for the ear and voice where the step was not about the page. Prose
that followed a new cue in the same paragraph now has a paragraph of its own.
The new prose check in human-language-data demands zero such spans in drivable
lessons. Every edited lesson stays `drivable: true` (only its
`core/lesson-modality` source hash changes).

- **Count:** 68 drivable lessons; 72 `[YOU READ: …]` and 2 `[YOU COVER: …]`
  cues, and 12 ear-and-voice rewrites.
- "Hear, picture the part, say, and read **おなか**." → "Hear, picture the part,
  and say **おなか**. [YOU READ: **おなか**]" in JA-C12-atama, JA-C12-ha-tooth,
  JA-C12-kami, JA-C12-kata, JA-C12-koshi, JA-C12-onaka and JA-C12-senaka; the
  same split for "Hear, say, and read …" in JA-C12-body-map-two,
  JA-C12-ha-tooth, JA-R12-body-01, JA-R12-farewell, JA-R12-foundation-01,
  JA-R13-family-b and JA-R13-family-d. Judgement call: the reading stays,
  deferred, because these lessons teach the kana as well as the word.
- Sign-by-sign readings ("2. Read **あ | し**.", "Read **は | な** as *ha-na*.",
  "Read **さ | ん** as two beats.") and single readings ("Read **いちど**.", "Read
  **よく**, then open a repair …") become READ cues across JA-C09..C15; "2. See
  **め**; say the meaning." → `[YOU READ: **め**, then say the meaning]`.
- Four-skill labels "3. **Read:** …" become "3. **Reading:** [YOU READ: …]"
  (JA-C01-practice, JA-C09-mou-ichido-onegaishimasu, JA-C10-slower-please,
  JA-C11-body-map); JA-C08-sayounara's "cover the romanization and read
  **さようなら** sign by sign" is one COVER cue; JA-C08-hear-sayounara "Close the
  text …" is a COVER cue.
- JA-C19-greetings / JA-C19-kyoushitsu reading steps, "Read on sight:"
  (JA-C95-mochimono), "Read this without stopping: **…**" (JA-C72-gogo,
  JA-C119-sakanaya), the notices JA-C142-eigyouchuu / JA-C142-junbichuu and
  JA-R142's clinic notice become READ cues. JA-R131..R137 "Read each one aloud
  and say what it means." (above existing READ cues) → "For each one below, say
  what it means."
- JA-R12-foundation-01 "Tap one mora for **い**" (a gesture the gesture pass
  missed) → "Say **い** and count its one mora aloud"; "Stop after one accurate
  card" → "… round". JA-C10-slower-please "Read *sumimasen* and
  *onegaishimasu*" (romanization) → "Say …"; JA-C11-hana "Read **は** as *ha*
  inside this word" → "Inside this word, **は** is *ha*."; JA-C18-hitori-futari
  "Read the four words together" → "Take the four words together".
- Left alone: "Read a notice like this and act on it: stop, look, and cross
  with care" (JA-C142-chuui — advice for the street), "Read once off a page, it
  becomes something you can reach for" (description), "**五** is **five**, read
  *go*" (a gloss), "Keep listening and reading scores separate".
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - JA-C17-kokonotsu keeps its premise spoken: "The first two signs of **ここのつ**
    spell **ここ**, the word for *here* you have had since chapter ten. [YOU READ:
    the first two signs on their own]". The door notices JA-C142-eigyouchuu and
    JA-C142-junbichuu say what the door says in prose ("The door says
    **じゅんびちゅう**, so the shop is not open yet.") after "[YOU READ: the door sign
    and act on it]".
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (JA-C19-greetings,
    JA-C72-gogo, JA-C95-mochimono, JA-C119-sakanaya): "[YOU READ: the six
    greetings again, …]", "[YOU READ: the request without stopping — …]", "[YOU
    READ: the three greetings on sight]".
  - JA-R131..R137: the first pass's "[PAUSE 2s each] For each one below, say
    what it means." dropped the reading the step was for → "[PAUSE 2s each] [YOU
    READ: each one below aloud, then say what it means]" (JA-R131: "…, count its
    beats, then say what it means").
  - JA-C08-hear-sayounara "[YOU COVER: the text before the new expression
    begins]" → "[YOU COVER: the text]" and, in prose, "Then the new expression
    begins."
  - Four-skill items drop the "**Reading:**" label in front of a READ cue, which
    the book printed as "Reading: *Read it:* …" (JA-C01-practice,
    JA-C09-mou-ichido-onegaishimasu, JA-C10-slower-please, JA-C11-body-map).
    JA-C10-slower-please's item "[YOU READ: …] [YOU WRITE: …]" becomes two
    numbered items.
  - A new cue followed by prose or by another cue in the same paragraph now ends
    its paragraph, so the book no longer runs "*Cover:* the page The next
    question …" together (JA-C09-sumimasen, JA-C10-wakarimashita, the JA-C12
    body words, JA-R12, JA-R13; 17 lessons).
- **Second review follow-up:** JA-C08-sayounara drops the `**Reading:**` label in front of its COVER cue, which printed as "Reading: Cover:" and left "Reading:" alone in the narration.
- **Book compile follow-up:** JA-C72-gogo's cue "[YOU READ: the request without
  stopping — **もういちど、おねがいします** (Once more, please.)]" left the book
  one overfull line (27pt, chapter 72), because the bold Japanese is one
  unbreakable box and the English before it was too long to share its line and
  too short to stand alone. The cue now leads with the Japanese, as other READ
  cues do: "[YOU READ: **もういちど、おねがいします** (Once more, please.) without
  stopping]". The narration still defers it ("once you have stopped driving —
  read: …"), and the book's overfull count is back to its baseline of 0.

