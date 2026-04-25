# RNG — Random Number Generator

## Overview

This specification covers the random number generator (RNG) library for the
coding-adventures monorepo. It is divided into three tiers that represent the
present implementation and the forward roadmap:

| Tier | Name | Status | Description |
|------|------|--------|-------------|
| 1 | Algorithmic PRNGs | **Implemented here** | Pure deterministic generators seeded by the caller |
| 2 | OS Entropy | Roadmap | Tap kernel entropy sources (`/dev/urandom`, `BCryptGenRandom`, etc.) via a Rust crate + native extensions |
| 3 | Hardware Entropy | Roadmap | Mix in physical entropy (Cloudflare-style lava lamps, audio jitter, hardware RNG instructions) |

---

## Tier 1 — Algorithmic PRNGs

### Motivation

A *pseudorandom number generator* (PRNG) produces a long, statistically
uniform sequence of numbers from a compact seed. The sequence is completely
determined by the seed — same seed, same sequence — which is essential for
reproducible simulations, games, tests, and procedural generation.

The three algorithms below span 70 years of PRNG design and illustrate the
trade-offs between simplicity, speed, and statistical quality.

### Algorithms

#### LCG — Linear Congruential Generator (Knuth 1948)

The simplest useful PRNG. State advances via:

```
state′ = (state × a + c) mod 2^64
output = state′ >> 32      -- upper 32 bits
```

Constants (`a`, `c`) are the "Numerical Recipes" values:

```
a = 6 364 136 223 846 793 005
c = 1 442 695 040 888 963 407
```

These satisfy the **Hull-Dobell theorem** — the generator visits every one of
the 2^64 possible states exactly once per cycle (full period). The output uses
the *upper* 32 bits because the lower bits of an LCG have much shorter sub-
periods and are statistically poor.

**Strengths**: trivially fast; no branches; full period.  
**Weaknesses**: consecutive outputs are correlated; fails lattice tests; never
use for cryptography.

#### Xorshift64 (Marsaglia 2003)

Three XOR-shift operations scramble a 64-bit state — no multiplication:

```
x ^= x << 13
x ^= x >> 7
x ^= x << 17
output = x & 0xFFFF_FFFF    -- lower 32 bits
```

Period: 2^64 − 1. State 0 is a fixed point (every operation leaves it at 0),
so seed 0 is replaced with 1. The shift amounts (13, 7, 17) were found by
Marsaglia's exhaustive search to maximise the period and pass the DIEHARD
battery.

**Strengths**: no multiply; extremely fast on SIMD hardware; decent statistics.  
**Weaknesses**: fails some lattice tests; seed must be non-zero.

#### PCG32 — Permuted Congruential Generator (O'Neill 2014)

Uses the same LCG recurrence as above but applies an **XSH RR** (XOR-Shift
High / Random Rotate) output permutation before returning:

```
old  = state
state′ = (old × a + c) mod 2^64          -- advance LCG

xorshifted = ((old >> 18) ^ old) >> 27   -- mix high bits down
rot        = old >> 59                    -- 5-bit rotation amount
output     = rotr32(xorshifted, rot)      -- right-rotate 32-bit word
```

The rotate scatters every bit of state into the output uniformly. PCG32 passes
all known statistical test suites (TestU01 BigCrush, PractRand) while using
only 8 bytes of state. Seeded with the "initseq" warm-up to prevent poor
behaviour from seeds 0 and 1:

```
state = 0
state = state × a + (increment | 1)
state = state + seed
state = state × a + (increment | 1)
```

**Strengths**: best statistical quality of the three; tiny state; fast.  
**Weaknesses**: not cryptographically secure; not suited for secret key generation.

### API

All three generators expose an identical interface. Naming follows each
language's conventions (snake_case for Python/Ruby/Elixir/Rust/Go/Perl/Lua;
camelCase for TypeScript/Java/Kotlin/Swift/C#/Dart; `PascalCase` for Haskell
module-level functions).

| Operation | Returns | Description |
|-----------|---------|-------------|
| `new(seed)` | generator | Create generator from integer seed. Seed 0 valid for LCG/PCG32; replaced with 1 for Xorshift64. |
| `next_u32()` | uint32 | Advance state; return 32-bit unsigned integer in [0, 2^32). |
| `next_u64()` | uint64 | Two consecutive `next_u32` calls composed as `(hi << 32) \| lo`. |
| `next_float()` | float | `next_u32 / 2^32` — uniform in [0.0, 1.0). |
| `next_int_in_range(min, max)` | int | Rejection sampling; uniform in [min, max] inclusive. |

### Rejection Sampling

Naïve `value % range` over-samples low values when 2^32 is not divisible by
`range`. Example: range = 3, 2^32 = 4 294 967 296 = 3 × 1 431 655 765 + 1.
The remainder 1 means value 0 is sampled once more than values 1 and 2.

Rejection sampling eliminates the bias:

```
range     = max − min + 1
threshold = (−range) mod 2^32 mod range
loop:
    r = next_u32()
    if r >= threshold: return min + (r % range)
```

`threshold` is the count of "biased" values at the bottom of [0, 2^32). Any
draw below the threshold is discarded. The expected number of iterations is
< 2 for all ranges.

### Known Reference Values (seed = 1)

These values are the ground truth for cross-language test suites. All
implementations must produce exactly these outputs.

| Call | LCG | Xorshift64 | PCG32 |
|------|-----|------------|-------|
| 1st `next_u32` | 1 817 669 548 | 1 082 269 761 | 1 412 771 199 |
| 2nd `next_u32` | 2 187 888 307 | 201 397 313 | 1 791 099 446 |
| 3rd `next_u32` | 2 784 682 393 | 1 854 285 353 | 124 312 908 |

### Implementation Languages

Tier 1 is implemented as a library package in every language the monorepo
supports: Go, Rust, Python, TypeScript, Ruby, Elixir, Lua, Perl, Swift,
Haskell, Java, Kotlin, C#, and Dart.

Languages where 64-bit unsigned arithmetic requires explicit masking (Python,
Ruby, Elixir, Lua, Perl, Haskell) must apply `& 0xFFFFFFFFFFFFFFFF` after
every multiply/add to simulate C's `uint64_t` wraparound. TypeScript must use
`BigInt` for all state operations (JavaScript doubles only have 53-bit integer
precision).

---

## Tier 2 — OS Entropy (Roadmap)

The OS kernel maintains an entropy pool fed by hardware events (keystrokes,
disk seeks, network timing, CPU jitter). This pool is the foundation for
cryptographically secure random numbers on every major platform.

### Platform Sources

| OS | Source | Notes |
|----|--------|-------|
| Linux | `getrandom(2)` syscall | Blocks until enough entropy at boot |
| macOS / iOS | `getentropy(2)` | Never blocks; from Secure Enclave on Apple Silicon |
| Windows | `BCryptGenRandom` | Part of CNG (Cryptography Next Generation) |
| FreeBSD | `getrandom(2)` | Same interface as Linux |
| WASI / WASM | `random_get` | Part of WASI preview1 |

### Planned Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                 Language Native Extensions                    │
│  Python (cffi)  Ruby (FFI)  Node (N-API)  Java (JNI)  ...   │
└──────────────────────────┬──────────────────────────────────┘
                           │ C ABI
┌──────────────────────────▼──────────────────────────────────┐
│          rng-entropy-wrapper (Rust crate)                     │
│  Detects OS at compile time; delegates to platform crate     │
└─────┬──────────────┬──────────────┬──────────────┬──────────┘
      │              │              │              │
┌─────▼──────┐ ┌─────▼──────┐ ┌────▼──────┐ ┌────▼──────┐
│ rng-entropy│ │ rng-entropy│ │rng-entropy│ │rng-entropy│
│   -linux   │ │  -macos    │ │ -windows  │ │  -wasm    │
└────────────┘ └────────────┘ └───────────┘ └───────────┘
```

Each platform crate exposes a single function:

```rust
/// Fill `buf` with cryptographically secure random bytes from the OS.
pub fn fill_secure(buf: &mut [u8]) -> Result<(), EntropyError>;
```

The wrapper crate re-exports `fill_secure` and adds:

```rust
/// Return a cryptographically secure random u32.
pub fn secure_u32() -> u32;

/// Return a cryptographically secure random u64.
pub fn secure_u64() -> u64;

/// Return a cryptographically secure float in [0.0, 1.0).
pub fn secure_float() -> f64;

/// Return a cryptographically secure integer in [min, max].
pub fn secure_int_in_range(min: i64, max: i64) -> i64;
```

The native extensions for each language thin-wrap these four functions,
giving every language the same `SecureRng` class/module alongside the existing
algorithmic generators.

---

## Tier 3 — Hardware Entropy (Roadmap)

### Cloudflare's Lava Lamp Wall

Cloudflare feeds chaotic visual entropy from a wall of lava lamps into their
random number generator. A camera captures frames; each pixel's colour value
contributes to a hash that seeds their CSPRNG. Because the physical process
is unpredictable, even an attacker who controls the software cannot predict
the seed.

### Other Physical Sources

| Source | Mechanism |
|--------|-----------|
| Audio jitter | Capture microphone noise; hash raw samples |
| CPU timing jitter | Measure loop execution time variation (JENT algorithm) |
| RDRAND / RDSEED | x86 hardware RNG instructions feeding the OS entropy pool |
| TPM | Trusted Platform Module hardware RNG |
| Network timing | Hash inter-packet arrival jitter |

### Planned Integration

Hardware entropy sources act as additional inputs mixed into the OS entropy
pool (Tier 2). The Tier 2 `fill_secure` function therefore benefits
automatically when the kernel uses RDRAND/RDSEED or a TPM. Dedicated
application-level sources (lava lamp cameras, audio jitter) would be
implemented as separate Rust crates that produce entropy bytes fed into a
`ChaCha20` stream cipher used as a CSPRNG.

This remains future work — the current Tier 1 implementation provides the
purely algorithmic foundation that all tiers build upon.

---

## Testing Requirements

- **≥ 95% line coverage** for all library packages
- Every implementation must pass the known-reference-value tests (seed=1 outputs above)
- Tests must verify: determinism, different seeds diverge, float in [0,1), range bounds, single-value range, distribution uniformity (chi-square ±30%), and for Xorshift64: seed-0 replacement and state-never-zero

## Package Naming

| Language | Package name |
|----------|-------------|
| Go | `github.com/adhithyan15/coding-adventures/code/packages/go/rng` |
| Rust | `coding-adventures-rng` (crate) |
| Python | `coding-adventures-rng` (PyPI) |
| TypeScript | `@coding-adventures/rng` (npm) |
| Ruby | `coding_adventures_rng` (gem) |
| Elixir | `:coding_adventures_rng` (Hex) |
| Lua | `coding-adventures-rng` (LuaRocks) |
| Perl | `CodingAdventures::Rng` (CPAN) |
| Swift | `CodingAdventuresRng` (SPM) |
| Haskell | `coding-adventures-rng` (Hackage) |
| Java | `com.codingadventures.rng` (Maven) |
| Kotlin | `com.codingadventures.rng` (Maven) |
| C# | `CodingAdventures.Rng` (NuGet) |
| Dart | `coding_adventures_rng` (pub.dev) |
