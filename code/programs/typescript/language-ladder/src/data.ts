// data.ts — the ONLY place that reaches out to the curriculum's canonical
// script files. We import them directly (no copy) so the app always reflects
// the same data the lessons teach from. Adding a script here is the single edit
// needed to surface it in the app.
//
// Paths climb out of this package (src → package → typescript → programs → code)
// into code/learning/human-languages/data/scripts/. Vite bundles the JSON at
// build time; `server.fs.allow` (see vite.config.ts) lets the dev server read it.

import type { ScriptData } from "./types.ts";
import cyrillic from "../../../../learning/human-languages/data/scripts/cyrillic.json";
import hebrew from "../../../../learning/human-languages/data/scripts/hebrew.json";
import chinese from "../../../../learning/human-languages/data/scripts/chinese.json";
import arabic from "../../../../learning/human-languages/data/scripts/arabic.json";
import persoArabic from "../../../../learning/human-languages/data/scripts/perso-arabic.json";
import urduNastaliq from "../../../../learning/human-languages/data/scripts/urdu-nastaliq.json";
import devanagari from "../../../../learning/human-languages/data/scripts/devanagari.json";
import gujarati from "../../../../learning/human-languages/data/scripts/gujarati.json";
import tamil from "../../../../learning/human-languages/data/scripts/tamil.json";
// The Dravidian syllabaries (Telugu/Kannada/Malayalam) are GENERATED from Unicode
// by data/scripts/generate_syllabary.py — each "letter" is a base consonant
// composed with a vowel sign (ka, ki, ku, kha, …), for reading-recognition.
import kannada from "../../../../learning/human-languages/data/scripts/kannada.json";
import telugu from "../../../../learning/human-languages/data/scripts/telugu.json";
import malayalam from "../../../../learning/human-languages/data/scripts/malayalam.json";

// The JSON files are authored to the ScriptData shape; assert it once here.
export const SCRIPTS: ScriptData[] = [
  cyrillic as ScriptData,
  hebrew as ScriptData,
  chinese as ScriptData,
  arabic as ScriptData,
  persoArabic as ScriptData,
  urduNastaliq as ScriptData,
  devanagari as ScriptData,
  gujarati as ScriptData,
  tamil as ScriptData,
  // The Dravidian trio, in chain order (Tamil → Kannada → Telugu → Malayalam).
  kannada as ScriptData,
  telugu as ScriptData,
  malayalam as ScriptData,
];
