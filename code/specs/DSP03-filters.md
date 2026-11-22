# DSP03 — FIR / IIR Filters

**Status**: V1 spec (this document is Phase 0).

**Scope**: a new `dsp-filters` crate providing finite (FIR) and
infinite (IIR) impulse-response filters in pure Rust, plus the
canonical filter design helpers (Butterworth / Chebyshev /
windowed-sinc).  Built on top of `dsp-fft` for FFT-accelerated
convolution and lifts onto the matrix execution layer through
the same path as DSP01 / DSP02.

## Why filters?

Filters are the workhorse primitive of every signal-processing
pipeline.  They show up everywhere:

- **Audio EQ, noise gates, anti-aliasing** — IIR biquads
  (low-pass / high-pass / band-pass) at audio sample rates.
- **Image blurring, sharpening, edge detection** — small FIR
  kernels (3×3, 5×5) convolved over `[H, W]` images.
- **Communications** — root-raised-cosine, matched filtering,
  windowing.
- **Sensor preprocessing** — moving-average, exponential
  smoothing, Savitzky-Golay polynomial fits.
- **OpenCV-equivalent**: `filter2D`, `sepFilter2D`, `GaussianBlur`
  all reduce to FIR/IIR convolution.
- **scipy.signal.lfilter** / **scipy.signal.firwin** / **scipy.signal.butter** — V1 will match these contracts.

After DSP01 (FFT) and DSP02 (DCT), filters are the natural next
primitive.  FIR convolution lifts to FFT for long signals
(`O(N log N)` vs the `O(NK)` direct form), so `dsp-filters`
plugs directly into `dsp-fft`'s matrix-ir path.

## Filter taxonomy

### FIR (Finite Impulse Response)

```text
    y[n] = Σ_{k=0..K-1}  h[k] · x[n - k]
```

- Always stable (no feedback).
- Linear phase (when the kernel is symmetric).
- Easy to design — windowed sinc gives any low-pass / high-pass
  / band-pass at any cutoff.
- Computational cost `O(K)` per output sample for direct
  convolution; `O(log N)` per sample via FFT-based overlap-add
  for long `K`.

### IIR (Infinite Impulse Response)

```text
    y[n] = ( Σ_{k=0..M}  b[k] · x[n - k]
           - Σ_{k=1..N}  a[k] · y[n - k] ) / a[0]
```

- Recursive: each output depends on past outputs.
- Much smaller order than FIR for the same magnitude response
  (a 4th-order Butterworth IIR matches a ~64-tap FIR).
- Can be unstable if poles are outside the unit circle.
- Phase response is non-linear (matters for some applications,
  not for others).
- Implemented as cascades of biquads (2nd-order sections) for
  numerical stability at higher orders.

V1 ships **direct-form-II Transposed** for IIR, which is the
numerically-stable canonical form scipy / MATLAB use.

## V1 scope

**Phase 1**: `dsp-filters` crate skeleton (Cargo.toml,
README.md, CHANGELOG.md, src/lib.rs stub).

**Phase 2**: scalar FIR via direct convolution.

```rust
pub fn fir(signal: &[f32], kernel: &[f32]) -> Result<Vec<f32>, FilterError>;
```

Handles boundary mode = `Full` (length `N + K - 1`, the natural
linear convolution).  Tests: known impulse / box / Gaussian
kernels, scipy cross-check, edge cases (empty signal, empty
kernel).

**Phase 3**: FIR via FFT (overlap-add) for long signals.

```rust
pub fn fir_fft(signal: &[f32], kernel: &[f32]) -> Result<Vec<f32>, FilterError>;
```

Reuses `dsp-fft::fft_via_runtime` so the convolution lifts onto
the matrix execution layer.  Switches between direct and FFT
based on a heuristic (kernel length crossover ~64).

**Phase 4**: scalar IIR direct-form-II Transposed.

```rust
pub fn iir(signal: &[f32], b: &[f32], a: &[f32]) -> Result<Vec<f32>, FilterError>;
```

Matches `scipy.signal.lfilter(b, a, x)` exactly.  Tests: known
biquads (low-pass, high-pass), step response, scipy cross-check.

**Phase 5**: filter design helpers — Butterworth, Chebyshev,
windowed-sinc.

```rust
pub fn design_low_pass(cutoff_norm: f32, num_taps: u32, window: WindowType)
    -> Vec<f32>; // legacy panic-on-invalid API
pub fn try_design_low_pass(cutoff_norm: f32, num_taps: u32, window: WindowType)
    -> Result<Vec<f32>, FilterError>;
pub fn design_high_pass(cutoff_norm: f32, num_taps: u32, window: WindowType)
    -> Vec<f32>; // legacy panic-on-invalid API
pub fn try_design_high_pass(cutoff_norm: f32, num_taps: u32, window: WindowType)
    -> Result<Vec<f32>, FilterError>;
pub fn design_band_pass(low_cutoff_norm: f32, high_cutoff_norm: f32, num_taps: u32, window: WindowType)
    -> Vec<f32>; // legacy panic-on-invalid API
pub fn try_design_band_pass(low_cutoff_norm: f32, high_cutoff_norm: f32, num_taps: u32, window: WindowType)
    -> Result<Vec<f32>, FilterError>;
pub fn butterworth_lowpass(order: u32, cutoff_norm: f32)
    -> (Vec<f32>, Vec<f32>);  // (b, a)
pub fn chebyshev1_lowpass(order: u32, ripple_db: f32, cutoff_norm: f32)
    -> (Vec<f32>, Vec<f32>);

pub enum WindowType { Rectangular, Hamming, Hann, Blackman }
```

The FIR designer contract is lane-specific. `cutoff_norm` is a
cycles-per-sample frequency, with Nyquist at `0.5`; it is not a
frequency in Hz and takes no sample-rate argument. Low-pass and
high-pass accept finite cutoffs in the inclusive range `[0.0, 0.5]`.
Band-pass accepts finite cutoffs satisfying
`0.0 <= low_cutoff_norm < high_cutoff_norm <= 0.5`. Rust uses `f32`,
and Python/TypeScript use their native numeric types; negative zero is
accepted wherever the comparisons admit it.

Rust and Python allow at most `2^20` taps (largest accepted odd count
`1,048,575`); TypeScript allows at most `2^16` taps (largest accepted
odd count `65,535`). All three require a positive odd tap count. The
Rust `u32` parameter cannot represent a negative count. TypeScript
validates integer tap counts at runtime. Python's contract is for callers
supplying an integer as annotated: its implementation checks positivity,
parity, and the cap but does not enforce the runtime type, so wrong-object
behavior (including `bool` or floating-point values) is outside this
contract.

Rust window selection is the typed `WindowType` enum with Rectangular,
Hamming, Hann, and Blackman. Python accepts that lane's enum or a
trimmed, case-insensitive string with one of those four names. TypeScript
accepts those four normalized names plus `hanning` as a Hann alias and
`kaiser` (using the `dsp-fft` default beta of 5); unknown window errors
are lane-specific. Python and TypeScript report invalid design inputs
with their existing typed exceptions. The additive Rust `try_design_*`
APIs report validation errors through the existing
`FilterError::InvalidCoefficient(String)` variant; no new public error
variant is introduced.

Rust validates low/high taps (positive odd, then cap) before cutoff
finiteness and range. Band-pass validates taps, then cap, cutoff
finiteness, then ordering/range. High-pass delegates to low-pass
validation. The legacy Rust `design_*` wrappers and new `try_design_*`
functions share each operation's checked calculation path. The legacy
wrappers preserve their observable panic payload type, message, check
order, and valid-input coefficient bits. Validation precedes
tap-proportional coefficient allocation; this is not a global
no-allocation or no-OOM guarantee.

The current low-pass normalizer uses `sum(h)` and divides only when its
absolute value is positive. Band-pass normalizes by the absolute
center-frequency response, `abs(sum_k h[k] * cos(2*pi*f0*x_k))`, where
`f0=(low_cutoff_norm+high_cutoff_norm)/2` and
`x_k=k-(num_taps-1)/2`; it likewise divides only when the gain is
positive. This work preserves those zero/cancellation branches and
non-finite normalization behavior. A shared cancellation/output policy
is tracked separately in AF-01c. This contract does not promise finite
outputs for every otherwise accepted input.

### Minimal Python/TypeScript conformance fixture (AF-02)

`code/specs/fixtures/biquad-sos-v1/cases.json` is a versioned shared fixture
for exactly two existing public operations: stateful `Biquad.process` and
one-shot `sosfilt`. It uses JSON numbers decoded as finite binary64 values,
exact numeric equality for finite outputs (positive and negative zero compare
equal), and exact output length. No epsilon or platform-dependent
transcendental golden is involved. Fixture validation is test-harness behavior
and does not change either package API.

The input is a 16-sample unit impulse. The `biquad.process` record uses the
single section `[0.5, 0.25, 0.5, 2, -2, 0.5]`; its impulse response is
`h[0]=1/4` and `h[n]=(6n-3)/2^(n+2)` for `1 <= n < 16`. The `sosfilt`
record uses that section followed by `[1, -1, 0, 1, 0, 0]`; its response is
`g[0]=1/4`, `g[1]=1/8`, and `g[n]=(15-6n)/2^(n+2)` for `2 <= n < 16`.
These values are exact dyadic rationals representable in binary64. The fixture
README records the independent derivation and limits.

The Biquad operation is checked on one whole input, chunk lengths `[3, 1,
12]`, and 16 single-sample calls. One instance retains state within each run;
equivalent fresh instances are used for comparisons. A separate reset check
processes a nonempty prefix, resets, and compares replay with a fresh instance.
The test domain includes every nonempty contiguous partition of the 16
samples: `2^15 = 32,768` partitions and 524,288 output samples per lane. SOS
is invoked as a one-shot operation with fresh state each time; it is not a
streaming contract. Empty sections with nonempty input are identity, and empty
input is rejected even when sections are empty.

The fixture has exactly two required unique operation records. Python and
TypeScript test loaders reject missing or duplicate operation IDs, extra or
missing fields, malformed section/sample shapes, booleans or non-numeric and
non-finite values, and wrong expected lengths. Negative controls alter one
expected dyadic sample and must fail as an output-value mismatch. Shared
fixture or loader changes select both package suites in CI. This narrow
fixture does not establish Rust parity, general audio/math/units schemas,
native floating-point proofs, or package-wide exhaustive assurance; those
remain separate backlog work.

For this version, the root has exactly `schema_version`, `input`,
`comparison`, and `cases`; the decoded finite numeric version must equal `1`
(booleans are rejected), and the comparison string is exactly
`finite_exact_numeric_equal_zero_sign_ignored`.
The input must equal the 16-sample unit impulse. The only valid ID/operation
pairs are `biquad.process.impulse16` / `biquad.process` and
`sosfilt.impulse16.two_sections` / `sosfilt`. The first record has exactly
one six-number section, 16 expected samples, and positive non-boolean numeric
`chunk_lengths` equal by decoded value to `[3, 1, 12]`; the second has exactly
two six-number sections and 16 expected samples. Every numeric slot must convert to a finite
binary64 value; booleans are not numbers in this fixture contract. Decoded
numeric values for `schema_version` and `chunk_lengths` use finite binary64
numeric equality to the required integers, independent of whether the JSON
token spells the value as `1`, `1.0`, or `1e0`. The Python loader converts
accepted chunk lengths to Python integers before slicing. Both language
loaders reject booleans, strings, nonintegral values, nonfinite values, and
structural values that do not equal the required small integer. General
numeric slots must be finite binary64 values; the Rust fixture consumer also
applies the declared f64-to-f32 exactness check. JSON objects with duplicate field
names are rejected by the Python loader.

The canonical fixture uses integer tokens for its version and chunk lengths.
Literal JSON controls also cover equivalent decimal and exponent spellings
(`1.0`, `1e0`, and `[3.0, 1e0, 12]`) and reject values such as `1.5`, `true`,
and strings for structural fields. This is decoded-value compatibility for
the fixed schema, not a promise that arbitrary JSON documents have identical
lexical handling across languages. Rust's AF-03 fixture adapter accepts an
integer or floating parser value only when its finite binary64 value equals
the required integer exactly.

#### AF-02c / Shared fixture integer decoding

AF-02c resolves the version-1 structural-number ambiguity before adding a
third consumer. Required integer-valued fields are checked by decoded finite
binary64 value, not by JSON token category; booleans and strings remain
invalid. Python canonicalizes accepted chunk lengths to integers before using
them as slice indices. Rust must preserve the same value semantics when its
strict JSON parser distinguishes integer and floating-number nodes.

The Rust test adapter uses the existing bounded JSON parser and does not
change that parser or the fixture. If a legal plain integer token exceeds the
parser's signed-64-bit integer representation, the adapter may retry after
changing only that complete numeric token to a decimal-exponent spelling with
the same binary64 parse result. The parser's reported byte offset is only a
candidate locator. The adapter independently verifies signed-64-bit overflow,
the complete `-?[1-9][0-9]*` token, and JSON value delimiters (BOF, `[`, `,`,
or `:` before it; EOF, `,`, `]`, or `}` after it), allowing only JSON whitespace
around delimiters. It then requires a finite binary64 parse before insertion
of `e0`. It must not rewrite fractions, existing exponents, strings,
malformed numbers, or values whose binary64 parse is non-finite. Reparse the
whole document with the same depth limit after each bounded repair, so
duplicate keys and all other syntax errors remain visible. At most 70 repairs
are permitted for this fixed fixture schema. Tests pin positive/negative
overflow offsets, including a UTF-8 prefix, and cover accepted wide integers
in the first and a later array slot (for example
`[0,18446744073709551616,0,1,0,0]`), plus fraction, exponent, quoted, malformed, overflow-to-infinity,
duplicate-key, and retry-limit controls. This test-only adaptation is not a
general JSON parser or production API and makes no offset claim for other
parser implementations.

### Checked Rust Biquad and SOS primitives (AF-03)

AF-03 adds public Rust-only checked primitives while leaving the existing
Python and TypeScript entry points and Rust `iir` function unchanged. The
intended crate-root API is `Biquad::new([f32; 6])`,
`Biquad::process_sample(f32)`, `Biquad::process(&[f32])`, `Biquad::reset()`,
and one-shot `sosfilt(&[[f32; 6]], &[f32])`, with fallible construction and
processing using the existing `FilterError` variants. Section slots are
`[b0,b1,b2,a0,a1,a2]`; normalized coefficients are computed by directly
dividing each of the five non-`a0` values by finite, nonzero `a0`. Reject
non-finite raw or normalized coefficients. Do not add epsilon thresholds or
pole-stability rejection: finite unstable filters are accepted until an
operation produces a non-finite result or state candidate.

For each sample, use Direct Form II Transposed:
`y=b0*x+z1`, `z1'=b1*x-a1*y+z2`, and `z2'=b2*x-a2*y`. Reject a non-finite
input, output, or either candidate delay using the existing invalid
coefficient error. A sample commits both delays only on success. A block
evaluates against local candidate state and commits only after every sample
succeeds; any returned `Err` leaves its entry state unchanged. `reset()` sets
both delays to positive zero. This failure-atomic statement applies to
returned errors; it does not promise recovery from allocation failure, panic,
or process termination and is not a no-panic or heap-bound proof.

Each multiply, add, and subtract is a separately rounded binary32 operation
in the written parenthesization; do not use fused multiply-add or reassociate
the recurrence. Pin this source-level policy with the control section
`[f32::MAX,0,0,1,1,0]`: processing `1` yields finite `f32::MAX` and delay
`-f32::MAX`; processing `2` next returns an error because the separately
rounded `f32::MAX * 2` overflows before adding the delay. A fused expression
could remain finite, so this case distinguishes the declared evaluation.

Empty signal returns `FilterError::EmptySignal`. Enforce the existing
`MAX_SAMPLES=2^24` samples per block and `MAX_ORDER=1024` as the maximum
number of SOS sections; do not introduce a product cap. Bounds errors use
`FilterError::InvalidCoefficient` with descriptive text; tests pin the error
variant and precedence, not fragile text.
For `sosfilt`, check empty signal, sample cap, and section cap in that order;
then validate all sections in order before evaluating samples. Construction
checks raw coefficients in slot order, rejects zero `a0`, then checks
normalized coefficients in `[b0,b1,b2,a1,a2]` order. Processing checks each
input sample and its output/delay candidates in input order. Check bounds
before coefficient- or sample-proportional work. SOS is one-shot, starts each
section with zero delays, processes sections in order, and returns a complete
finite vector or an error. `reset()` returns unit. Empty sections with nonempty finite input return an
identity copy; empty input is rejected even when sections are empty.

Rust `f32` checked behavior and errors are intentionally not claimed to match
Python/TypeScript binary64 behavior beyond the selected AF-02 fixture cases.
The fixture consumer must use the actual shared JSON file and public crate
exports, reject duplicate keys and malformed shapes, convert each numeric
slot to finite `f64` then finite `f32`, and require casting that `f32` back to
`f64` to equal the parsed value (positive and negative zero compare equal).
Golden values are never rounded to make them fit `f32`.

AF-03 acceptance separates the fixed 32,768-partition fixture run from
deterministic properties, general state/error properties, and fuzz trials.
The direct-form oracle family uses raw section
`[c0/4,c1/4,c2/4,2,0,0]`, integer coefficients `c0,c1,c2 ∈ [-4,4]`, samples
`x=k/16` with `k ∈ [-8,8]`, and lengths 1 through 32. For zero-padded integer
samples `k_n`, the independent oracle computes
`D_n=c0*k_n+c1*k_(n-1)+c2*k_(n-2)` and expects `D_n/128`; missing negative
indices are zero. The required nontrivial-feedback family uses the same
coefficient and input domains with raw section
`[c0/4,c1/4,c2/4,2,s1,s2/2]`, `s1,s2 ∈ {-1,+1}`, and lengths 1 through 12.
With `Y[-1]=Y[-2]=0`, it computes integer recurrence
`Y_n=2^n*D_n-s1*Y_(n-1)-s2*Y_(n-2)` and expects `Y_n/2^(7+n)`. Both oracles
use integer arithmetic for intermediates and convert only the exact dyadic
result to `f32`. Shrinking reduces integer coefficients and samples within
their declared ranges; feedback signs remain exactly `-1` or `+1`. Lengths
remain positive and within the family's upper bound, with inputs truncated
when length shrinks. The shrinker then recomputes derived floats and expected
values; it does not shrink floats independently. Both finite families are
exactly representable binary32 arithmetic, as checked by bounded independent rational
enumeration; that arithmetic derivation does not prove implementation source
or native floating-point behavior. VER05/VER06 harness results and fault
mutations are evidence for their stated bounded domains only. Native
floating-point/source bindings, exhaustive `f32` arithmetic, SAT/SMT proofs,
and package-wide verification remain release-blocking.

### Biquad/SOS failure and state-commit characterization (AF-02a)

AF-02a characterizes existing Python and TypeScript public exports without
changing them. The current public constructors reject raw non-finite
coefficients and zero `a0`, then normalize by multiplying each numerator and
feedback coefficient by `1/a0`. `Biquad.process` and `process_sample` reject
non-finite input samples but do not check whether normalized coefficients,
outputs, or updated delays became non-finite. `process` mutates the instance
one sample at a time, so a later bad sample can throw after earlier samples
have changed state. `sosfilt` builds and processes each section in order; a
non-finite final-stage output can be returned, while an intermediate
non-finite value is rejected when the next section validates its input.
These are named legacy observations, not promises that all finite input yields
finite output or that a failed call rolls state back.

The characterization uses the actual Python and TypeScript public exports
with binary64 inputs. Tests assert exception classes, finite/non-finite
categories, returned values where finite, and recovery behavior; they do not
pin NaN payloads or error messages. Let `m=2^-1074` be the least positive
binary64 subnormal (bits `0x0000000000000001`) and `H=2^1023` (bits
`0x7fe0000000000000`). Both are finite and exactly representable.

1. Construct `Biquad(m,0,0,m,0,0)` and process `[1]`. The legacy constructor
   accepts raw finite coefficients, `1/m` overflows to infinity, and the
   returned sample is non-finite. A finite control verifies direct
   `m/m = 1` and a normalized identity section returns `[1]`.
2. Construct `Biquad(2,0,0,1,0,0)` and process `[H]`. The output is
   non-finite from finite-input multiplication overflow.
3. Construct `Biquad(0,2,0,1,0,0)`. Processing `[H]` returns zero but leaves
   a non-finite delay; processing `[0]` next returns a non-finite value.
   Calling `reset()` before processing `[0]` returns zero.
4. Construct the pure two-sample delay `Biquad(0,0,1,1,0,0)`. Process
   `[1,2]` to warm it, then call `process([3,NaN,4])`. The public call raises
   the lane's `InvalidCoefficientError` when it reaches NaN, after committing
   the first sample and before processing the suffix. A subsequent `[0,0]`
   returns `[2,3]`. Reset followed by replaying `[1,2,3]` returns `[0,0,1]`.
5. For SOS, a gain-two section `[2,0,0,1,0,0]` as the final section returns
   a non-finite output on `[H]`; identity then gain-two also returns a
   non-finite output. Gain-two then identity rejects its non-finite
   intermediate at the next section's input check.

The five recipes are bounded legacy observations; they do not prove every
finite binary64 case or establish a generalized IEEE-754 claim. Do not alter
the AF-02 finite-binary64 fixture schema to accommodate these diagnostics.

AF-02a sets an opt-in checked-API policy for later Rust primitives. Checked
construction divides each of the five non-`a0` coefficients directly by
finite, nonzero `a0`, then rejects any non-finite normalized coefficient.
Checked processing accepts finite inputs, computes output and candidate delays
in local state, and rejects if output or either candidate delay is
non-finite. A failed sample or block leaves the instance at its entry state;
a block's state commits only after the entire block succeeds. `reset()` sets
all delays to zero. Checked SOS returns a complete finite output or an error
and rejects a non-finite intermediate before feeding the next section. Finite
signed zeros and subnormals are allowed except for zero `a0`; there is no pole
stability screen. This opt-in contract does not alter legacy Python or
TypeScript entry points. AF-02b tracks potential additive checked
Python/TypeScript APIs. AF-03 implements this checked policy in Rust and must
not claim general waveform, pole-stability, or floating-point parity beyond
the explicitly tested cases.

### Additive checked Python and TypeScript APIs (AF-02b)

AF-02b adds opt-in checked APIs without changing the existing `Biquad` or
`sosfilt` entry points, their coercion rules, rounding, partial-progress
behavior on errors, or legacy error classes. Python exports `CheckedBiquad`
with the six positional coefficients `[b0,b1,b2,a0,a1,a2]`,
`process_sample(x) -> float`, `process(signal) -> list[float]`, and
`reset() -> None`, plus `checked_sosfilt(sections, signal) -> list[float]`.
TypeScript exports `CheckedBiquad` with the same coefficient order,
`processSample(x) -> number`, `process(signal) -> Float64Array`, and
`reset() -> void`, plus `checkedSosfilt(sections, signal) -> Float64Array`.
Constructors and methods are synchronous; state atomicity is scoped to
non-reentrant calls. It does not cover side effects performed by caller
accessors, allocation failure, process termination, heap bounds, or real-time
behavior.

The checked kernel is separate from legacy code. It rejects a zero or
non-finite `a0`, directly divides the other five coefficients by `a0`, and
rejects any non-finite raw or normalized coefficient. Invalid primitive
values, coefficients, normalized results, sample results, delay candidates,
row shapes, and bounds raise the lane's existing
`InvalidCoefficientError`; empty input raises `EmptySignalError`. Exceptions
raised by caller-owned length or indexed-access methods propagate unchanged.
For each sample it
computes the DFII-T output and both candidate delays using separately rounded
binary64 operations in the written order. A non-finite input, output, or
candidate delay fails with the lane's existing filter error. The filter commits
both delays only after a sample succeeds; block processing evaluates using
private candidate state and commits only when the whole block and output are
complete. Any returned error or propagated input-accessor exception leaves
filter-owned entry state unchanged. `reset()` sets both delays to positive
zero. SOS constructs and validates all sections before processing, starts each
section with zero state, and returns a complete finite output or an error.
There is no pole-stability screen: finite unstable filters are accepted until
an operation violates a finite-value check.

The checked APIs accept finite primitive binary64 values, including signed
zeros and subnormals, except that either signed zero is invalid as `a0`.
Python accepts only exact built-in `int` and `float` values, excluding booleans,
subclasses, and conversion hooks. Each integer is converted once to binary64;
conversion overflow is reported as the existing coefficient error, while
ordinary binary64 rounding is part of the API (`2^53+1` becomes `2^53`).
TypeScript accepts only primitive finite `number` values; it does not coerce
strings, boxed numbers, booleans, `bigint`, or objects through `Number` or
`valueOf`. Coefficient and signal arrays are read by a single captured length
and bounded indexed access, not by an unbounded user iterator. Lengths must be
safe nonnegative integers and within the existing per-call limit before
allocation or traversal. Python invokes the indexed sequence's `__len__()` once
and requires its raw result to be an exact built-in nonnegative `int`; it does
not route the result through `len()` or `__index__`. Boolean, subclass, other
integer-like, negative, and over-cap results are rejected. The ordinary
descriptor call supports built-in and custom `Sequence` implementations, and
exceptions raised by that accessor propagate unchanged. For TypeScript, every
index of an actual `Array` must be an own element, so inherited numeric
properties cannot fill a sparse slot; other indexed array-like objects retain
their indexed accessor behavior. Holes, invalid elements, and accessor
exceptions fail without committing state; caller-defined accessor side effects
are outside the guarantee. Normalized coefficients are kept in private
immutable storage and filter delays are private state; the guarantee does not
cover deliberate mutation of Python underscore-prefixed internals.

The checked APIs retain the existing `2^24` sample-per-call and `1024` SOS
section limits; there is no product cap. A block reads signal length once,
then rejects empty input or an over-cap signal before reading any samples.
SOS follows the same first signal-length/empty/cap checks before it reads the
sections length once and validates its nonnegative safe bound and section cap.
It then reads each row length once, rejects anything other than six slots
before touching that row's coefficient getters, and validates all rows in
order before reading signal samples. A block reads samples in index order and
stops at the first invalid value or propagated accessor exception; entry state
remains unchanged. For a Biquad constructor, inspect raw coefficients in slot
order, reject zero `a0`, then inspect normalized values in
`[b0,b1,b2,a1,a2]` order. Empty SOS with a nonempty finite signal returns a
validated copy, preserving negative zero. Bounds and coefficient checks
precede sample traversal and output allocation where applicable. Tests pin
error class and precedence without relying on incidental message text,
including empty/over-cap signals paired with poison section accessors and
five- or seven-slot rows paired with poison coefficient getters.

The shared AF-02 fixture and all 32,768 partitions remain selected conformance
cases; legacy AF-02a counterexamples remain unchanged. Direct FIR and feedback
integer-oracle properties each run 512 cases with seeds `0xAF02B001` and
`0xAF02B002`; shrink proposals preserve coefficient/sample ranges, positive
length, and `±1` feedback signs. A one-case result-fault control per family
checks that the constrained shrinker preserves a failing valid case and that
the same case passes the healthy operation. Separate raw binary64 state cases
run 128 words at seed `0xAF02B003`; that sample was 128 finite words. A fixed
bit control set separately exercises five finite values (`±0`, minimum
subnormal, minimum normal, maximum finite) and three non-finite values
(`±Inf`, NaN).

Checked tests separately
cover minimum-subnormal identity, normalized coefficient/output/delay
overflow, rollback after late invalid samples and accessor errors, SOS order,
caps with poison accessors, and the separately rounded `f64::MAX` feedback
boundary. They also accept exactly `MAX_SAMPLES` samples and `MAX_ORDER`
sections, while testing each `+1` rejection without allocating an
over-cap-sized buffer. Python seeded mutation fuzz
must raise on semantic violations because that runner ignores returned values;
record exceptions/crashes actually reported by the harness, not inferred
timeout/OOM or native coverage. The healthy Python run uses 256 iterations at
seed `0xAF02BF01`, with corpus size 23 and no crashes; same-seed replay matches
those fields. Its 64-iteration result-fault run at `0xAF02BF02` recorded 28
crash entries and 23 unique exception fingerprints. Python has no producer
minimizer, so the test applies an external fixed-width byte-zeroing reducer to
one recorded crash; it reaches the all-zero word, which still fails the faulty
operation and passes the healthy one. This is a bounded test helper, not a
general producer feature. TypeScript semantic-guided fuzz must report
semantic violations as crashes, exercise edge admission beyond initial
triage, and retain actual outcome/replay counters only. Its healthy 1,024
iteration run at `0xAF02BF01` reported 196 admitted seeds, 198 manually recorded
outcome edges, and zero crashes; same-seed replay matches. The same-target
result-fault control is caught during initial triage, then the engine reducer
shrinks the `1.0` witness to the all-zero eight-byte word; it crashes under the
faulty callback and passes under the healthy callback. Both campaigns are
bounded tests; they do not establish general cross-language bit/error parity,
native float/source bindings, Warrant/SAT
proof, a coverage-guided Python campaign, or package-wide verification. Future
producer accounting/feedback work is tracked by AF-04c, and generic Python
optional-development-dependency selection by AF-04d.

The Butterworth and Chebyshev signatures above are existing separate
IIR helpers; their cutoff parameter is also normalized in the current
implementation. AF-01b does not change their behavior or specify their
full validation policy.

**Phase 6**: matrix-ir-lowered FIR.

`build_fir_graph_with_input(signal, kernel) -> (Graph, TensorId)`
emits a matrix-ir Graph computing the FIR via either direct
convolution (small kernel) or FFT (long kernel via
`dsp-fft::build_fft_graph_with_input` × 2).  Lifts to GPU once
backends claim the relevant ops.

**Out of V1 scope**:

- 2-D image filters (`filter2D`, `sepFilter2D`).  Lives in DSP04
  per the DSP overview.
- Adaptive filters (LMS, RLS).  Specialised; defer.
- Wavelet filters.  Lives in DSP06.
- Polyphase resampling, Cascaded Integrator-Comb (CIC) filters.
  Audio-specific; defer.
- Fixed-point / integer kernels.  MX05 territory.

## Public API

Lives in a new crate **`dsp-filters`** depending on `dsp-fft`.

```rust
/// Finite-impulse-response filter via direct convolution.
///
/// `signal` is a length-N real signal; `kernel` is a length-K
/// real impulse response.  Returns a length-(N + K - 1) Vec
/// holding the linear convolution (mode = "full").
pub fn fir(signal: &[f32], kernel: &[f32])
    -> Result<Vec<f32>, FilterError>;

/// FIR via FFT-based overlap-add.  For long kernels (K > ~64)
/// this is asymptotically faster than direct convolution.
/// Reuses `dsp-fft::fft_via_runtime` so the convolution lifts
/// onto the matrix execution layer.
pub fn fir_fft(signal: &[f32], kernel: &[f32])
    -> Result<Vec<f32>, FilterError>;

/// Infinite-impulse-response filter via direct-form-II
/// Transposed.  `b` is the feed-forward (numerator) polynomial,
/// `a` is the feedback (denominator) polynomial.  Matches
/// `scipy.signal.lfilter(b, a, x)` exactly.
///
/// Both `b` and `a` are length ≥ 1.  `a[0]` must be non-zero.
pub fn iir(signal: &[f32], b: &[f32], a: &[f32])
    -> Result<Vec<f32>, FilterError>;

#[derive(Debug)]
pub enum FilterError {
    EmptySignal,
    EmptyKernel,
    InvalidCoefficient(String),  // a[0] == 0, NaN, etc.
    Fft(String),                  // wraps dsp_fft::FftError
}
```

## Numerical accuracy contract

Per the DSP roadmap:

- FIR direct vs FIR FFT round-trip within `1e-4` relative
  tolerance for `N ≤ 64K`, f32 dtype.
- IIR vs scipy reference output within `1e-4` for stable filters
  (poles strictly inside the unit circle).
- Closed-form impulse responses match analytic forms to ULP.

The reference implementation in Phase 2 (`fir`) and Phase 4
(`iir`) is the oracle for all later phases.

### Filter-designer normalization compatibility

For the Python and TypeScript FIR design helpers, let `d` be the
normalization denominator: the coefficient sum for low-pass, and the
absolute center-frequency response gain for band-pass,
`abs(sum_k h[k] * cos(2*pi*f0*x_k))`, where `x_k=k-centre`,
`centre=(num_taps-1)/2`, and `f0` is the midpoint of the two band edges.
Preserve the existing result
`h * (1 / d)` bit-for-bit whenever `1 / d` is finite. When `1 / d` is
non-finite and `d` itself is finite and nonzero, use `h / d` so a finite
normalized coefficient is not lost to reciprocal overflow. Keep the
existing behavior when `d` is zero, NaN, or infinite. High-pass and the
`firwin` convenience entry points inherit the behavior of the design
helper they call (`firwin` exists in the TypeScript package).

This compatibility rule applies to the Python and TypeScript lanes. Rust
already divides directly and is not changed by this rule. It defines a
numerical compatibility target for these helpers; it does not establish
native floating-point or Warrant proof evidence.

## Testing strategy

**Phase 2 (scalar FIR direct)**:

- Error paths: empty signal, empty kernel.
- Closed-form: convolution with `[1.0]` (identity), `[0, 1, 0]`
  (delay by 1), uniform kernel of length K (moving average).
- Known DSP textbook outputs: 5-tap Hamming low-pass on a 64-sample
  sinusoid; impulse response of a windowed-sinc filter.
- Cross-check against a naive O(N · K) reference (which is
  itself the implementation, so this is more of a smoke test).

**Phase 3 (FIR via FFT)**:

- `fir_fft(signal, kernel)` matches `fir(signal, kernel)` within
  `1e-4` tolerance for varying N, K.
- Edge cases: K = 1, N = K, K > N.

**Phase 4 (IIR direct-form-II)**:

- Error paths: empty signal, empty / `a[0] = 0` coefficient
  vectors.
- Closed-form: `b = [1.0], a = [1.0]` (identity).  `b = [1.0],
  a = [1.0, -0.9]` (single-pole low-pass).
- Step response of canonical biquads (low-pass, high-pass).
- Cross-check against scipy's `lfilter` output for several
  filter designs (saved as golden test vectors).

**Phase 5 (design helpers)**:

- `design_low_pass(0.25, 33, Hamming)` matches scipy's
  `firwin(33, 0.25, window='hamming')` to ULP.
- `butterworth_lowpass(2, 0.1)` matches scipy's
  `butter(2, 0.1)` coefficients.
- The composed `iir(x, *butterworth_lowpass(2, 0.1))`
  matches scipy's full filter pipeline.

**Phase 6 (matrix-ir FIR)**:

- `fir_via_runtime` matches `fir` within `1e-4` for several
  N, K combinations.
- Round-trip via the matrix execution layer.

## Phase plan

| Phase  | Lands                                                     | Risk |
| ------ | --------------------------------------------------------- | ---- |
| 0      | Spec (this document)                                      | Low. |
| 1      | `dsp-filters` crate skeleton                              | Low. |
| 2      | Scalar FIR via direct convolution + tests                 | Low. |
| 3      | FIR via FFT (overlap-add) using `dsp-fft`                 | Medium — first FFT integration in DSP03. |
| 4      | Scalar IIR direct-form-II Transposed + tests              | Medium — recursion + numerical stability. |
| 5      | Filter design helpers (Butterworth, Chebyshev, windowed-sinc) | Medium-high — bilinear transform, classical filter math. |
| 6      | Matrix-ir-lowered FIR                                     | Medium — reuses dsp-fft graph builders. |

Phases 1+2 are typically bundled into one PR.  Phase 5 may
itself split into 5a (windowed-sinc), 5b (Butterworth), 5c
(Chebyshev) since each design family is independent.

## Dependencies

- `dsp-fft` — for `fft_scalar` (Phase 3 overlap-add) and
  `fft_via_runtime` / `build_fft_graph_with_input` (Phase 6).
- `dsp-complex` — for the FFT spectra in Phase 3.
- `matrix-ir`, `matrix-runtime`, `matrix-cpu`, `compute-ir`,
  `executor-protocol` — for Phase 6 (same set as `dsp-fft`
  already pulls in).

No FFI, no `unsafe`, no external crates beyond the DSP / matrix
layers we control.

## Open questions

1. **Overlap-save vs overlap-add for Phase 3.**  Both are
   standard; overlap-save is slightly faster but overlap-add
   is easier to verify against the direct path.  Defaulting
   to overlap-add for V1.
2. **Cascade biquad vs direct higher-order IIR.**  Higher-order
   direct forms accumulate noise quickly.  Phase 4 ships direct
   form for the API contract. The current Phase 5 Butterworth and
   Chebyshev-I helpers support orders 1 and 2; higher-order
   factorization into biquad cascades and stacked `iir(...)` calls
   remains future work.
3. **Group delay compensation.**  FIR with linear phase has
   constant group delay = `(K - 1) / 2`.  The Phase 2 API
   returns the full convolution and lets the caller trim;
   later phases may add an `fir_centered` variant.
4. **Streaming filter state.**  The Phase 4 `iir(signal, b, a)`
   API processes a whole signal at once.  A future
   `IirState::new(b, a)` + `state.feed(&samples)` would let
   callers stream samples one block at a time.  Defer until
   a real consumer needs it.
