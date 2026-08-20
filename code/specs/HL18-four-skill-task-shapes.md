# HL18: Four-skill task shapes

Status: active foundation. This specification defines the performance target;
it does not claim that any current book reaches it.

## 1. The missing middle

An exam-point inventory says **what language** an assessment may require. HL16's
assessment contract says **what evidence** must exist before a book can claim a
level. Neither says what the learner actually has to do under assessment
conditions. "Teach invitations" does not tell an author that German A1 asks for
an approximately 30-word personal message covering three prompts in 20 minutes.

A task-shape inventory records that missing middle for reading, listening,
writing and speaking. It is the target from which the five-minute teaching ramp
is decomposed; it is not itself a lesson and does not relax the five-minute cap.

## 2. Location and coverage

Each track may contain `<track>/task-shapes/<level>.json` for A1 through C2.
There is no external pre-A1 certificate in this model. The project's pre-A1
floor exists to make the first approach to A1 gentle.

Every inventory must name exactly one external exam or clearly labelled
project-defined equivalent, cite official or project-owned sources, and contain
exactly one section for each of reading, listening, writing and speaking. A
file's absence is backlog. A malformed file is an error, never zero debt.

## 3. Required task dimensions

Each task part records:

- prompt mode and genre;
- response mode;
- item count;
- stimulus and response length, with units and approximate/exact status;
- interaction mode and replay count;
- scoring ceiling and named criteria;
- allowed, forbidden and unpublished aids;
- source identifiers; and
- every important measurement the source does not publish.

Unknown is first-class. A source that does not publish audio speed, text length,
or response length is represented by `null` plus a `notPublished` explanation.
It must never be filled with a plausible-looking estimate just to satisfy a
schema.

Published time ranges are first-class too. An administration or section may use
one exact positive minute count or an exact `{ minimum, maximum }` range. A
published 5–7 minute speaking paper stays 5–7; it must not become an invented 5,
6, or 7. Section ranges must add up to the administration range exactly.

Alternate official forms are represented as named variant sets over the
section's union of parts. Each variant cites its source and lists exactly the
parts administered together. Every union part must occur in at least one form.
This preserves a four-exercise and a five-exercise paper as two alternatives;
it never flattens them into a fictional nine-exercise paper. With no explicit
variants, all parts form one backward-compatible implicit administration.

## 4. Awarding-body pass rules versus project evidence

The inventory transcribes the awarding body's real rule. It does not improve
that rule. For example, Goethe German A1 awards the certificate at 60/100 after
all sections are attempted, but publishes no independent pass threshold for all
four skills. Those four thresholds are therefore `null` in the task-shape file.

HL16 is intentionally stricter: the later project assessment contract must add
independent internal thresholds so a strong reading score cannot hide an
untaught speaking or writing strand. The source-backed external rule and the
project's stronger readiness claim remain distinguishable.

## 5. From exam task to five-minute lessons

A task shape is decomposed backward into microsteps. A 30-word message is not
introduced as one "writing lesson". Its ramp begins with observing and tracing,
then copying a known chunk, changing one slot, filling one field, joining two
known chunks, satisfying one prompt, and only later combining three prompts
under time. Listening similarly separates genre recognition, one-pass versus
two-pass audio, short clips, distractors and timed sets. Speaking separates a
known response, one substitution, a prompted exchange, an unprompted exchange,
and timed interaction.

Every instructional unit remains at most five minutes. No lesson may introduce
several new grammar, vocabulary, script and task-format demands merely because
the final exam combines them.

## 6. Computed backlog

`buildTaskShapeBacklog()` enumerates every missing `(track, certifiable level)`
pair in level-first, round-robin order. This keeps the finite research work
visible across all languages instead of letting one favored track consume the
queue. Adding a valid inventory removes exactly one stable
`task-shape/<language>/<level>` item.

The initial proof is German A1 against current official Goethe administration,
model-set and exam-information sources. It contains eleven task parts. It is a
research target for later curriculum and mock work, not a claim that the German
book currently prepares a learner to pass.
