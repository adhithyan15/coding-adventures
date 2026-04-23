# markov-chain

A general-purpose Markov Chain library for Lua 5.4. This is the DT28 package from the coding-adventures monorepo.

## What it does

A Markov Chain is a mathematical model of a system that moves between a finite set of states over time. The central property — the "Markov property" — is that the probability of transitioning to the next state depends only on the current state, not on any past history.

```lua
local mc = require("coding_adventures.markov_chain")
local MarkovChain = mc.MarkovChain

-- Build a weather model from observations.
local chain = MarkovChain.new(1, 0.1)
chain:train({"Sunny", "Cloudy", "Sunny", "Sunny", "Rainy", "Cloudy", "Sunny"})

-- What comes after Sunny?
local next = chain:next_state("Sunny")    -- "Sunny", "Cloudy", or "Rainy"

-- Generate a 7-step forecast.
local forecast = chain:generate("Sunny", 7)
-- e.g. {"Sunny", "Sunny", "Cloudy", "Rainy", "Cloudy", "Sunny", "Sunny"}
```

## Where it fits

This package sits at layer DT28 in the coding-adventures dependency stack. It depends on the directed graph package (layer DT20) for topology tracking — the graph records which states and transitions have been observed, while transition probabilities are stored separately.

Markov Chains appear throughout the monorepo's compression packages: LZMA-family compressors (CMP06, CMP08) use per-context probability models that are conceptually Markov chains over byte values.

## Installation

```bash
luarocks install coding-adventures-markov-chain
```

Or add to your rockspec:

```lua
dependencies = {
    "coding-adventures-markov-chain >= 0.1.0",
}
```

## Usage

### Basic character-level text generation (order-1)

```lua
local mc = require("coding_adventures.markov_chain")
local MarkovChain = mc.MarkovChain

local chain = MarkovChain.new(1, 0.0)
chain:train_string("the quick brown fox jumps over the lazy dog ")

-- Generate a 50-character string starting from "t".
local text = chain:generate_string("t", 50)
```

### Order-2 chain for more realistic output

```lua
-- Higher-order chains condition on more context and produce more
-- coherent text, at the cost of needing more training data.
local chain = MarkovChain.new(2, 0.1)
chain:train_string("the quick brown fox jumps over the lazy dog " .. string.rep("abcde ", 20))
local text = chain:generate_string("th", 100)
```

### Laplace smoothing

Smoothing prevents the chain from getting stuck in states with no observed outgoing transitions. It's critical when training on small corpora.

```lua
-- Pre-register the full alphabet so smoothing covers all transitions,
-- including those to states not seen in training.
local chain = MarkovChain.new(1, 1.0, {"A", "B", "C"})
chain:train({"A", "B"})

-- P(A→C) = (0+1) / (1+1*3) = 0.25  (smoothed)
-- P(A→B) = (1+1) / (1+1*3) = 0.50  (smoothed)
print(chain:probability("A", "C"))  -- 0.25
print(chain:probability("A", "B"))  -- 0.50
```

### Stationary distribution (PageRank-like)

```lua
-- For an ergodic chain, the stationary distribution π tells you the
-- long-run fraction of time spent in each state.
local chain = MarkovChain.new(1, 0.1, {"A", "B", "C"})
chain:train({"A", "B", "C", "A", "B", "C", "A"})
local pi = chain:stationary_distribution()
-- pi["A"] + pi["B"] + pi["C"] ≈ 1.0
```

### Transition matrix inspection

```lua
local chain = MarkovChain.new()
chain:train({"A", "B", "A", "C"})
local tm = chain:transition_matrix()
-- tm["A"]["B"] = 0.5
-- tm["A"]["C"] = 0.5
-- tm["B"]["A"] = 1.0
```

## API Reference

| Method | Description |
|--------|-------------|
| `MarkovChain.new(order, smoothing, states)` | Constructor. Defaults: order=1, smoothing=0.0, states=nil. |
| `chain:train(sequence)` | Train on an array of state strings. Accumulates counts; can be called multiple times. |
| `chain:train_string(text)` | Train on a string, treating each character as a state. |
| `chain:next_state(current)` | Sample one transition. Raises `error()` if `current` is unknown. |
| `chain:generate(start, length)` | Return an array of exactly `length` states starting from `start`. |
| `chain:generate_string(seed, length)` | Return a string of exactly `length` characters. |
| `chain:probability(from, to)` | Transition probability. Returns 0.0 for unknown transitions. |
| `chain:stationary_distribution()` | Power-iteration stationary distribution. |
| `chain:states()` | List of all registered states (registration order). |
| `chain:transition_matrix()` | Deep copy of the full transition table. |

## Order-k chains

For order-k chains with k > 1, `next_state()` and `probability()` expect the context key, which is `k` state strings joined by a null byte `"\0"`:

```lua
local chain = MarkovChain.new(2)
chain:train_string("abcabcabc")

-- Context key for the digram ("a", "b") is "a\0b"
print(chain:probability("a\0b", "c"))  -- 1.0

-- generate_string handles this automatically:
chain:generate_string("ab", 9)  -- "abcabcabc"
```

## Development

```bash
# Run tests (requires busted and luarocks)
bash BUILD

# Or manually:
cd tests && busted . --verbose --pattern=test_
```

## Architecture

The package uses a single module: `src/coding_adventures/markov_chain/init.lua`.

Internally:

- `_states` / `_state_set` — the alphabet (ordered list + hash for O(1) lookup)
- `_counts` — raw transition counts: `{context → {target → integer}}`
- `_transitions` — normalised probabilities: `{context → {target → float}}`
- `_graph` — a `DirectedGraph` (self-loops allowed) for topology tracking
- `_normalise()` — rebuilds `_transitions` from `_counts` with smoothing
- `_sample(row)` — inverse-CDF sampling over a probability row

For order-k chains the context key is `table.concat(k-gram, "\0")`, making each k-gram a single string key in the transition table. This keeps the implementation simple and avoids nested tables for contexts.
