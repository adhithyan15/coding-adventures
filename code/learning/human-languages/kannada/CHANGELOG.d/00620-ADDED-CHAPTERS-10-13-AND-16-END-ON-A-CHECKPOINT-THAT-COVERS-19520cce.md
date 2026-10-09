## Added — chapters 10, 13 and 16 end on a checkpoint that covers their script lessons

Three chapter payoffs were under the 0.5 `chapter-payoff-not-representative`
floor because the chapter's script lessons come after its spoken lesson, so
the spoken lesson could not assess them. Following RU-C01-checkpoint and
AR-C02-checkpoint, each chapter now ends on a closing checkpoint
(`practice-mix`, introduces nothing) after every lesson of the chapter on both
the sequence and the curriculum path, and the payoff moves to it (kind
`task`).

| chapter | new payoff | sequence | path | before | after |
|---|---|---|---|---|---|
| 10 | KA-C10-checkpoint | 389 | KA-PATH-013 order 1 | 1/3 | 3/3 |
| 13 | KA-C13-checkpoint | 419 | KA-PATH-016 order 1 | 1/3 | 3/3 |
| 16 | KA-C16-checkpoint | 449 | KA-PATH-019 order 1 | 2/5 | 5/5 |

- **KA-C10-checkpoint**: the seven days and the *bhānuvāra* / *ravivār*
  contrast from memory; ◌ೋ found in ಸೋಮವಾರ and ೨ read as *eraḍu*; both
  written from sound. Computed 174 s, declared 200.
- **KA-C13-checkpoint**: *tale* and *kai* and their Tamil and Malayalam
  cousins; ಚ found in ಚೆನ್ನಾಗಿ and ೫ read as *aidu*; both written from sound.
  Computed 159 s, declared 180.
- **KA-C16-checkpoint**: the twelve months, the calendar shared with Hindi,
  Ugadi on *Chaitra Śukla Pratipada*, *uttara*; ಉ standing in ಉತ್ತರ against ು
  riding in ಹೌದು, ೮ read as *eṇṭu*; ಉ and ೮ written from sound. Computed
  242 s, declared 260.
- Each has a spoken "Guided Practice" (voice core), a detachable "Script —"
  block with `[YOU READ: …]` cues and a detachable "Writing — from sound" block
  (`dictation-transcription`).
- **Path:** three new `required` consolidation extensions
  (KA-EXT-013-, KA-EXT-016-, KA-EXT-019-CONSOLIDATION) inlined on their
  segments; KA-C17-madhyaahna-madhyaraatri and KA-C18-gante move down one
  place on KA-PATH-019.
- **chapters.d**: chapters 10, 13 and 16 name the checkpoint and its atoms,
  and the note says why the spoken lesson could not be the payoff.
- Regenerated book chapters, narration, modality owners (`pen`, `voice` core)
  and hash shards; the Kannada curriculum digest pin moves (three more
  lessons). Kannada now has no chapter-gate debt.
