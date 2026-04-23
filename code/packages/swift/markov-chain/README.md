# CodingAdventuresMarkovChain

A general-purpose Markov Chain library for Swift, implementing DT28 from the
coding-adventures monorepo.

## What is a Markov Chain?

A Markov Chain models a system that moves between **states** over time, where
the probability of the next state depends *only* on the current state — not
on any prior history. This "memorylessness" is the **Markov property**.

Classic applications include text generation, PageRank, LZMA compression,
biology (nucleotide models), and Monte Carlo sampling.

## Installation

Add to your `Package.swift`:

```swift
.package(path: "../../swift/markov-chain"),
```

Then add `"CodingAdventuresMarkovChain"` to your target's dependencies.

## Usage

### Basic training and sampling

```swift
import CodingAdventuresMarkovChain

var chain = MarkovChain(order: 1, smoothing: 0.0)
chain.train(["A", "B", "A", "C", "A", "B", "B", "A"])

print(chain.probability(from: "A", to: "B"))  // ≈ 0.667
let next = try chain.nextState("A")            // "B" or "C"
let seq  = try chain.generate(start: "A", length: 10)
```

### Character-level text generation

```swift
var chain = MarkovChain(order: 2, smoothing: 1.0)
chain.trainString(String(repeating: "the quick brown fox jumps over the lazy dog ", count: 20))

let text = try chain.generateString(seed: "th", length: 100)
// → "the quick brown fox jumps over the lazy dog the quick..."
```

### Stationary distribution

```swift
var chain = MarkovChain(order: 1, smoothing: 0.1)
chain.train(["A", "B", "C", "A", "B", "A", "C", "B"])

let pi = try chain.stationaryDistribution()
// pi["A"] ≈ fraction of time spent in state A in the long run
```

### Laplace smoothing

Smoothing prevents the chain from getting stuck by giving every unseen
transition a pseudo-count of α:

```swift
// With smoothing=1.0 and 3 declared states, P(A→C) = 1/4
var chain = MarkovChain(order: 1, smoothing: 1.0, states: ["A", "B", "C"])
chain.train(["A", "B"])
chain.probability(from: "A", to: "C")  // 0.25
```

### Order-k chains

Higher-order chains remember more history, producing more realistic text at
the cost of more training data:

```swift
var chain = MarkovChain(order: 2, smoothing: 0.0)
chain.train(Array("abcabcabc").map { String($0) })

try chain.generateString(seed: "ab", length: 9)  // "abcabcabc"
```

## API

```swift
public struct MarkovChain {
    public init(order: Int = 1, smoothing: Double = 0.0, states: [String] = [])
    public mutating func train(_ sequence: [String])
    public mutating func trainString(_ text: String)
    public func nextState(_ current: String) throws -> String
    public func generate(start: String, length: Int) throws -> [String]
    public func generateString(seed: String, length: Int) throws -> String
    public func probability(from: String, to: String) -> Double
    public func stationaryDistribution() throws -> [String: Double]
    public func states() -> [String]
    public func transitionMatrix() -> [String: [String: Double]]
}

public enum MarkovError: Error {
    case unknownState(String)
    case notErgodic
    case seedTooShort(required: Int, got: Int)
}
```

## Stack Position

- **Spec**: `code/specs/DT28-markov-chain.md`
- **Depends on**: `code/packages/swift/directed-graph` (for transition topology)
- **Used by**: Future compression (CMP08/LZMA conceptual foundation),
  text-generation tools, procedural-generation packages

## Testing

```
swift test --enable-code-coverage
```

All 10 spec test cases plus additional edge-case tests are in
`Tests/CodingAdventuresMarkovChainTests/MarkovChainTests.swift`.
