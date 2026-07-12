# note-frequency (C)

Musical note names to pitch frequencies, in pure ISO C17 — a faithful port of
the Rust `note-frequency` crate. **No libm**: the one power (`2^x`) is computed
from a from-scratch `e^x`.

## The math

Western music divides each octave into twelve equal semitones (12-tone equal
temperament). Fixing concert **A4 = 440 Hz**, any note's frequency is:

```
freq = 440 * 2^(semitones_from_A4 / 12)
```

A note is a letter (A–G), an optional accidental (`#` or `b`), and an octave —
`"A4"`, `"C#5"`, `"Db3"`. Only natural notes plus a single sharp/flat, and only
spellings that name a real chromatic pitch, so `"Cb"` / `"E#"` / `"B#"` / `"Fb"`
are rejected (matching the Rust crate).

## API

```c
#include "note_frequency.h"

double hz;
if (nf_note_to_frequency("A4", &hz) == NF_OK) {
    /* hz == 440.0 */
}

NfNote n;
nf_parse_note("C#5", &n);
nf_note_semitones_from_a4(&n);   /* 4  */
nf_note_frequency(&n);           /* 554.365… Hz */

char buf[16];
nf_note_to_string(&n, buf, sizeof buf);   /* "C#5" */
```

`NfNote` is a small fixed-size value — copy it freely, nothing to free.

## Divergence from the Rust crate

Rust returns `Result<_, String>`; this port returns an `NfStatus` code
(`NF_OK` / `NF_ERR_INVALID_SPELLING` / `NF_ERR_INVALID_NOTE`) and writes results
through out-parameters. Frequencies match Rust's `f64::powf` to within 1e-6.

## Building

```sh
sh BUILD    # builds & runs the tests under every C compiler present
```

Pure ISO C17, no `<math.h>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Where it fits

Part of the C/C++ port campaign. Reuses the from-scratch `e^x` first built for
[`trig`](../trig) and [`activation-functions`](../activation-functions), here in
service of equal-temperament pitch.
