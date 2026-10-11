## Added — chapters 10, 13, 16, 19 and 27 end on a checkpoint that covers their script lessons

Five chapter payoffs were still under the 0.5 `chapter-payoff-not-representative`
floor. In each, the payoff was the chapter's spoken lesson and every missing
atom came from a script lesson sequenced after it, so no edit to the payoff
could assess them honestly. Following RU-C01-checkpoint and AR-C02-checkpoint,
each chapter now ends on a short closing checkpoint (`practice-mix`,
introduces nothing) that comes after every lesson of the chapter on both the
sequence and the curriculum path, and the payoff moves to it (kind `task`).

| chapter | new payoff | sequence | path | before | after |
|---|---|---|---|---|---|
| 10 | TA-C10-checkpoint | 535 | TA-PATH-016 order 1 | 2/6 | 6/6 |
| 13 | TA-C13-checkpoint | 575 | TA-PATH-019 order 1 | 1/6 | 6/6 |
| 16 | TA-C16-checkpoint | 615 | TA-PATH-022 order 1 | 2/6 | 6/6 |
| 19 | TA-C19-checkpoint | 695 | TA-PATH-023 order 2 | 3/7 | 7/7 |
| 27 | TA-C27-checkpoint | 817 | TA-PATH-027 order 8 | 2/5 | 5/5 |

Each checkpoint has the same shape: a spoken "Guided Practice" that recalls
the chapter's own words from memory (with the answer in romanization, so the
spoken core stays voice), a detachable "Script —" block that asks what each
letter or sign is and where it sits, with one `[YOU READ: …]` cue, and a
detachable "Writing — from sound" block (`dictation-transcription`: cover,
write, check against a key, repair one shape). Every answer comes from the
introducing lessons.

- **TA-C10-checkpoint**: the week with *kizhamai*, the four native and three
  Sanskrit planet-words; ந against ன, ற below the baseline, ◌ா in நான்;
  writes ந, ன, ற and நா. Computed 224 s, declared 260.
- **TA-C13-checkpoint**: *talai* and *kai* as native words, why *naṉṟi* is
  heard *nandri*; the parts of க, where ◌ி attaches, the pieces of நன்றி, ர
  in சரி; writes க, ர, றி and நன்றி. Computed 222 s, declared 270.
- **TA-C16-checkpoint**: the twelve months, Chithirai 1 and Thai 1; ம's
  upright, why ஆம் opens on a full vowel letter, the pieces of ஆம்; writes ம,
  ஆ and ஆம். Computed 241 s, declared 270.
- **TA-C19-checkpoint**: the casual and formal age questions and the formal
  answer, *vayathu* from *vayas*; what ◌ி and ச do, the pieces of சரி, ◌ே in
  பேசு; writes றி, சரி and ◌ே. Computed 257 s, declared 280.
- **TA-C27-checkpoint**: evening *mālai*, the garland homonym and the hedged
  darkness link; the pieces of நீங்கள், why its க sounds *g*, ள among the
  three *l*-letters, ◌ீ against ◌ி; writes நீ, நீங்கள் and மாலை. Computed
  247 s, declared 270.
- **Path:** five new `required` consolidation extensions
  (TA-EXT-016-, -019-, -022-, -023- and TA-EXT-027-CONSOLIDATION) hold the
  checkpoints and are inlined on their segments. The four later TA-PATH-022
  lessons (chapters 17-18) and the six later TA-PATH-027 lessons (chapters
  28-31) each move down one place, since path orders stay dense.
- **chapters.d**: chapters 10, 13, 16, 19 and 27 name the checkpoint, list the
  chapter's atoms, and say in the note why the spoken lesson could not be the
  payoff. The spoken lessons are unchanged.
- Regenerated book chapters, narration, modality owners (each checkpoint is
  `pen` with a `voice` core) and hash shards; the Tamil curriculum digest pin
  moves (five more lessons). Tamil now has no chapter-gate debt.
