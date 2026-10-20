---
category: Repo policy / workflow reminders
---

# A non-Latin-script vocabulary track needs English chapter titles and a letter lesson for every letter its headwords contain

**What went wrong.** The Russian B1 vocabulary (chapters 271-534) generated
with 0 validate errors and 0 forward references, but three gates failed.

1. **An untaught letter.** Three headwords contained ъ (*объявление*,
   *объявлять*, *предъявлять*). The track had never given ъ a letter lesson, so
   `letter-anchoring.test.ts` (unwritten letters, pinned at 0) and
   `script-closure.test.ts` (closure debt, pinned at 0) failed.
2. **Cyrillic chapter titles.** Two drafters titled their chapters in Russian.
   `slugify` keeps only ASCII, so all 132 chapters got the label `ch:ru-` and
   the file name `chNNN-.tex`. The strict book compile then reported 247
   hyperref warnings, from Cyrillic in PDF bookmarks.

**Fix.** Chapter 266 gained a word lesson for *объявление* and the letter
lesson RU-W266-tvyordyy-znak, anchored in it; tranche a's *объявление* was
replaced. The 132 titles were translated into English, as the track's other
chapters are, and the stale `chNNN-.tex` files were removed. The B1 builders
now assert that every chapter title slugifies to a non-empty string.

**Next time.** For a track in another script:
- list every letter in every candidate headword and compare it with the
  track's letter lessons before drafting;
- tell drafters that chapter titles and review headings are English;
- run the strict compile on the first generation.
