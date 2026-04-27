# markov-chain (Perl)

`CodingAdventures::MarkovChain` — Pure-Perl general-purpose Markov Chain.

## What it is

A Markov Chain is a mathematical model of a system that moves between a finite
set of **states** over time, where the probability of going to the next state
depends **only on the current state** — not on any earlier history.  This
"memorylessness" property is called the **Markov property**.

This module lets you:

1. **Train** on any sequence of values (characters, words, symbols, …).
2. **Sample** one step from the learned distribution.
3. **Generate** sequences of arbitrary length.
4. **Inspect** the transition matrix and stationary distribution.

Spec: [`code/specs/dt28-markov-chain.md`](../../../../specs/dt28-markov-chain.md)

## How it fits in the stack

`CodingAdventures::MarkovChain` (DT28) sits above
`CodingAdventures::DirectedGraph` (which it uses for topology) and below
compression packages such as CMP06 (Huffman) and the planned CMP08 (LZMA),
which use the Markov Chain to model symbol probabilities.

```
CMP08  ← uses → DT28 MarkovChain ← uses → CodingAdventures::DirectedGraph
CMP06  ← uses → DT28 MarkovChain
```

## Installation

```bash
# From the package root
bash BUILD
```

The BUILD script installs the `directed-graph` dependency first, then installs
`Test::More` and runs the test suite.

## Usage examples

### Order-1 chain (default)

```perl
use CodingAdventures::MarkovChain;

my $chain = CodingAdventures::MarkovChain->new;
$chain->train(['A', 'B', 'A', 'C', 'A', 'B', 'B', 'A']);

printf "P(A->B) = %.3f\n", $chain->probability('A', 'B');  # 0.667
printf "P(A->C) = %.3f\n", $chain->probability('A', 'C');  # 0.333

my $seq = $chain->generate('A', 10);
print join(', ', @$seq), "\n";
# e.g. A, B, A, B, A, C, A, B, B, A
```

### Laplace smoothing

```perl
my $smooth = CodingAdventures::MarkovChain->new(
    order     => 1,
    smoothing => 1.0,
    states    => ['A', 'B', 'C'],
);
$smooth->train(['A', 'B']);
printf "P(A->C) = %.4f\n", $smooth->probability('A', 'C');  # 0.2500
```

### Order-2 character chain (text generation)

```perl
my $chain = CodingAdventures::MarkovChain->new(order => 2);
$chain->train_string("abcabcabc");

# Context keys for order-2 are two-character k-grams joined by "\0".
printf "P(ab->c) = %.1f\n", $chain->probability("a\0b", 'c');  # 1.0

print $chain->generate_string("ab", 9), "\n";  # abcabcabc
```

### Stationary distribution

```perl
my $chain = CodingAdventures::MarkovChain->new(smoothing => 1.0);
$chain->train(['A', 'B', 'A', 'C', 'A', 'B', 'B', 'A']);

my $pi = $chain->stationary_distribution;
for my $state (sort keys %$pi) {
    printf "%s: %.4f\n", $state, $pi->{$state};
}
# A: 0.5000
# B: 0.3333
# C: 0.1667  (approximately)
```

## API reference

| Method | Description |
|--------|-------------|
| `new(order => $k, smoothing => $α, states => \@list)` | Constructor. All args optional (defaults: order=1, smoothing=0.0, states=[]). |
| `train(\@sequence)` | Accumulate transition counts from sequence; renormalise. |
| `train_string($text)` | Character-level convenience wrapper around `train`. |
| `next_state($context)` | Sample one transition; dies on unknown context. |
| `generate($start, $length)` | Return arrayref of exactly `$length` states. |
| `generate_string($seed, $length)` | Return string of exactly `$length` characters. |
| `probability($from, $to)` | Trained probability T[$from][$to]; 0.0 if unseen. |
| `stationary_distribution()` | Power-iteration approximation of π; hashref. |
| `states()` | Sorted arrayref of all known atomic states. |
| `transition_matrix()` | Deep-copy of transition table as hashref-of-hashrefs. |

## Running tests

```bash
prove -l -v t/
```

## License

MIT
