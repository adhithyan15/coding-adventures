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

## Coverage-guided fuzzing (VER06q)

The same package carries the Perl lane of the Level 2 coverage-guided fuzz engine (spec
[VER06q](../../../specs/VER06q-perl-fuzz-engine.md), child of
[VER06](../../../specs/VER06-coverage-guided-fuzz-engine.md)), under
`CodingAdventures::TestBootstrap::Fuzz`. It passes the shared `fuzz-v1` corpus (25 cases) and
replays the Go lane's 820 reference vectors, and its mutators, power schedule, `sig_hash`,
minimiser and TAP line match the Ruby, Lua and Haskell lanes (pinned from the Ruby lane).

```perl
use CodingAdventures::TestBootstrap;
my $Fuzz = 'CodingAdventures::TestBootstrap::Fuzz';

my $options = CodingAdventures::TestBootstrap::Fuzz::Options->new(
    { max_iterations => 500, seed => 7, ci => 'off', timeout_ms => 1000 } );
my $target = sub {
    my ( $data, $h ) = @_;                       # a FRESH handle for every execution
    $h->edge( $h->edge_id('entered') );          # coverage: edges the target reports
    $h->tick;                                    # cooperative step budget
    $h->reserve( length $data );                 # cooperative memory budget
    die "boom\n" if $data eq "\xff\xff";
};
my $report  = CodingAdventures::TestBootstrap::Fuzz::run( ["\0\0"], $options, $target );
my $verdict = CodingAdventures::TestBootstrap::Fuzz::verdict_for( $target, $report, $options, 'my/target' );
print "$_\n" for @{ $verdict->{lines} };   # not ok 1 - my/target [fuzz-crash:3a8f19b2]
```

* **Coverage** is what the target reports through the handle (`$h->edge($id)`). The debugger
  hooks (`Devel::*`, `$^P`, `DB::DB`) are deliberately not used: 10x to 50x slower, one global
  slot shared with the coverage gate, and a target can silently drop them (spec section 4.2).
* **Isolation** is one `Safe::call` per execution under a one-shot `Time::HiRes::alarm`.
  Perl delivers signals between ops, so a **single long op (a huge `x`, a pathological regex)
  is not interruptible**, and a target that catches the alarm's exception in an unbounded loop
  cannot be stopped in-process; the CI job timeout is the backstop (an out-of-process runner is
  tracked as PT-F14). The step budget (`tick`) and the memory budget (`reserve`) are
  cooperative; there is no RSS poll (spec section 5.5).
* **Handles** are inert once their execution ends: `edge` does nothing and `tick` / `reserve`
  die with a retired sentinel, compared by `refaddr` and never by message. Exhaustion is sticky
  in both directions and judged after retirement, so a target that swallowed the sentinel is
  still a timeout or an oom.
* **Tampering** (replacing or cancelling the alarm, planting a sub in the engine) is detected
  after the run, never a pass, and capped at four leaked executions in every path (triage,
  mutation, minimisation, re-verification). A forked child that returns into the engine exits
  at once. `exit`, `exec` and `kill` in a target are not contained.
* **Trust model:** cooperative. Targets are trusted not to attack the harness.
* **Caps:** retained bytes 256 MiB, inputs 1 MiB, a campaign-wide minimisation budget that
  counts rejected candidates, an aggregate `mutate_sequence` work cap of 2^24 in the corpus
  runner; no bignum is ever built from input bytes.
* **Reports** are honest: zero executions is never a pass; in CI (or unset) mode the verdict
  carries no seed and no witness; `seed`, `corpus` and each crash's `minimal_input` are behind
  accessors and are never in a printed form.

Fuzz modules: `Fuzz`, `Fuzz::Limits`, `FailureIds`, `Mutate`, `Bitmap`, `Signature`, `Outcome`,
`Options`, `Report`, `Handle`, `Executor`, `Engine`, `Hooks`, `Corpus`.

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
control for every case and corruption; `t/12-fuzz-corpus.t` does the same for
`fuzz-v1`, and `t/11-fuzz-mutate.t` replays the Go lane's 820 reference vectors, which it parses
from the committed Go source.

Line coverage of the Fuzz tree needs no `Devel::Cover`: `t/lib/LineCov.pm` is a small
debugger-hook recorder (a test tool; the engine never uses the debugger hooks).

```text
for t in t/*.t; do
  PERL5DB='BEGIN { require "./t/lib/LineCov.pm" }' LINECOV_OUT=/tmp/lc.out perl -d -Ilib $t
done
perl -It/lib -MLineCov -e 'LineCov::report("/tmp/lc.out")'
```

(perl attributes the single statement of an `if`/`elsif` body that calls a sub to the line of
its condition, so the recorder undercounts those lines: the figure is a lower bound.) Coverage, on demand, with `Devel::Cover`
(not core, never a dependency): `cover -test`.
