# coding_adventures_hash_analysis

Statistical quality measures for hash functions (DT17 "Analysis helpers"), split out
of `coding_adventures_hash_functions` (WX-P1a2).

- `avalanche_score(hash_fn, output_bits, sample_size)`: the average fraction of output
  bits that flip when one input bit of a random 8-byte input is toggled. The ideal is
  `0.5`.
- `distribution_test(hash_fn, inputs, num_buckets)`: the chi-squared statistic of the
  bucket counts of `hash(input) % num_buckets`. `0` is a perfectly even spread.

Both take any closure `Fn(&[u8]) -> H` with `H: Into<u64>`, so they measure any hash.

## Why a separate crate

`avalanche_score` draws its random inputs from the operating system (`getrandom`), which
is the `rand:read:*` capability. The hash functions themselves are pure computation with
a Warrant document, a no-capability declaration and a jailed runtime gate. Keeping the
randomness here lets `coding_adventures_hash_functions` declare `"capabilities": []`
truthfully, and this crate declares `rand:read:*` once, in `required_capabilities.json`.
The sampling is for a quality statistic, never for keys or secrets.

## Usage

```rust
use coding_adventures_hash_analysis::{avalanche_score, distribution_test};
use coding_adventures_hash_functions::{fnv1a_32, murmur3_32};

let score = avalanche_score(murmur3_32, 32, 64);
assert!((0.0..=1.0).contains(&score));

let chi2 = distribution_test(
    fnv1a_32,
    vec![&b"alpha"[..], &b"beta"[..], &b"gamma"[..]],
    8,
);
assert!(chi2 >= 0.0);
```

## Building and Testing

```bash
cargo test -p coding_adventures_hash_analysis -- --nocapture
```

`BUILD` also runs the capability analyzer against `required_capabilities.json`. The
crate has no Warrant document: both helpers return `f64` and take a generic closure,
which Warrant does not cover (floats are VER04p, generics over closures are not
Warrant). See [DT17a](../../../specs/DT17a-hash-functions-warrant.md).
