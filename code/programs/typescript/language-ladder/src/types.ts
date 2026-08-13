// The shape of the SCRIPT data this app consumes.
//
// These two interfaces used to be declared here, with a note explaining that
// they were a deliberate local duplicate: the app read `data/scripts/*.json`
// directly, so redeclaring a handful of field names was cheaper than routing
// the JSON through a package.
//
// They are no longer a duplicate. `@coding-adventures/script-ductus` now owns
// both the script files and the hand-authored pen paths that cite the letters
// in them — a letter's stroke order is verified against the very font its
// `ScriptData` names — so the type and the data it describes live together, and
// this file re-exports rather than restates. One definition, and every existing
// `from "./types.ts"` import in the app keeps working unchanged.
//
// (The app also depends on `@coding-adventures/human-language-data` — see
// `lessons.ts`, which uses its pure `parseLesson` to read the lesson files. It
// deep-imports `.../src/parse.ts` rather than the barrel, which would drag
// `node:fs` and `process` into the browser bundle.)

export type { Letter, ScriptData } from "@coding-adventures/script-ductus";
