# coding-adventures-markov-chain (Go)

A general-purpose Markov Chain library for training on sequences and sampling
from the learned model. Part of the CodingAdventures stack (DT28).

## What is a Markov Chain?

A Markov Chain models a system that moves between a finite set of **states**
over time. The defining property is **memorylessness**: the probability of
the next state depends **only** on the current state, not on any history.

```
States:       S = {s₀, s₁, …, sₙ₋₁}
Transitions:  P(next = sⱼ | current = sᵢ) = T[i][j]
```

The full model is captured in a **transition matrix** T where each row sums
to 1.0 (the chain always goes somewhere).

## Where it fits in the stack

- **CMP06 (Brotli)** uses context-adaptive probabilities conceptually similar
  to Markov Chain transition tables — a compressor maintains one probability
  estimate per "state" (recent context) and updates it as symbols are coded.
- **LZMA (CMP08, future)** formalizes this: each byte value is a state, and
  transition probabilities feed a range coder. DT28 is the general training
  and sampling layer; LZMA-specific adaptive probability update rules would
  build on top.

## Installation

```
go get github.com/adhithyan15/coding-adventures/code/packages/go/markov-chain
```

Or with a local `replace` directive in your `go.mod`:

```
require github.com/adhithyan15/coding-adventures/code/packages/go/markov-chain v0.0.0
replace github.com/adhithyan15/coding-adventures/code/packages/go/markov-chain => ../markov-chain
```

## Quick Start

### Order-1 chain (standard)

```go
package main

import (
    "fmt"
    markovchain "github.com/adhithyan15/coding-adventures/code/packages/go/markov-chain"
)

func main() {
    // Create a chain with order=1, Laplace smoothing, no pre-registered states.
    m := markovchain.New(1, 1.0, nil)

    // Train on a sequence of weather observations.
    m.Train([]string{
        "Sunny", "Cloudy", "Rainy", "Sunny", "Sunny",
        "Cloudy", "Rainy", "Rainy", "Sunny", "Cloudy",
    })

    // Sample the next state from "Sunny".
    next, err := m.NextState("Sunny")
    if err != nil {
        panic(err)
    }
    fmt.Println("Next state:", next)

    // Generate a 10-step forecast.
    forecast, _ := m.Generate("Sunny", 10)
    fmt.Println("Forecast:", forecast)

    // Inspect transition probabilities.
    fmt.Printf("P(Sunny→Rainy) = %.3f\n", m.Probability("Sunny", "Rainy"))

    // Compute the long-run distribution.
    dist, _ := m.StationaryDistribution()
    for state, prob := range dist {
        fmt.Printf("Long-run %s: %.3f\n", state, prob)
    }
}
```

### Order-2 chain (character-level text)

```go
m := markovchain.New(2, 0.0, nil)

// Train on a repeating pattern — order-2 chains memorize exact subsequences.
m.TrainString("abcabcabcabc")

// Seed must be at least `order` (2) characters long.
// The last 2 characters form the initial context window.
text, err := m.GenerateString("ab", 12)
if err != nil {
    panic(err)
}
fmt.Println(text) // "abcabcabcabc"
```

### Laplace smoothing

```go
// Pre-register all states so unseen transitions get smoothed correctly.
m := markovchain.New(1, 1.0, []string{"A", "B", "C"})
m.Train([]string{"A", "B"})

// P(A→C) = 1/(1 + 3) = 0.25  (1 smooth count / (1 raw + 3×α))
fmt.Println(m.Probability("A", "C")) // 0.25
```

## API Reference

```go
// Construction
func New(order int, smoothing float64, states []string) *MarkovChain

// Training — may be called multiple times; counts accumulate.
func (m *MarkovChain) Train(sequence []string) error
func (m *MarkovChain) TrainString(text string) error

// Sampling
func (m *MarkovChain) NextState(current string) (string, error)
func (m *MarkovChain) Generate(start string, length int) ([]string, error)
func (m *MarkovChain) GenerateString(seed string, length int) (string, error)

// Queries
func (m *MarkovChain) Probability(from, to string) float64
func (m *MarkovChain) StationaryDistribution() (map[string]float64, error)

// Inspection
func (m *MarkovChain) States() []string
func (m *MarkovChain) TransitionMatrix() map[string]map[string]float64
```

## Order-k chains

For `order > 1` the context key is `k` tokens joined by a null byte
(`"\x00"`). When using `Probability` or `NextState` directly:

```go
m := markovchain.New(2, 0.0, nil)
m.Train([]string{"a", "b", "c", "a", "b", "c"})

// Query using the raw k-gram key.
p := m.Probability("a\x00b", "c") // 1.0
```

`GenerateString` handles the windowing automatically — just provide a seed
of at least `order` characters:

```go
s, _ := m.GenerateString("ab", 9) // "abcabcabc"
```

## Smoothing

| α value | Name              | Effect                                                   |
|---------|-------------------|----------------------------------------------------------|
| `0.0`   | No smoothing      | Unseen transitions have probability 0 — chain can freeze |
| `1.0`   | Laplace smoothing | Each unseen transition gets 1 pseudo-count               |
| `0 < α < 1` | Lidstone      | Fractional pseudo-counts; gentler regularization         |

With smoothing > 0 it is recommended to pre-register the full alphabet via the
`states` parameter so unseen transitions are distributed over ALL possible
states, not just those observed.

## Stationary distribution

The stationary distribution π satisfies π · T = π: it is the long-run
fraction of time the chain spends in each state. Computed via power iteration
(up to 10,000 iterations, threshold 1e-10). Returns an error if the chain is
not ergodic.

```go
dist, err := m.StationaryDistribution()
// dist["Sunny"] ≈ 0.47 for the weather example
```

## Dependencies

- [`directed-graph`](../directed-graph) — topology tracking for state nodes.
  The Markov chain stores which (context, target) pairs have been observed
  as edges in the graph, and keeps probabilities separately in a map.

## Running tests

```bash
cd code/packages/go/markov-chain
mise exec -- go mod tidy
mise exec -- go test ./... -v -cover
```
