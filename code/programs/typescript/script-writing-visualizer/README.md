# Script Writing Visualizer

**The HL02 companion app, MVP.** The Human Languages curriculum teaches a
non-Latin script *inline* — a letter is introduced inside the first word that
needs it. This app is the other half of that promise: it **breaks each letter
apart** into its pieces and shows a **stroke order**, so you can practise
*writing it on paper*.

Pick a script, pick a letter, and the detail panel shows:

- the **glyph**, big, with its sound and role;
- **Break it apart** — the letter's component pieces (the "a vertical + two
  stacked bowls" of Cyrillic *в*);
- **Write it** — a conventional stroke order, numbered;
- a **⚠ false friend** badge for letters that look like a Latin letter but
  aren't (Cyrillic *в*=v, *р*=r, *с*=s, *н*=n) — the fastest way into the script.

## Where it fits

```
code/learning/human-languages/data/scripts/*.json   ← the source of truth (HL01)
        │  (glyph, components, strokeOrder, notes per letter)
        ▼
script-writing-visualizer                           ← this app renders it (HL02 MVP)
```

The app imports those JSON files **directly**, so it can never drift from the
curriculum. Adding a script to the curriculum surfaces it here with a one-line
edit in `src/data.ts`. Ships today with **Cyrillic, Hebrew, Chinese, Arabic, and
Devanagari**.

## Design

- **`src/core.ts`** — the pure, unit-tested heart: `buildScriptView`,
  `scriptSummary`, `isFalseFriend`, `falseFriends`. No DOM, no globals; this is
  where the pedagogy is tested.
- **`src/data.ts`** — the only place that imports the canonical script JSON.
- **`src/main.ts`** — a deliberately framework-free vanilla-DOM shell.

## Develop

```sh
npm install
npm run dev        # local dev server
npm test           # unit tests (vitest)
npm run build      # production build to dist/
npm run preview    # serve the production build
```

## Scope (MVP) and what's next

v1 is **read + decompose** only — recognition and hand-writing practice, no
in-app handwriting capture and no scheduler. The next steps toward the full
`HL02` spec are the **interleaving scheduler** (spaced, cross-language review)
and **recall drills** (prompt → pick the glyph). See
`code/specs/HL02-companion-practice-app.md`.
