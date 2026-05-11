# llm-primitives (Rust)

Skeleton for the typed LLM primitives layer of the adjudication
framework. This crate holds the shared scaffolding every primitive
needs; the six concrete primitives (`decompose_text`, `render_node`,
`entail`, `find_contradicting_reading`, `judge_plausibility`,
`extract_rules`) ship as follow-up PRs that depend on this one.

## What This Is

Reference implementation of [LM00b — LLM Primitives](../../../specs/LM00b-llm-primitives.md).

The framework never lets its checkers / extractor call an
`LlmClient` directly. Instead, every LLM-driven operation goes
through a **typed primitive**: a pure function from typed input to
typed output, with a versioned prompt, a JSON-schema-validated
output, an audit-trail record, and a deterministic retry harness.
This crate defines the building blocks every primitive plugs into.

## Where It Fits

```text
   framework consumers (extractor, ADJ02–05, ADJ06, ADJ09)
        │
        ▼
   llm-primitives                            ← this crate (scaffolding)
        │     ├── decompose_text             ← follow-up crate
        │     ├── render_node                ← follow-up crate
        │     ├── entail                     ← follow-up crate
        │     ├── find_contradicting_reading ← follow-up crate
        │     ├── judge_plausibility         ← follow-up crate
        │     └── extract_rules              ← follow-up crate
        │
        ▼
   llm-gateway (LlmClient trait + types)
        │
        ▼
   llm-provider-anthropic / -openai / -ollama / -mock
```

## Public API

**Scaffolding (v0.1.0):**

| Item | Role |
|---|---|
| `Role` enum | Which slot a primitive fills (Extractor / Renderer / Nli / Adversary / Plausibility / RuleExtractor) |
| `GatewayConfig` | Role → `Box<dyn LlmClient>` registry used by every primitive |
| `IndependenceViolation` | ADJ05 check: extractor and adversary must be different model families |
| `LlmCallRecord` | Audit-trail row for one LLM call (provider identity, prompt hash, usage, latency, cost) |
| `PrimitiveCallRecord` | Per-primitive audit record wrapping retry attempts |
| `PrimitiveError` | `Gateway` / `ValidationExhausted` / `StructuralFailure` / `NoClientForRole` |
| `DECOMPOSE_TEXT_PROMPT_VERSION` … `EXTRACT_RULES_PROMPT_VERSION` | Six version constants |
| `fingerprint_prompt(&CompletionRequest)` | Deterministic content hash of the prompt portion |

**Primitives:**

| Primitive | Status | Role | Module |
|---|---|---|---|
| `entail(req, gateway)` | ✅ v0.2.0 | `Nli` | `entail` |
| `decompose_text` | planned | `Extractor` | — |
| `render_node` | planned | `Renderer` | — |
| `find_contradicting_reading` | planned | `Adversary` | — |
| `judge_plausibility` | planned | `Plausibility` | — |
| `extract_rules` | planned | `RuleExtractor` | — |

The crate has **no async runtime dependency**: the `LlmClient` trait
in `llm-gateway` is synchronous in v0.1.0, and so is everything
that wraps it here. A future async surface can be added without
breaking these types.

## Usage

```rust
use llm_primitives::{GatewayConfig, Role, fingerprint_prompt};
use llm_gateway::MockLlmClient;

let gateway = GatewayConfig::new()
    .with_client(Role::Extractor, Box::new(MockLlmClient::new()))
    .with_client(Role::Renderer,  Box::new(MockLlmClient::new()));

// ADJ05 sanity check at startup:
gateway.check_independence().expect("extractor and adversary must differ");

// Each primitive crate calls `gateway.client(Role::Extractor)` etc.
```

## Why a Skeleton-Only PR

The six primitives can be implemented in parallel once the shared
types are in place. Shipping the skeleton first as a small,
mechanically reviewable PR unlocks six concurrent implementation
PRs that won't conflict on a single giant file. This is the
fork-point pattern: ship the cheap multiplier first.
