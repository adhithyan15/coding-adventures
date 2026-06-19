# OFFLINE-DEMO — a real local model passing the board with zero online calls

This is the live-model run behind [OFFLINE-BOARD-EXAM.md](../OFFLINE-BOARD-EXAM.md):
two on-device MLX models decompose 27 prose board stems into ADJ recall queries, the
native adj-lang engine answers over the grounded edges, and the **whole path runs
inside the network-egress guard** — so `online_calls == 0` is enforced, not asserted.
Reproduce (needs `mlx_lm` + the cached model; the load is cache-only):

```sh
HF_HUB_OFFLINE=1 python run_offline_demo.py mlx-community/gemma-3-4b-it-bf16 gemma3-4b
HF_HUB_OFFLINE=1 python run_offline_demo.py Qwen/Qwen2.5-0.5B-Instruct  qwen2.5-0.5b
```

Transcripts: `offline-demo-transcript-gemma3-4b.json`, `offline-demo-transcript-qwen2.5-0.5b.json`.

## Headline

| model | decompose acc | correct | abstained | **wrong** | defensibility | online calls |
|---|---|---|---|---|---|---|
| Gemma-3-4B-it | 74% | 19 | 8 | **0** | **100%** | **0** |
| Qwen2.5-0.5B-it | 15% | 4 | 21 | 2 | 93% | **0** |

Neither model ever touched the network. The 4B model **passes the board with zero
wrong answers** — every answer it gives is correct and carries a grounded citation;
everything it can't decompose, it abstains on. The 0.5B model is far weaker at
decomposition (15%) yet still **93% defensible**, because its failures show up as
abstentions, not fabrications. That is the whole thesis in one table: *the reasoning
lives in the framework, so a small local model degrades to honest "UNKNOWN" rather
than to confident error.*

## What the faithfulness gate bought

The first run (no gate) exposed the real failure mode: grounding stops *fabrication*
but not *misdirection*. A model that mis-maps the prose to a **different but valid**
entity makes the engine answer the wrong question — a confident wrong answer:

| | decompose acc | correct | abstained | **wrong** | defensibility |
|---|---|---|---|---|---|
| Gemma-3-4B, **no gate** | 74% | 19 | 3 | **5** | 81% |
| Gemma-3-4B, **+ gate** | 74% | 19 | 8 | **0** | **100%** |
| Qwen2.5-0.5B, **no gate** | 15% | 4 | 18 | **5** | 81% |
| Qwen2.5-0.5B, **+ gate** | 15% | 4 | 21 | **2** | 93% |

The **decomposition-faithfulness gate** (`decompose_query.attested_in_stem`) requires
the chosen subject to be attested by the stem's own bytes — byte-provenance applied to
the query. Concrete rescues from the Gemma transcript (all 5 → abstention):

| stem names | model chose subject | attested? | result |
|---|---|---|---|
| Von Gierke disease | `fabry` | no | reject → abstain |
| antidiuretic hormone (ADH) | `thyroxine` | no | reject → abstain |
| hemophilia A | `factor_vii_deficiency` | no | reject → abstain |
| hemophilia A (which test) | `factor_vii_deficiency` | no | reject → abstain |
| glucagon (uncovered) | `insulin` | no | reject → abstain (gold ABSTAIN ✓) |

Decompose accuracy is **unchanged** by the gate (it is measured against the gold query
pre-gate); the gate only converts a wrong answer into an abstention. It is the
mechanism, not a claim: "no wrong answers" is enforced by checking the query against
the source, exactly as the rest of the framework checks facts against grounded bytes.

## The residual — now closed by the RELATION gate (REL-OFFLINE-2)

The subject gate alone left one gap: right-subject / **wrong-relation**. Qwen2.5-0.5B
asked `has_mcv(hereditary_spherocytosis)` for a stem about the classic *finding* —
`hereditary_spherocytosis` **is** named (so the subject gate passes), but `has_mcv`
resolves to a real edge (`normocytic`) that isn't what the question asked → a confident
wrong answer.

The **relation gate** (`decompose_query.relation_attested_in_stem`) closes it: the
stem's interrogative must contain a cue for the chosen relation (cues derived purely
from the relation name + its conventional variable — `has_mcv` + `Class` →
`{mcv, class}` — no new hand-authored knowledge; whole-word match so `class` is not
found inside `classic`). Re-scoring the committed transcripts through both gates
(deterministic — the gate is a pure function of the recorded `model_query` + stem):

| model | recorded wrong | + relation gate |
|---|---|---|
| Gemma-3-4B-it | 0 | 19 correct · 8 abstained · **0 wrong** · 100% |
| Qwen2.5-0.5B-it | 2 | 4 correct · 23 abstained · **0 wrong** · **100%** |

Qwen's two `has_mcv`-on-finding errors (`ft_hs_finding`, `ft_g6pd_finding`) flip to
abstentions; Gemma is unchanged (its mis-maps were already caught by the subject gate).
**Both models now reach 0 wrong / 100% defensibility on the recorded runs** — a weak
local model's errors are honest UNKNOWNs, never confident wrong answers.

## Caveats

- The egress guard is a tripwire, not a kernel sandbox (it patches the socket connect
  surface; `os.system("curl …")` would slip past). For this pure-Python + local MLX +
  local-subprocess path it is the right granularity.
- 27 stems across 5 domains is a demonstration, not a board. The point is the
  *property* (zero online calls, never-fabricate defensibility, model-size-graded
  abstention), measured end-to-end on real local models.
