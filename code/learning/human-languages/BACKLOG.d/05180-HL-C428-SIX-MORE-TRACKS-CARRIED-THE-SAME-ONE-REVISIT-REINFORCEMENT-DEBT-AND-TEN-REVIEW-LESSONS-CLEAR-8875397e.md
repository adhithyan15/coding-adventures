## HL-C428-8875397e — Six more tracks carried the same one-revisit reinforcement debt, and ten review lessons clear all of them

**Status: CLOSED (2026-09-23) — implemented and measured.** HL-C426 cleared
Telugu's reinforcement blocker and argued the shape generalises. This is that
claim tested against the other twenty-two tracks, and then acted on.

### The corpus-wide measurement, which had never been taken

Before choosing anything, every track's thin-atom list was dumped the same way:
`measureContinuity(lessons).reinforcement`, filtered to `revisits < 2`, to
non-etymology atoms, and to atoms whose introducing lesson derives to pre-A1.

```
track        thin  r=0   track        thin  r=0
japanese        0    0   chinese        16    6
spanish         0    0   sanskrit       28   17
telugu          0    0   german         37   19
marwadi         1    0   punjabi        40   11
persian         3    1   kannada        41    3
portuguese      6    2   hindi          44   10
italian         7    0   malayalam      44    1
urdu            7    2   tamil          73   15
latin          12    0   arabic         78   68
gujarati       13    0
russian        13    1   total                505
bengali        14    2
french         14    3
marathi        14    0
```

**505 atoms across twenty tracks**, and the distribution is the finding: the
debt is not spread evenly, it is concentrated. Twelve tracks carry sixteen or
fewer, and the six smallest carry thirty-six between them — fewer than Telugu
alone had.

That is what decided the next unit. Clearing Tamil (73) would move one track.
Clearing the six smallest moves six, for less work.

### What was done

Ten `review` lessons, no new atoms, no new headwords:

| track | lessons | what they revisit |
|---|---|---|
| marwadi | 1 | the four-skill refusal, on a figure not rehearsed |
| italian | 1 | chapters 2-4's exchanges run as one conversation, with *Prego?* repairing it |
| persian | 2 | *mamnun · zan · dust*, once as letters the script has caught up with, once counted |
| portuguese | 2 | the closings with *coração*, then *ter* + participle against *adeus* |
| urdu | 2 | the letters that differ by one mark, then the layers an Urdu word arrives from |
| latin | 2 | the social phrases sorted by what Latin attests, then *focus · mare · somnus* |

Measured, same command before and after:

```
before   marwadi 1, persian 3, portuguese 6, italian 7, urdu 7, latin 12
after    all six at 0
```

Nine of twenty-three tracks now carry no pre-A1 reinforcement debt, up from
three. **Eight** carry no reinforcement blocker, and the difference is Spanish:
it works at A2, so its reinforcement is scoped to A2 and 32 atoms are still
thin there. The table above is pre-A1 only, which is the right scope for
twenty-two tracks and the wrong one for Spanish — worth remembering before the
same probe is pointed at a track that has moved up a rung.

### THE FINDING THAT MATTERS MORE THAN THE SWEEP

Running the level gate per track afterwards — rather than the ladder — shows
what the ladder has been hiding, and it is worse than HL-C427 estimated from
one track:

```
chinese    4 blockers: spine-nodes 1, vocabulary 252, verb-vocabulary 4, reinforcement 16
french     4 blockers: vocabulary 253, verb-vocabulary 4, atom-budget 3, reinforcement 14
german     4 blockers: vocabulary 201, verb-vocabulary 2, atom-budget 5, reinforcement 37
urdu       3 blockers: vocabulary 240, verb-vocabulary 4, atom-budget 3
```

**The ladder prints one line per track showing only `vocabulary`.** Two criteria
nobody has been costing are live across most of the corpus:

- **verb-vocabulary** blocks **sixteen** tracks. The level wants five verbs at or
  below it, and sixteen tracks do not have them.
- **atom-budget** blocks **eight** tracks: french, german, italian, kannada,
  malayalam, persian, tamil, urdu.
- **spine-nodes** blocks **two**: chinese and portuguese.

Counted from the gate rather than the ladder, the twenty-three tracks carry
**64 blockers between them**, and the ladder shows 23.

Neither appears anywhere in the planning numbers this project has been working
from, because the ladder never showed them. HL-C427 is therefore not a cosmetic
reporting defect — every per-track estimate made from that ladder, including
"Telugu is 79 words from pre-A1", is an understatement of unknown size.

**Telugu and marwadi are the only two tracks whose pre-A1 rung is now down to a
single criterion**, and in both cases that criterion is vocabulary.

### Four things the gates caught that reading had not

1. **`pathOrder` is not `sequence`.** Two Latin prerequisites sit on a LATER path
   segment than the sequence numbers suggest, so a lesson correctly placed at
   sequence 1145 was still "before" its prerequisites in the curriculum graph.
   Both Latin lessons moved to `LA-PATH-032`. Filed separately.
2. **Marwadi requires exactly one objective activity per lesson**, pinned as a
   sorted list of all 346 ids. A review lesson without an `hl-activity`
   directive broke a rule no other track enforces. The pin is the documentation.
3. **Persian and Portuguese attach extensions via `after`, not `inline`.** Both
   were written as `inline` first; the git diff is what showed the convention.
4. **One banned word** — "the three above are *simply* three sizes of it".

### What this does NOT do

No exam item moves on any of the six. Reinforcement is one of five criteria, and
on five of the six tracks two or three others remain. What it buys is that the
cheapest criterion — the only one that can be cleared without buying new debt —
is now clear on nine tracks, and the remaining work on those tracks is honest
vocabulary work rather than vocabulary work that would make another number
worse.

### Where the remaining 469 sit

`arabic` (78, of which **68 have no later revisit at all**) and `tamil` (73) are
the two large ones, and arabic's shape is different in kind: a track of 129
lessons with 68 atoms taught exactly once is not carrying debt, it is missing a
review layer entirely. It wants its own entry rather than the next tranche of
this one.
