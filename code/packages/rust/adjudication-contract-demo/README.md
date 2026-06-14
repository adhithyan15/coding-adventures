# adjudication-contract-demo (Rust)

Third-domain A/B demo. Same framework, contract-review domain.

## Why a third demo

After TSA (compliance) and clinical (triage), contracts give us a
domain whose IR shape is meaningfully different: the **rule +
exception** structure (`NodeKind::Rule` with `Conditional` modality
+ `NodeKind::Exception` referencing the rule via `part_of`).

Small models often produce IRs that drop the exception entirely.
This is exactly the kind of structural omission the framework's
ADJ02 + ADJ03 are designed to catch.

## The fixture (105 bytes)

```text
If the buyer pays within 30 days, the seller delivers the goods, unless the goods are out of stock.
```

Hand-built IR:

| Node | Kind      | Modality      | Term                                                      |
|------|-----------|---------------|-----------------------------------------------------------|
| R1   | Rule      | Conditional   | `implies(payment_within(30_days), delivers(seller, goods))` |
| E1   | Exception | Present       | `out_of_stock(goods)` — `part_of: R1`                     |
| Q1   | Query     | Present       | `delivers(seller, goods)?`                                |

Span tiling: R1 covers bytes 0..p1, E1 covers p1..end where `p1` is
the byte index where `" unless"` starts. ADJ02 passes; ADJ03 sees
the Conditional rule with an Affirmed exception; ADJ04 round-trips
each; ADJ05 attacks each.

## Quick start

```bash
ollama serve     # in another terminal
ollama pull gemma4:latest
ollama pull llama3.1:8b      # for ADJ05

ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434 \
  ADJ_DEMO_ADVERSARY_MODEL=llama3.1:8b \
  cargo run -p adjudication-contract-demo
```

## Env vars

Same set as the other demos: `ADJ_DEMO_{ENDPOINT, MODEL,
ADVERSARY_MODEL, SOURCE, CACHE_DIR, TIMEOUT_SECS, AUDIT}`.

## What v0.1 ships

- `DemoConfig`, `run_raw_arm`, `run_pipeline_arm`,
  `contract_ir_document`, `format_side_by_side`.
- 7 offline tests cover canonical IR shape, conditional modality,
  `part_of` link from exception to rule, span tiling, fallback
  behavior.

## What v0.1 deliberately does NOT do

- LlmExtracted mode. Hand-built IR is the v0.1 baseline.
- Real legal authority. The IR shape is illustrative, not a
  contract-law reference.
