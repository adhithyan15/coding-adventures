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
correct 59 · abstained 7 · wrong 0  (of 66)
by tactic — differential: 1✓/1·/0✗  ·  management: 5✓/0·/0✗  ·  recall: 53✓/6·/0✗
defensibility 100%  ·  accuracy-on-attempted 100%  ·  grounded-coverage 96%
✓ never fabricated — every answer is correct-with-proof or an honest abstention.
```

The board now scores **all three board question-types** over one engine + one
defensibility metric: **recall** (binding queries over the grounded graph),
**differential** (the LR engine ranks hypotheses; commit or abstain), and
**management** (the chart-as-constraints engine compiles a patient context into a
constraint program and SOLVES a regimen — or proves it INFEASIBLE with a named
conflict). Constraints solved OR made unsatisfiable; both are defensible, neither
fabricates.

The bank spans **five recall domains** — 12 inborn-errors-of-metabolism diseases
(`recall/iem-edges.adj`), 8 vitamin deficiencies (`recall/vitamin-edges.adj`), 8 anemia
classifications (`recall/anemia-edges.adj`), 8 endocrine hormones
(`recall/endocrine-edges.adj`), and 5 coagulation / bleeding disorders queried three ways
(`recall/coag-edges.adj`) — merged into one store, **all spider-grounded** (REL-8 /
REL-10b / REL-11b / REL-12b / REL-13b). grounded-coverage is **96%**: 51 of 53 recall
answers cite an `authoritative` edge; the two holdouts (`cortisol_def`,
`factor7_def_factor`) stay `consensus` + FLAG because the adversarial verify could not pin
them verbatim — the framework declines to claim grounding it cannot defend, by design.
Each new domain entered as authored-debt (dipping the number) and its spider run retired
it — expansion adds debt, grounding
retires it, one watchable number. **A new domain = drop in its `*-edges.adj` file + its
board items + the filename in `EDGE_FILES`; the harness merges and scores it unchanged.**

## Files

| file | what it is |
|---|---|
| `items.json` | the board-style item bank. **recall** items (fact recall as a relational binding query, gold answer or `ABSTAIN`) + **differential** items (a diagnostic case, gold leader or `ABSTAIN`). |
| `cases/*.adj` | differential case rulebooks — `prior`/`contributes` + observations + `? hypothesis` queries the native engine ranks. |
| `board_eval.py` | the harness. **All three tactics — recall, differential, management — run on the ONE native `adj-lang-cli`** (recall as a `? relation(subject, $Var)` binding query over the imported grounded edges; differential as a ranked decision; management as a chart-as-constraints solve). Scores correct/abstained/wrong, emits `board-scorecard.json`, exits non-zero on any fabrication. With the CLI absent, engine-backed items abstain and the run logs how many were skipped (no silent caps, no Python fallback). |
| `test_board_eval.py` | pins the defensibility contract for all tactics (never fabricate, covered→correct-with-proof, uncovered→abstain, differential commits only on decisive evidence, grounded-coverage tracks recall trust). |

Three query tactics, **one engine**, one defensibility metric — boards test fact
recall, diagnostic reasoning, and management, and all three reduce to
"answer-with-proof or abstain" over the same native adj-lang engine. (Recall used to
take a Python `RelationStore` shortcut; it now runs on the engine like everything
else — `recall.py` is deprecated.)

## Offline mode — prose questions, zero online model calls

Real board items are *prose*. `board_offline.py` adds the missing frontend: a
**local, in-memory** model decomposes a prose stem into an ADJ recall query, then the
native engine answers — with the whole path wrapped in a network-egress guard so the
run *proves* it made no online call. A **faithfulness gate** requires the model's
chosen subject to be attested by the stem's own bytes, so a mis-decomposition becomes
an honest abstention rather than a wrong answer. See
[OFFLINE-BOARD-EXAM.md](../OFFLINE-BOARD-EXAM.md) and
[OFFLINE-DEMO.md](OFFLINE-DEMO.md).

| file | what it is |
|---|---|
| `free_text_board.json` | prose board stems (gold answer + gold query) across all five domains |
| `offline_guard.py` | `no_network()` — a reusable egress tripwire that raises on any non-loopback outbound connection |
| `decompose_query.py` | prose → `{relation, subject, $Var}` via an injected local-model generator (constrained to legal relations + canonical subjects; faithfulness gate on the subject) |
| `board_offline.py` | decompose → answer on the native engine inside `no_network()` → score; cached (deterministic) or `--model PATH` (live local MLX) |
| `run_offline_demo.py` | live demo driver; writes `offline-demo-transcript-<tag>.json` |

## Run

```sh
python3 board_eval.py          # structured-item scoreboard (recall+differential+management)
python3 test_board_eval.py     # 12 tests
python3 board_offline.py       # offline prose pipeline (cached gold queries, 0 online calls)
python3 test_board_offline.py  # 14 tests
# live local-model decode (needs mlx_lm + a cached model):
HF_HUB_OFFLINE=1 python run_offline_demo.py mlx-community/gemma-3-4b-it-bf16 gemma3-4b
```

## Next

Expanding the bank (more diseases, more organ systems) grows coverage toward a full
board; routing prose *vignettes* (infer the disease from findings, then recall) through
the differential tactic is the reverse-direction slice of the same engine.
