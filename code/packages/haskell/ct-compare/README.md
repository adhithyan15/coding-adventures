# ct-compare (Haskell)

Constant-time byte comparison and selection: the Haskell lane of
[CT01](../../../specs/CT01-constant-time-compare.md).

A secret comparison that stops at the first differing byte tells an attacker how
long the matching prefix was. This package reads every byte regardless, folds
the differences into one strict accumulator and inspects it once at the end.

## Type

library

## API

```haskell
import CtCompare

ctEq          :: ByteString -> ByteString -> Bool
ctEqFixed     :: ByteString -> ByteString -> Bool          -- same as ctEq
ctSelectBytes :: ByteString -> ByteString -> Word8 -> Either CtError ByteString
ctEqU64       :: Word64 -> Word64 -> Bool

ctEqBit       :: ByteString -> ByteString -> Word8          -- 1 or 0
ctEqFixedBit  :: ByteString -> ByteString -> Word8
ctEqU64Bit    :: Word64 -> Word64 -> Word8

data CtError = SelectLengthMismatch | InvalidChoice
```

* Inputs are strict `ByteString`s. Lazy `ByteString`, `String` and thunks are
  deliberately not accepted.
* A length mismatch makes `ctEq` return `False` (`ctEqBit` returns 0) without
  reading either input: lengths are public.
* `ctSelectBytes` returns `Left SelectLengthMismatch` (CT01's
  `CT_SELECT_LENGTH_MISMATCH`) for unequal lengths and `Left InvalidChoice` for
  a bit other than 0 or 1; both are decided before any content is read. It
  returns a freshly allocated `ByteString`.
* The selection bit is an integer (`Word8`, exactly 0 or 1): `Bool` has no
  branch-free conversion to a number that source code can name. Produce the bit
  with the `...Bit` variants and never with a conditional:

  ```haskell
  ctSelectBytes a b (ctEqBit x y)
  ```
* `ctEqU64Bit` computes `((d .|. negate d) `shiftR` 63) `xor` 1` for
  `d = x `xor` y`, folding all 64 bit positions with one fixed sequence.

## What this does and does not guarantee

The source has no content-dependent early return, branch, index or loop bound.
Work depends only on the public lengths, the accumulator is strict, and
selection uses the mask form `r xor ((l xor r) .&. mask)`.

It does **not** guarantee wall-clock constant time:

* GHC is an optimising compiler. It may unroll, vectorise or rewrite the loop,
  and nothing in Haskell lets the source pin the machine code. Whether an
  optimised build reintroduced an early exit has to be checked in the Core or
  assembly (`-ddump-simpl`, `-ddump-asm`) of the compiler version in use; this
  package does not claim that was done for every GHC version.
* The runtime is garbage collected: it can pause at any allocation and copies
  live data, so secrets may be moved around the heap and left behind in freed
  memory. The package cannot zero them.
* `ByteString`s are immutable and may share buffers; a returned selection is a
  fresh buffer but cannot be wiped.
* Cache and speculation behaviour of the host are outside the model.

Treat it as protection against *algorithmic* timing leaks only.

## Tests

```bash
cabal test all
```

`test/CtCompareSpec.hs` replays all 21 cases of the shared CT01 corpus
(`code/specs/fixtures/ct-compare-v1/cases.json`, decoded by the small JSON
reader in `test/Corpus.hs` because no JSON library ships with GHC) and adds
every single-byte mismatch position, every single-bit difference of every byte
value, all 256x256 byte pairs through selection under both bits, length
mismatches, every invalid choice 2..255, and all 64 single-bit flips of a
`Word64` from five bases. GHC's `hpc` reports 100% of the library's expressions
covered.

## Where it fits

A leaf utility with no dependencies beyond `base` and `bytestring` and no
capabilities (see `required_capabilities.json`).
