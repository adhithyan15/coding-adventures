# llm-cache (Rust)

Content-addressed prompt cache. Wraps any `LlmClient` with a
`(model, prompt_hash)`-keyed in-memory cache. Pure in-memory, zero
third-party deps.

## Why this exists

The framework's primitives are deterministic: `temperature: 0.0`,
content-addressed via `llm_primitives::fingerprint_prompt`. Same
input → same output, always. Caching it is **sound**.

For the framework's "small local models do extraordinary work"
design principle, the asymmetry between expensive-to-produce model
output and cheap-to-compare prompt hashes is the load-bearing
economic argument. A demo run that calls `render_node` + `entail`
6 times per IR document, plus `decompose_text` once, is doing 7
LLM round-trips that get repeated across debug iterations, test
fixtures, replay scenarios, and the ADJ06 retry loop. Caching turns
those 7 round-trips into 0 on subsequent runs.

## Usage

```rust
use llm_cache::CachingClient;
use llm_provider_ollama::OllamaClient;
use llm_primitives::{GatewayConfig, Role};

let inner = OllamaClient::new("gemma4:latest");
let cached = CachingClient::new(Box::new(inner));

// Register in a GatewayConfig like any other LlmClient:
let gateway = GatewayConfig::new()
    .with_client(Role::Renderer, Box::new(cached));

// ... run primitives ...

// Inspect cache telemetry:
// let stats = cached.stats();
// println!("hit rate: {:.0}%", stats.hit_rate() * 100.0);
```

## What v0.1 ships

- `CachingClient` — wraps a `Box<dyn LlmClient>` and implements
  `LlmClient` itself, so it drops into any `GatewayConfig` slot.
- `CacheStats { hits, misses, entries }` with a `hit_rate()`
  helper.
- Optional capacity limit (FIFO eviction) for bounded memory use.
- Separate keying for `complete_json` calls that includes the
  schema name, so `entail` and `judge_plausibility` calling the
  same model with structurally similar prompts don't collide.
- 10 unit tests covering: cache hit on identical prompt, distinct
  prompts get distinct entries, complete_json keys per schema,
  capacity-bounded FIFO eviction, clear preserves stats, hit-rate
  arithmetic, identity delegation, and model-name-included keys.

## What v0.1 deliberately does NOT do

- **Disk persistence.** v0.2 will add content-addressed files
  keyed on `prompt_hash`, so cache survives a process restart.
- **TTL / staleness.** Deterministic prompts don't go stale.
- **Cross-call dependency tracking.** Each entry is one
  `(request, response)` pair.

## Wire format

The cache key is `"<model>|<prompt_hash>[|<schema_name>]"`:

- `<model>` falls back to the inner client's `identity().model_family`
  if the request's `model` field is empty.
- `<prompt_hash>` is the FNV-1a 64-bit hex of the system + messages
  (same fingerprint primitives use for the audit trail).
- `<schema_name>` is appended for `complete_json` calls so the same
  prompt with different schemas doesn't collide.
