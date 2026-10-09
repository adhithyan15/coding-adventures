## Added — chapters 8 and 14 end on a checkpoint after both their orders agree

Chapters 8 and 14 were the two Hindi payoffs left below the 0.5
`chapter-payoff-not-representative` floor, because their lesson sequence and
their curriculum path disagreed. HI-C08-kripaya (sequence 510) is first in
chapter 8 by sequence but last on the path (HI-PATH-012), after the chapter's
three one-character segments in HI-PATH-100. HI-C14-ritu (610) comes before
the candrabindu, घ and ऋ lessons by sequence (611-613) but after all of them
on the path, since HI-S118 and HI-S136 sit in HI-PATH-100 and HI-S125 in
HI-PATH-016. A payoff has to come after what it assesses on both orders, so
neither spoken lesson could assess the script atoms, and no script lesson
could take the spoken atoms.

Each chapter now ends on a closing checkpoint placed after every chapter
lesson on both orders: the last sequence number in the chapter, and the place
right after the spoken lesson on its path segment. This follows
RU-C01-checkpoint and AR-C02-checkpoint.

| chapter | new payoff | sequence | path | before | after |
|---|---|---|---|---|---|
| 8 | HI-C08-checkpoint | 517 | HI-PATH-012 order 1 | 2/5 | 5/5 |
| 14 | HI-C14-checkpoint | 614 | HI-PATH-017 order 1 | 3/7 | 7/7 |

- **HI-C08-checkpoint**: *kṛpayā* and its root *kṛpā*, the everyday
  *baiṭhiye*; ◌ृ in कृपया said *kri*, अ opening अलविदा, ◌े on मे and in
  नमस्ते; writes कृ, अ and मे from sound. Computed 234 s, declared 260.
- **HI-C14-checkpoint**: the six seasons, *varṣā* without a Western match,
  the cold split in two, *ghī*; ◌ँ in पाँच, घ opening घी, ऋ against र; writes ऋ,
  घ and ◌ँ from memory. Computed 218 s, declared 250.
- Each has a spoken "Guided Practice" (voice core), a detachable "Script —"
  block with `[YOU READ: …]` cues and a detachable "Writing —" block
  (`dictation-transcription`).
- **Path:** two new `required` consolidation extensions,
  HI-EXT-012-CONSOLIDATION (HI-PATH-012 had no inline extension) and
  HI-EXT-017-CONSOLIDATION. HI-C09-maaf-kijiye moves to order 2 on
  HI-PATH-012.
- **chapters.d**: chapters 8 and 14 name the checkpoint and all the chapter's
  atoms. The notes now explain how the checkpoint resolves the order conflict
  and no longer say it cannot be fixed. HI-C08-kripaya and HI-C14-ritu are
  unchanged.
- Not changed: HI-S125-letter-vocalic-r opens "You have said ऋतु" while
  sitting on HI-PATH-016, before ऋतु on the path. Its extension
  (HI-EXT-016-RRI-SCRIPT) says it comes before reading ऋतु on purpose, so
  that placement is left as authored.
- Regenerated book chapters 8 and 14, narration, modality owners (`pen`,
  `voice` core) and hash shards; the Hindi curriculum digest pin moves (two
  more lessons). Hindi now has no chapter-gate debt.
