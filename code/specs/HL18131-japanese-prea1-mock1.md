# HL18131 — Japanese pre-A1 full mock 1

## Status and boundary

Issue #18131 is a structural first form against the checked-in, project-defined
Japanese pre-A1 task shape. This is a precursor below JLPT and JF Standard A1,
not an official examination, readiness claim, or human-validation result.
The whole administration is 20 written minutes (5 reading, 5 listening,
10 writing) plus 5 individual speaking minutes and 1 minute of speaking
preparation. Lessons that prepare for it stay at or below 300 seconds; the
continuous mock is assessment, not a lesson.

## Taught-content closure before form writing

Keep scored language within direct lesson owners, rather than the many later
Japanese headwords in the book:

- `JA-C01-practice` teaches and separately retrieves the daytime doorway
  exchange **こんにちは**, **日本語?**, **はい**, **ありがとうございます**, **いいえ**;
  it also calls for delayed handwriting of these already introduced forms.
- `JA-C09-onegaishimasu` teaches the courteous request **おねがいします**;
  `JA-C09-mou-ichido-onegaishimasu` teaches the bounded one-more-time
  request. Keep its memorised use separate from free verb composition.
- `JA-C11-mimi`, `JA-C11-hana`, and `JA-C11-te-hand` directly teach known
  body words and independent handwriting. A written dictation or delayed card
  may use only words whose complete kana have been taught and retrieved.
- `JA-C41-namae` teaches the word **なまえ**. It does not teach a full “my
  name is” construction. An identity prompt can ask for one chosen personal
  name as a single spoken response, without scoring an untaught sentence.
- Reading recognition can also use **コーヒー** from `JA-C01-koohii` and **あし**,
  **口**, and **め** from their direct `JA-C11` lessons. The reading packet must
  not print the exact later writing or speaking response targets.

Any candidate prompt, recording line, distractor, or model beyond those
bounded forms needs an exact direct lesson owner and script check before it
is admitted. A Japanese specialist reviews naturalness, register, mora timing,
and recording quality; a file-system check cannot substitute for that review.

## Form and security contract

- Reading: five 1–3-word known Japanese-script selections in five minutes.
- Listening: five 2–8-second recordings, each played twice, testing a
  greeting, yes/no response, identity detail, or classroom cue in five minutes.
- Writing: one delayed kana recall after a ten-second card view (5 points),
  two 2–5-second dictated items (10 points), and two independently produced
  words or memorised chunks from non-copyable cues (10 points) in ten minutes.
- Speaking: five 2–8-second live prompts, each answered in 2–15 seconds,
  covering the taught doorway exchange, one chosen name, yes/no, and a
  memorised request. One slow repeat is the limit.

Keep the four learner skill packets physically separate and collect each
before releasing the next. Put recording bodies, dictation targets, card
models, productive scoring targets, and the key only in protected examiner
material. The learner paper contains no transcript, romaji, furigana over
assessed kana, or copyable answer model. The manifest pins every part ID,
skill time, and file identity. Reading, listening, writing, and speaking each
need 60% independently; an overall total cannot compensate for a failed skill.

## Verification

Follow spec, red focused test, implementation, changelog. Direct Japanese
corpus tests must pin exact item counts, timing, source-question-key meaning,
taught-token closure, secure non-leakage, independent scoring, and the absence
of readiness language. Run affected package, shard, artifact, book, and app
checks on the final head, then obtain independent confidentiality/integrity
review before publishing a PR. Form 2 remains separate dependent work.
