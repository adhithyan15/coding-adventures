### Fixed — drivable lesson prose stops asking a driver to read or handle cards

- Issue #12070, tenth pass. Narration reads bare prose aloud as written, and
  the earlier passes gated writing in prose but read only cues for reading,
  pointing and gestures. So a prose instruction to read printed script, handle
  cards or cover the page still reached a driver unhedged: "Hear, picture the
  part, say, and read **おなか**." (seven Japanese body-word lessons), "Then
  take six meaning cards, say each word, and read the six character cards."
  (Chinese R12/R13), "Look, cover, and wait five seconds." (31 Marwadi
  lessons), "Read the advert: **شقة للإيجار** — …" (the notice chapters of
  seventeen tracks), "Read down once, without stopping." and its elliptical
  "Again — and notice …" (every first-reading chapter), and "Now your turn,
  out loud and then in writing." (21 Spanish synthesis lessons).
- Inventory: every drivable lesson whose narrated prose (the writing check's
  `narratedProseSpans`, so never a cue) says read, look, see, watch, check,
  find, card, cover, hide, close, open, page, printed, "in writing", "sound
  out", "type", or a hand gesture was read by hand and sorted into
  instruction and description. 759 lessons in all 23 tracks were edited.
- Fixes, in the authored order: 634 `[YOU READ: …]`, 78 `[YOU COVER: …]`,
  12 `[YOU CHECK: …]`, 5 `[YOU FIND: …]` and 27 `[YOU WRITE: …]` cues, all
  manual verbs the narration defers ("once you have stopped driving — …") and
  the book prints as "*Read it:*", "*Cover:*", "*Check:*", "*Find:*",
  "*Write it:*". About 190 steps were said for the ear and voice instead,
  where the step was about a sentence rather than the script: "Say that again
  and listen for the verb", "produce all six from their English meanings",
  "go on to find out" for "read on", "Leave the lessons before this one
  closed" for "Close the lessons before this one" (34 Malayalam, 29 Spanish
  and 2 Marathi reviews; Marathi's "Close the chapter" and Telugu's "Close the
  book" the same way). A spoken step about the word just read stays inside
  the READ cue (", and say" becomes ", then say"), as in the recall split.
  Prose that followed a new cue in the same paragraph has its own paragraph,
  and in lessons wrapped at 80 columns the touched paragraphs are rewrapped,
  each cue on one line.
- Two asks the gesture pass missed are fixed here: JA-R12-foundation-01's
  "Tap one mora for **い**" and GE-C27-schliessen's "Close your hand, both
  ways". FA-C13-khorshid's "Look up" and ES-C54-sintesis-lo-que-hay's "Look
  around the room you are in" (eyes off the road) become "Picture the sky"
  and "Picture the room you are in".
- Left alone, and recorded in each track's changelog: interpretation ("Read
  it literally", "read it as", "Read it in Kannada order"), description ("you
  can now read", "read off the page centuries later", participles), glosses
  and headings ("— read it, then do it yourself"), advice for the street
  ("Read a notice like this and act on it", "Read the door before you lean on
  it"), narrative conditionals ("Look up **पहिला** in Molesworth's dictionary
  and it is there"), the idiom "Look at what happened", states rather than
  steps ("Keep the page covered", "Keep your pencil down"), answers spoken
  after a pause ("Check: **ਸੱਚਾ**"), and lessons that are not drivable.
- Detector (`tests/drivable-writing-imperatives.ts`): new
  `proseAsksToReadOrHandleCards` / `readingOrCardProse`, over the narrated
  prose spans. Four shapes, each a closed vocabulary: "read" where a step
  starts, with its object on the page (bold script or any non-Latin word,
  "printed", "script", a card, "down"/"across" a list, after at most two
  particles and a determiner plus four plain words — `proseReadsThePage`); a
  card-handling verb (turn, shuffle, take, place, put, pick up, reverse, sort,
  deal, lay out, hold up, match, read) with "card"/"cards" before the clause
  ends; a card named by what is printed on it (meaning, character, printed or
  unpointed card); and cover/uncover/hide with a pronoun, script, a page noun
  within three words, or as a bare chained step ("Look, cover, and wait").
  Step links are sentence starts and ", ", ", and ", "; ", " and ", " then "
  — not the em dash, which in prose opens a gloss ("*Cubre la olla* — cover
  the pot"). Quotations are blanked first. Linear: fixed-literal links and
  verbs ending in a lookahead, then constant work per match over at most 80
  characters.
- Measured over the corpus before the fix, the check fired on 418 prose spans
  in 391 drivable lessons, every one of which this change rewrote, and after
  it on none; in lessons that are not drivable it fires on 739 spans in 603
  lessons, all real reading, card and look-cover-write work in a sample.
  Interpretation ("Read it literally", "read it as **is held**"), participles
  ("written in **नमस्ते** and read in **स्टेशन**"), glosses, card vocabulary
  ("the Bengali for an identity card") and "cover" as a meaning do not fire.
  Out of scope, and fixed by the inventory: a passage as the object ("Read it
  once without stopping"), a look at a table ("Look down the two columns"),
  and narrative conditionals.
- Tests (`tests/drivable-writing-cues.test.ts`): 25 positives (corpus prose
  as authored before the fix, one per shape, and a span wrapped across two
  source lines), 27 controls (the rewrites, interpretation, description,
  glosses, advice, card vocabulary, cover as a meaning, a quotation), 17
  ~50,000-character adversarial inputs asserting answers only, the corpus
  check (zero in drivable lessons) and an anti-vacuity floor on non-drivable
  lessons (> 400; 603 measured).
- Modality: no lesson changes drivability. A deferred cue is not a sight
  signal, and no rewrite adds "look at", "see the" or a table, so
  core/lesson-modality changes only the source hash of the 759 edited
  lessons. Still drivable, and worth a modality decision rather than a prose
  fix: lessons of `type: reading` (the first-reading chapters) whose whole
  task is printed script; their reading steps are now deferred cues.
- Regenerated the books, narration and their hashes for the touched chapters,
  and the 759 lesson-modality owners. Changelogs: a shard in each of the
  twelve sharded tracks and an entry in each of the eleven CHANGELOG.md
  tracks.
