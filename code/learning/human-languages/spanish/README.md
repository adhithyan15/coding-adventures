# Spanish

The pilot track for the [Human Languages](../README.md) curriculum. Goal:
absolute-beginner to B1 ("can hold a normal day-to-day conversation") over a
year, in ~5-minute units consumed during a daily car commute. Framework
details (unit anatomy, spaced-repetition schedule, etymology methodology) are
in [`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) — this
README is just "how to actually use this in the car."

## How to use this in the car

1. Before you drive, open [`session-map.md`](./session-map.md) and find the
   next session you haven't done yet.
2. Read (or have read to you) the units listed for that session, in order —
   the review units first, then the one new unit, then the practice-mix.
   Each unit is self-paced: pause, speak your answer out loud, then continue.
3. That's the core block, ~15-25 minutes. If your drive is longer, keep going
   into that session's bonus queue — extra review units, never anything you
   haven't earned yet.
4. Next drive, start from the top of the next session. Don't skip ahead —
   the review schedule assumes you did the units in order.

Units are plain Markdown files today, written in an audio-script style
(`[PAUSE]`, `[REPEAT x2]`, `[YOU SAY: ...]`) so they can be read aloud by you,
a passenger, or (eventually) a voice pipeline — that pipeline doesn't exist
yet; see `HL00`'s "Explicitly Out of Scope" section.

## Progress

- **Phase 0 — Foundations**: Week 1 fully authored (`units/`, sessions 1-5).
  Weeks 2-4 not yet written.
- **Phases 1-4**: skeleton only, in [`roadmap.md`](./roadmap.md).

See [`CHANGELOG.md`](./CHANGELOG.md) for what's been added week by week.

## Files

- [`roadmap.md`](./roadmap.md) — the full year, phase by phase, topic by topic.
- [`session-map.md`](./session-map.md) — which units make up which session,
  and the worked spaced-repetition schedule for Week 1.
- [`units/`](./units/) — the actual lesson files.
