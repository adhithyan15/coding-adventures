# VIS04 — Pattern Scanning

## Status

Fourth and last package in `VIS00-vision-roadmap.md`'s L2 build order.
New crate (`vis-pattern-scanning`) — no existing crate is a natural
home for a run-length ratio scanner, and it isn't specific to
`barcode-2d`'s `ModuleGrid` (a scanner runs over a photographed
*bitmap*, before a `ModuleGrid` exists).

## 1. Why this exists

`VIS00` scoped this precisely: "the general form of 'scan a row of
pixels for a 1:1:3:1:1 ratio of alternating dark/light runs,'
generalized so it isn't QR-specific. Every 2D barcode format locates
itself in an image the same way: a distinctive run-length signature,
scanned across rows and columns, cross-checked between the two, then
clustered into candidate centers."

Concretely: `VIS02` (connected components, merged) turns "a blob of
dark pixels" into a measurable shape, and `VIS03` (geometric
estimation, merged) turns point correspondences into a transform — but
nothing yet turns "a photographed, binarized image" into "here are
the point correspondences." That first step — finding *candidate*
finder-pattern locations at all, before anything can be measured or
transformed — is what's missing, and it's what `VIS04` closes.

## 2. Scope

A general **run-length ratio scanner** over a binary bitmap (`true` =
foreground/dark, matching `VIS02`'s established convention): given a
ratio sequence like QR's `[1, 1, 3, 1, 1]`, find positions where a
horizontal scanline *and* a vertical scanline through the same point
both produce runs in that ratio, within a tolerance. Not specific to
QR — Aztec, Data Matrix, and PDF417 each use their own ratio sequence
over the same scan-and-cross-check skeleton (their format-specific
scanning is `L3` work, once it exists, not this crate's).

Explicitly out of scope, both deferred to whatever assembles `L3`:

- **Clustering** multiple nearby confirmed candidates (typically one
  per scanline that crosses a real finder pattern) into a single
  refined center. `VIS00`'s own roadmap names `VIS02`'s connected-
  component labeling as the tool for this — stamp each confirmed
  candidate onto a small region of a fresh bitmap, run
  `label_components` over it, and each merged blob's centroid is a
  refined finder-pattern center. Building that assembly here would
  make this crate depend on `VIS02` for a step that's really an `L3`
  concern, not a scanning concern.
- Deciding what a QR finder pattern's ratio actually is, or any other
  format's. `[1, 1, 3, 1, 1]` is a caller-supplied `&[u32]`, not a
  constant this crate hardcodes.
- Diagonal cross-checks (some real-world scanners add a third scan
  along both diagonals for extra confidence beyond horizontal +
  vertical). Two-way cross-check is the documented, sufficient
  baseline; a third check is a caller-side refinement if false
  positives turn out to matter once this runs on real photos.

## 3. Algorithm

### 3.1 Run-length ratio matching over one line

Given a 1D slice of `bool` (one row or one column) and a ratio
sequence, decompose the line into its alternating runs, then slide a
window of `ratio.len()` consecutive runs across them:

```text
line:   [F F T T T F F F T F]     (T = foreground/dark)
runs:    2F   3T   3F  1T 1F      (length, alternating starting color)
```

A window matches when:

1. Its first run is foreground (`true`) — the scan looks for a dark
   pattern on a light background, matching `VIS02`'s own foreground
   convention; a window starting on a light run is skipped. (Runs
   always alternate color by construction, so this alone fixes every
   other run's color in the window too.)
2. Letting `unit = (sum of the window's run lengths) / (sum of
   ratio)`, every run's length is within `tolerance` (a relative
   fraction) of `ratio[i] * unit`:
   `|run[i].len - ratio[i] * unit| <= tolerance * ratio[i] * unit`.

`unit` is a per-window estimate of one ratio-sequence unit's pixel
size — for QR's `[1, 1, 3, 1, 1]` crossing a finder pattern, `unit`
approximates one QR module's width in the photographed image, the
same quantity `VIS03`'s homography estimation ultimately needs to
relate back to the clean module grid.

A match's **center** is the midpoint of the window's middle run
(`ratio[ratio.len() / 2]`, so an odd-length ratio — every real 2D-
barcode finder pattern uses one — has a single well-defined middle
run); for an even-length ratio the center is the mean of the two
middle runs' midpoints, so the function stays total (defined for every
ratio length) rather than picking one arbitrarily or panicking.

### 3.2 Two-way cross-check

A horizontal-only match is a weak signal — plenty of unrelated dark
regions produce *some* row with a matching ratio by coincidence. The
standard fix (used by every real 2D-barcode scanner) is to confirm
each horizontal candidate with an independent vertical scan through
the same point:

```text
for each row, horizontal-scan it for ratio matches
for each horizontal match (at column c, estimated row r):
    vertical-scan the column nearest c
    among that column's matches, take the one whose center is
    closest to r
    if that closest vertical match's center is within one estimated
    module size of r:
        confirmed candidate at (vertical match's center, c),
        module_size = average of the two estimates
    else:
        discard — the horizontal match was a coincidence
```

The confirmed row comes from the *vertical* scan (a more precise
measurement of vertical position than the horizontal scan, which only
knows what row it was run on), while the column comes from the
horizontal scan, symmetrically. This is the same idea `VIS01`/`VIS03`
already lean on repeatedly in this series: don't invent a new
technique when the textbook one — here, zxing's and every other real
QR-locating library's actual approach — is well understood and
sufficient.

## 4. API

```rust
/// One confirmed 2D location where both a horizontal and a vertical
/// scan found the requested run-length ratio.
pub struct PatternCandidate {
    pub row: f64,
    pub col: f64,
    /// Average of the horizontal and vertical scans' estimated
    /// pixel size for one ratio unit (for QR, approximately one
    /// module's width in the photographed image).
    pub module_size: f64,
}

/// One ratio match found along a single scanned line, in that line's
/// own coordinate (a column position for a horizontal scan, a row
/// position for a vertical scan — the caller knows which).
pub struct RatioMatch {
    pub center: f64,
    pub module_size: f64,
}

/// The largest relative `tolerance` `scan_line` accepts (§4.2).
pub const MAX_TOLERANCE: f64 = 10.0;

/// Scan one line (a single row or column, already extracted by the
/// caller) for windows matching `ratio` within `tolerance` (a
/// relative fraction, e.g. `0.5` for QR's usual ±50%). Never panics:
/// an empty `line`, an empty `ratio`, or a `tolerance` outside
/// `(0.0, MAX_TOLERANCE]` (covering non-positive, NaN, infinite, and
/// unreasonably large values alike) all simply yield no matches, not
/// an error.
pub fn scan_line(line: &[bool], ratio: &[u32], tolerance: f64) -> Vec<RatioMatch>;

/// Scan every row and column of `bitmap` for `ratio`, cross-checking
/// each horizontal match against a vertical scan through the same
/// point (§3.2). Returns one `PatternCandidate` per *confirmed*
/// match — deliberately not deduplicated or clustered (§2); several
/// adjacent scanlines crossing the same real pattern typically
/// produce several nearby candidates. Never panics, including on an
/// empty or ragged (`Vec<Vec<bool>>` with rows of differing length)
/// bitmap — matching `VIS02`'s established convention for this input
/// shape.
pub fn find_pattern_candidates(
    bitmap: &[Vec<bool>],
    ratio: &[u32],
    tolerance: f64,
) -> Vec<PatternCandidate>;
```

### 4.1 Ragged input

Column scans over a ragged bitmap only include rows that actually
reach that column (`col < row.len()`); a row too short to reach a
given column is skipped for that column's scan, never indexed
out of bounds. This is a documented degenerate-input behavior, not a
claim that ragged input produces a meaningful answer — matching the
precedent `VIS02` §4.2 already set for exactly this input shape.

### 4.2 Why `tolerance` has an upper bound

`unit` (§3.1) scales with the window's own run lengths, so an
unbounded `tolerance` isn't just "generous" — a caller-supplied value
like `f64::INFINITY` makes `tolerance * expected` infinite too, and
every foreground-starting window in the line then satisfies the
`<=` check regardless of its actual proportions. That turns a scan
that should reject almost everything into one that accepts almost
everything, and since `find_pattern_candidates` does an independent
column scan for *every* horizontal match, the cost stops being
bounded by the bitmap's own size and instead grows with how
permissive a caller's `tolerance` happens to be — a caller-controlled
algorithmic-complexity footgun, not merely a correctness one.
`MAX_TOLERANCE` (`10.0`, already several times more permissive than
any real use needs) closes this: `scan_line` rejects any `tolerance`
outside `(0.0, MAX_TOLERANCE]` up front, the same way it already
rejects non-positive and NaN values, rather than leaving the bound to
fall out of comparison arithmetic that happens to work for the common
case.

### 4.3 Why `Vec<Vec<bool>>`, not `PixelContainer`

Same reasoning as `VIS02` §4.1: pattern scanning is a pure binary-
image algorithm, and coupling it to `pixel-container::PixelContainer`
would make every caller pay for a channel model this algorithm never
looks at. A caller that has a `PixelContainer` (after thresholding)
converts to a bool bitmap once, at the call site.

## 5. What this does not decide

- How candidates get clustered into one refined center per real
  pattern — `L3`'s job, via `VIS02` (§2).
- Any specific ratio sequence — QR's `[1, 1, 3, 1, 1]` is a caller
  value, not a constant in this crate.
- What tolerance a real photographed QR code actually needs — that's
  discovered empirically once `L3` runs this against real images; this
  crate exposes it as a parameter rather than guessing a constant.

## 6. Test strategy

- **`scan_line` on hand-built run sequences**: an exact-ratio match
  (module size recovered exactly); a within-tolerance but not exact
  match; a just-outside-tolerance near-miss (correctly rejected); a
  window starting on a light run (rejected, even if the ratio would
  otherwise match starting one run later — i.e. the scan must still
  find the real match at the correct offset, not just correctly reject
  the misaligned one); multiple non-overlapping matches on the same
  line, all reported; no match in an unrelated line.
- **Degenerate `scan_line` input**: empty line, empty ratio,
  `tolerance` of `0.0`, a negative `tolerance`, and `f64::NAN` —
  proven not to panic and to return an empty (or, for `0.0`, an
  exact-only) result, not a wrong-but-plausible one.
- **`find_pattern_candidates` on a synthetic finder pattern**: a
  hand-built bitmap containing one QR-style nested-square finder
  pattern (1:1:3:1:1 both ways) on an otherwise blank background —
  confirmed candidate(s) found near its true center, `module_size`
  close to the pattern's actual module width.
- **Cross-check actually rejects false positives**: a bitmap with a
  row that matches the ratio horizontally but has no matching
  structure vertically through that point (e.g. a single dark bar) —
  zero candidates returned, proving the vertical cross-check is doing
  real filtering work, not passing everything through.
- **Multiple patterns**: a bitmap with two separated synthetic finder
  patterns — candidates found near both, not merged into one and not
  missing either.
- **Degenerate `find_pattern_candidates` input**: an empty bitmap, an
  all-background bitmap, and a ragged bitmap — proven not to panic.

## 7. Acceptance gates

1. §6's full test matrix is green.
2. No panic on any input shape or content this crate accepts,
   including empty/ragged bitmaps, empty ratios, and non-finite
   tolerance.
3. The cross-check false-positive test (§6) passes, proving `VIS04`
   isn't just a horizontal scan with an unused vertical stub.
4. `BUILD`, `README.md`, `CHANGELOG.md`, `required_capabilities.json`
   (empty — pure computation, no I/O) present, matching this
   workspace's per-package convention.
