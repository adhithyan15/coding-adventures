# note-frequency (C++)

Musical note names to pitch frequencies, header-only in pure ISO C++17
(namespace `ca::note_frequency`) — a faithful port of the Rust `note-frequency`
crate. **No `<cmath>` / libm**: the one power (`2^x`) is computed from a
from-scratch `e^x`.

## The math

12-tone equal temperament, with concert **A4 = 440 Hz**:

```
freq = 440 * 2^(semitones_from_A4 / 12)
```

Notes are a letter (A–G), an optional accidental (`#`/`b`), and an octave —
`"A4"`, `"C#5"`, `"Db3"`. Only spellings that name a real chromatic pitch are
accepted, so `"Cb"` / `"E#"` / `"B#"` / `"Fb"` are rejected.

## Usage

```cpp
#include "note_frequency.hpp"
namespace nf = ca::note_frequency;

double hz = nf::note_to_frequency("A4");   // 440.0

nf::Note n = nf::parse_note("C#5");
n.semitones_from_a4();   // 4
n.frequency();           // 554.365… Hz
n.to_string();           // "C#5"
```

## Divergence from the Rust crate

Rust returns `Result<_, String>`; this port throws `std::invalid_argument` on a
malformed note or unsupported spelling. Frequencies match Rust's `f64::powf` to
within 1e-6.

## Building

```sh
sh BUILD    # builds & runs the tests under every C++ compiler present
```

Pure ISO C++17, no `<cmath>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).
