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
correct 27 · abstained 4 · wrong 0  (of 31)
by tactic — differential: 1✓/1·/0✗  ·  recall: 26✓/3·/0✗
defensibility 100%  ·  accuracy-on-attempted 100%  ·  grounded-coverage 100%
✓ never fabricated — every answer is correct-with-proof or an honest abstention.
```

The bank spans **two recall domains** — 12 inborn-errors-of-metabolism diseases
(`recall/iem-edges.adj`) and 8 vitamin deficiencies (`recall/vitamin-edges.adj`) — merged
into one store. grounded-coverage tracked the whole arc: REL-4b/REL-8 grounded the IEM
answers (→100%), REL-10 added the vitamin domain as authored-debt (→69%), and REL-10b
spider-grounded the vitamins (→**100%, both domains**). **A new domain = drop in its
`*-edges.adj` file + its board items; the harness merges and scores it unchanged**, and
its grounding climbs the same live number.

## Files

| file | what it is |
|---|---|
| `items.json` | the board-style item bank. **recall** items (fact recall as a relational binding query, gold answer or `ABSTAIN`) + **differential** items (a diagnostic case, gold leader or `ABSTAIN`). |
| `cases/*.adj` | differential case rulebooks — `prior`/`contributes` + observations + `? hypothesis` queries the native engine ranks. |
| `board_eval.py` | the harness. **recall** is scored over the grounded graph (REL-1 `RelationStore`, pure Python, 0 model calls). **differential** runs the native `adj-lang-cli` on a case and reads its ranked decision — determinate→commit, kickback/empty→abstain. Scores correct/abstained/wrong, emits `board-scorecard.json`, exits non-zero on any fabrication. If the CLI binary is absent, differential items abstain and the run logs how many were skipped (no silent caps). |
| `test_board_eval.py` | pins the defensibility contract for both tactics (never fabricate, covered→correct-with-proof, uncovered→abstain, differential commits only on decisive evidence, grounded-coverage tracks recall trust). |

Two query tactics, one defensibility metric — boards test both fact recall and
diagnostic reasoning, and both reduce to "answer-with-proof or abstain" over the
same engine.

## Run

```sh
python3 board_eval.py        # the scoreboard
python3 test_board_eval.py   # 6 tests
```

## Next

Differential and management items slot into the same `items.json` schema once the
harness drives the native `adj-lang-cli` for ranked hypotheses; expanding the bank
(more diseases, more organ systems) is how coverage grows toward a full board.
