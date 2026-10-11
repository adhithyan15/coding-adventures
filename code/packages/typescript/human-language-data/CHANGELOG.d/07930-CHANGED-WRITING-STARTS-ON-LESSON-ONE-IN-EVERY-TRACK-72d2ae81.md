### Changed — writing starts on lesson one in every track

The gentle-ramp report's `writing-ramp` queue is now empty, down from 6 rows
(10 opening lessons). `summary.tracksWhereWritingStartsLate` falls from 6 to
0. Punjabi (3), Gujarati (2), Sanskrit (2), French (1), Malayalam (1) and
Spanish (1) each gained an `observe-trace` Writing block in their first
lesson. In every case it is a finger trace of one shape the lesson already
holds: the first shape of a romanized headword (ਸ, ન, न, ന) or the silent
letter of the greeting (French *salut*'s final t, Spanish *hola*'s h). The
blocks are detachable, so each lesson's core modality stays `voice` and no
chapter's drivable prefix moves. No other gentle-ramp kind, script-closure
total or chapter gate changes.

`src/gentle-ramp.ts` is unchanged. The corpus pins that recorded the old
openings now record the new ones:

- the French, Gujarati, Punjabi and Sanskrit writing-stage ladders each gain
  their leading lesson-one `observe-trace`;
- the Gujarati and Malayalam meaning-first pins change from "no script
  anywhere in lesson one" to "no script outside one detachable trace block,
  which shows exactly one headword glyph", and both lessons now declare the
  `writing` skill.
