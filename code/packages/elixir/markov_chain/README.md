# coding_adventures_markov_chain

**DT28** — A general-purpose Markov Chain library for Elixir.

Part of the [coding-adventures](https://github.com/coding-adventures) polyglot monorepo.

## What is a Markov Chain?

A Markov Chain is a mathematical model for systems that hop between a finite set of
**states** over time. The defining property is that the probability of the next state
depends **only on the current state** — not on any history before it. This
"memorylessness" is called the **Markov property**.

```
States:   {Sunny, Cloudy, Rainy}

Transition matrix T:
          → Sunny  → Cloudy  → Rainy
Sunny  [    0.7      0.2      0.1  ]
Cloudy [    0.3      0.4      0.3  ]
Rainy  [    0.2      0.3      0.5  ]
```

Row "Sunny" reads: "If it is Sunny today, tomorrow it will be Sunny (70%), Cloudy (20%),
or Rainy (10%)."

## How it fits in the stack

- **DT27 (DirectedGraph)** — this package depends on `CodingAdventures.DirectedGraph` to
  store the chain's topology. Every observed transition becomes a directed edge.
- **CMP06 (Brotli)** — uses Markov chain probability tables for context modelling (the
  connection between DT28 and CMP06 is conceptual; this package is a general-purpose
  implementation).

## Features

- Pure functional design — all functions return new `%MarkovChain{}` structs.
- **Order-k chains** — extend memory window beyond one state (`order: 2` uses bigrams as
  context keys, etc.).
- **Laplace / Lidstone smoothing** — avoids stuck chains by adding α to all transition
  counts.
- **Power-iteration stationary distribution** — converges to π · T = π.
- **Text generation** — convenient `train_string/2` and `generate_string/3` for
  character-level models.

## Usage

```elixir
alias CodingAdventures.MarkovChain

# Basic order-1 chain
chain = MarkovChain.new()
chain = MarkovChain.train(chain, ~w[A B A C A B B A])

MarkovChain.probability(chain, "A", "B")   # ≈ 0.667
MarkovChain.probability(chain, "A", "C")   # ≈ 0.333

{:ok, next}  = MarkovChain.next_state(chain, "A")      # "B" or "C"
{:ok, seq}   = MarkovChain.generate(chain, "A", 10)    # 10-element list
{:ok, dist}  = MarkovChain.stationary_distribution(chain)

# Order-2 character chain
chain2 = MarkovChain.new(2)
chain2 = MarkovChain.train_string(chain2, "abcabcabc")
{:ok, text} = MarkovChain.generate_string(chain2, "ab", 9)
# => "abcabcabc"

# Laplace smoothing — never get stuck
chain3 = MarkovChain.new(1, 1.0, ["A", "B", "C"])
chain3 = MarkovChain.train(chain3, ["A", "B"])
MarkovChain.probability(chain3, "A", "C")  # 0.25 (not 0.0)
```

## API

| Function | Description |
|---|---|
| `new(order, smoothing, states)` | Create an empty chain. |
| `train(chain, sequence)` | Accumulate transition counts from a sequence. |
| `train_string(chain, text)` | Convenience: train on individual characters. |
| `next_state(chain, current)` | Sample one transition from `current`. |
| `generate(chain, start, length)` | Generate a sequence of `length` states. |
| `generate_string(chain, seed, length)` | Generate `length` characters as a string. |
| `probability(chain, from, to)` | Return `T[from][to]`. |
| `stationary_distribution(chain)` | Power iteration to find π · T = π. |
| `states(chain)` | List all known single states. |
| `transition_matrix(chain)` | Return full transition map. |

## Running tests

```bash
mix deps.get
mix test
mix test --cover
```

## Dependencies

- [`coding_adventures_directed_graph`](../directed_graph) — local path dependency for
  graph topology.
- [`excoveralls`](https://hex.pm/packages/excoveralls) — coverage reporting (test env
  only).
