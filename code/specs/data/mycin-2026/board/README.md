# board/ — the board-style defensibility scoreboard (REL-5)

The long-term goal is to **pass medical board exams**, built organ-by-organ
([[project_board_exam_goal]]). This is the scoreboard that makes progress
measurable: it runs a bank of board-style fact-recall questions end-to-end over the
grounded knowledge graph and scores each into **three** outcomes, not one.

| outcome | meaning |
|---|---|
| **correct** | answered, matches the key, **with a proof** (the citing edge) |
| **abstained** | returned UNKNOWN — no grounded edge supports an answer (the *honest* failure; the discriminator vs a hallucinating model) |
| **wrong** | answered incorrectly — the **only real failure** |

The headline is the **defensibility curve**: on the covered subset, near-100%
correct-with-proof; on the uncovered subset, abstain rather than fabricate. The
harness **gates on never-fabricate** — a single `wrong` is a non-zero exit.

## The live grounding number

The scorecard also reports **grounded-coverage**: of the correct answers, how many
cite an `authoritative` (spider-grounded) edge vs a `consensus` (authored-debt)
edge. That is the number every grounding/expansion PR moves:

```
correct 18 · abstained 2 · wrong 0  (of 20)
defensibility 100%  ·  accuracy-on-attempted 100%  ·  grounded-coverage 50%
✓ never fabricated — every answer is correct-with-proof or an honest abstention.
```

The number tracks **both levers**, honestly. The REL-4b spider grounded 9 of the
original answers (`consensus → authoritative`). REL-6 then expanded the bank from 6 to
**12 diseases** — the 6 new disease sets enter as authored-debt, so grounded-coverage
**dipped 90% → 50%**. Re-running `recall/ground-iem-edges.workflow.js` →
`iem_edge_ground.py` on the new edges climbs it back. **Expansion adds debt; grounding
retires it; the scoreboard shows both** — with no change to this harness.

## Files

| file | what it is |
|---|---|
| `items.json` | the board-style item bank — fact-recall questions as relational binding queries, each with a gold answer (or `ABSTAIN` for a deliberately-uncovered disease). |
| `board_eval.py` | the harness — runs each item over the grounded graph (REL-1 `RelationStore`, pure Python, 0 model calls), scores correct/abstained/wrong, emits `board-scorecard.json`, exits non-zero on any fabrication. |
| `test_board_eval.py` | pins the defensibility contract (never fabricate, covered→correct-with-proof, uncovered→abstain, grounded-coverage tracks trust). |

## Run

```sh
python3 board_eval.py        # the scoreboard
python3 test_board_eval.py   # 6 tests
```

## Next

Differential and management items slot into the same `items.json` schema once the
harness drives the native `adj-lang-cli` for ranked hypotheses; expanding the bank
(more diseases, more organ systems) is how coverage grows toward a full board.
