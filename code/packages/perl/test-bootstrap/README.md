# CodingAdventures::TestBootstrap (Perl)

The Perl lane of the property-based testing engine (spec
[VER05p](../../../specs/VER05p-perl-pbt-engine.md), child of
[VER05](../../../specs/VER05-property-engine.md)). It generates random inputs from a
seeded SplitMix64 stream, runs a property against them, shrinks a failing input to a
minimal witness, and reports an honest verdict. It passes the shared `pbt-v1`
conformance corpus (25 cases) with exact outputs, minimal witnesses and shrink step
counts.

**Zero third-party dependencies**: core Perl only (`Scalar::Util`, `B`,
`Math::BigInt`, `Encode`, `Config`). Needs a perl with 64-bit integers; loading fails
with a clear message otherwise.

## Where it fits

```text
L2  property engine (this package, one lane of VER05)
L0  test-bootstrap harness (TF01): the same SplitMix64 and failure vocabulary
```

Every lane (Rust, Python, Go, Java, C#, Ruby, C, Elixir, Haskell, Lua, ...) draws the
same values from the same seed and shrinks to the same witness, which is what the
corpus pins.

## Usage

```perl
use CodingAdventures::TestBootstrap;
my $Gens = 'CodingAdventures::TestBootstrap::Gens';
my $Runner = 'CodingAdventures::TestBootstrap::Runner';

my $verdict = $Runner->can('check')->(
    42,                                    # seed (an integer, any size)
    $Gens->can('integer')->( 0, 1000 ),    # generator
    sub { return $_[0] <= 100 ? 1 : 0 },   # the property: exactly one holds
    CodingAdventures::TestBootstrap::Runner::Options->new(
        { num_tests => 100, ci => 'off' } ),
);

# $verdict->{failure_id}   'property-counterexample'
# $verdict->{minimal}      101
# $verdict->{shrink_steps} the accepted shrink moves
```

A property **holds only when it returns exactly one**: the integer `1`, the string `"1"` or an NV that is exactly `1.0` (`!!1` is all three). `1 + 2**-52`, `"1.0"`, `2` and references falsify.
`0`, `undef`, `""`, a reference, a list or no return value falsify, so a property that
forgets its `return` cannot pass. A property that dies (with any value, including an
object whose `""` overload dies) is a `property-panic`.

The CI flag is a parameter, never read from the environment:
`Options->new({ ci => Options->ci_from_value($ENV{CI}) })`. `ci => 'unset'` (the
default) is treated as CI and redacts the seed and the witness.

## What is different about Perl (VER05p section 3 to 5)

| Hazard | What the engine does |
|---|---|
| Integers silently overflow into a double | words are UVs; `+` and `*` are built from 32-bit halves; public integers are native IVs or `Math::BigInt`, normalized; floats and strings are refused as seeds, bounds and counts |
| Values run code (overloads, ties, DESTROY) | values are read through `B` (internal slots), never dereferenced or stringified; anything with magic is opaque |
| `die` can throw anything; `last` crosses frames | outcomes are classified by `eval` and `blessed`/`reftype`; `$@`, `$_` and `$SIG{__DIE__}`/`__WARN__` are localized; a non-local exit is a panic |
| Cycles, depth, size | Canon caps depth 16, 10,000 nodes, 1 MiB, enforced while traversing |
| Bytes vs characters | Canon and JSON work on bytes; the utf8 flag never decides meaning; UTF-8 is validated by the engine |
| Shrink budget (VER05e 4.1) | a tree of scopes charged all-or-nothing, carried by `local $Budget::CURRENT` (restored on `die`), with an optional explicit `parent`; a stale handle is spent, never a fresh root |

The budget is cooperative: in-process code can forge handles. It bounds honest nesting.

The global `Math::BigInt` settings (`upgrade`, `accuracy`, `round_mode`) must be at their defaults.

Not contained: `exit`, `exec`, `fork` and `kill` inside a property, and a labelled
`last OUTER` aimed at the caller's loop. The redacted `repro-fnv` is a non-secret
fingerprint, not a commitment.

## Modules

`Int`, `Prng`, `Limits`, `FailureIds`, `Hooks`, `Safe`, `Budget`, `Gen`, `Gens`,
`Shrink`, `Runner`, `Canon`, `StrictJson`, `Values`, `Tfv`, `Corpus`, all under
`CodingAdventures::TestBootstrap::`.

## Testing

```text
cpanm --installdeps --quiet .
prove -l -v t/
```

`t/08-corpus.t` replays `code/specs/fixtures/pbt-v1/cases.json` and runs a negative
control for every case and corruption. Coverage, on demand, with `Devel::Cover`
(not core, never a dependency): `cover -test`.
