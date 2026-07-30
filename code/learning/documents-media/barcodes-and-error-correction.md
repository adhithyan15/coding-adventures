<!-- learning-concepts: barcode-1d, barcode-layout-1d, barcode-2d, codabar, code128, code39, ean-13, itf, upc-a, qr-code, micro-qr, data-matrix, aztec-code, pdf417, gf256, polynomial, reed-solomon -->
# Barcodes And Error Correction

A barcode is a physical communication channel. Software chooses symbols, a
printer turns them into light and dark regions, a surface damages or distorts
them, a camera or scanner samples the result, and a decoder tries to recover
the original message.

```text
message -> encode -> lay out modules -> print/display
                                      -> noisy image
message <- decode <- sample modules <-
```

Thinking of the whole channel explains why encoding, layout, checksums, error
correction, and rendering belong in separate layers.

## One-Dimensional Symbols

A 1D barcode encodes information along one axis as alternating bars and spaces.
The other axis mainly provides height so a scanning line can intersect the
symbol reliably.

Different symbologies make different tradeoffs:

| Family | Typical design |
| --- | --- |
| Code 39 | simple, discrete characters with a small alphabet |
| Code 128 | dense full-ASCII-oriented encoding with code-set switching |
| Codabar | small alphabet designed for easy printing |
| ITF | pairs of digits interleaved across bars and spaces |
| EAN-13 / UPC-A | fixed retail identifiers with guard patterns and checks |

An encoder should first produce an abstract sequence of module widths or bits.
A layout layer then adds quiet zones, bar height, text placement, scaling, and
device coordinates. If encoding writes pixels directly, it becomes hard to
test the symbol independently from a particular renderer.

## Checksums Detect Common Errors

Retail codes use weighted modular checks. A check digit does not hide the data
or correct arbitrary damage; it rejects many likely substitutions and
transpositions.

For a weighted checksum:

```text
sum = w0*d0 + w1*d1 + ... + wn*dn
check = value that makes sum divisible by the modulus
```

The decoder recomputes the sum. A mismatch means the scan should not be trusted.
Always specify digit order and weight order: reversing either can produce a
plausible but incompatible implementation.

## Two-Dimensional Symbols

A 2D barcode arranges modules in a matrix. It can carry more data and spread
redundancy across both axes. The decoder also needs landmarks that answer:

- Where is the symbol?
- How is it rotated?
- What is the module size?
- Which cells contain metadata rather than payload?
- How should damaged cells be reconstructed?

QR Code uses finder, timing, alignment, format, and version regions around
masked payload modules. Micro QR reduces overhead for smaller messages. Data
Matrix uses an L-shaped finder boundary. Aztec Code uses a central bullseye.
PDF417 uses stacked rows of codewords rather than a square module grid.

These formats differ, but their encoder pipelines rhyme:

```text
choose mode
  -> encode payload bits
  -> add length and control information
  -> pad into codewords
  -> add error-correction codewords
  -> place fixed patterns and data
  -> choose/apply mask when required
  -> render with quiet zone
```

## Finite Fields Make Byte Correction Possible

Many 2D formats use Reed-Solomon codes over GF(256), a finite field with 256
elements. Field addition is bitwise XOR. Multiplication treats bytes as
polynomials whose coefficients are bits, then reduces the result by a chosen
irreducible polynomial.

For example, the byte:

```text
0b10010110
```

represents:

```text
x^7 + x^4 + x^2 + x
```

The important field property is that every nonzero element has a multiplicative
inverse. That allows division and polynomial algorithms without leaving the
finite set of byte values.

Implementations commonly precompute exponent and logarithm tables:

```text
a * b = exp(log(a) + log(b))
```

with exponents wrapped by the field's multiplicative period. Zero needs a
separate branch because it has no logarithm.

## Reed-Solomon Works On Polynomials

Treat data codewords as coefficients of a polynomial. The encoder appends
redundancy so the transmitted polynomial is divisible by a generator
polynomial.

The decoder evaluates the received polynomial at selected field points. A
zero result at every point means the codeword is internally consistent. Nonzero
results are syndromes: evidence describing the error pattern.

A full decoder then:

1. computes syndromes;
2. derives an error-locator polynomial;
3. finds error positions;
4. computes error magnitudes;
5. corrects the damaged codewords;
6. verifies that all syndromes are now zero.

With `r` correction codewords, a Reed-Solomon code can generally correct up to
`r/2` unknown erroneous codewords, or up to `r` erasures whose positions are
already known. Errors cost more than erasures because the decoder must discover
both location and value.

## Placement And Masking

After correction codewords are produced, a 2D encoder maps bits into available
cells while skipping reserved regions. Off-by-one errors here are common because
placement may zigzag, wrap, or change direction.

Some formats mask data modules to avoid visual patterns that are hard to scan,
such as large solid regions or finder-like shapes. QR evaluates several masks
and chooses the lowest penalty. The decoder learns the chosen mask from protected
format metadata and reverses it before reading codewords.

Masking is not encryption. It improves the physical signal.

## Rendering Is Part Of Correctness

A mathematically valid symbol can still be unreadable if rendered badly.

- Preserve the quiet zone around the symbol.
- Scale modules by an integer number of pixels when possible.
- Avoid antialiasing between dark and light modules.
- Preserve sufficient contrast.
- Do not crop finder or guard patterns.
- Keep 1D bar widths proportional after device scaling.

Renderers should consume a logical matrix or bar sequence, not repeat encoding
rules. This lets text, SVG, raster, terminal, and native paint backends share
the same tested symbol.

## Testing The Channel

Useful tests operate at several levels:

1. compare codewords against standard examples;
2. verify checksums and syndromes independently;
3. round-trip payload through encoder and decoder;
4. rotate and scale rendered symbols;
5. flip modules within the advertised correction budget;
6. reject damage beyond that budget;
7. compare output with an independent scanner;
8. fuzz dimensions, lengths, and truncated inputs.

The central lesson is that a barcode is not merely a black-and-white picture.
It is a layered protocol whose final medium happens to be light.
