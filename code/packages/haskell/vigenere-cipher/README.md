# vigenere-cipher

## CR03 conformance

This implementation follows [CR03](../../../specs/CR03-vigenere-cipher.md): keys and cipher transforms use ASCII letters only, analysis ignores non-ASCII letters, and non-ASCII Unicode scalars pass through without advancing the key. Analysis accepts at most 8,192 Unicode scalars and key-length bounds up to 40, uses the smallest candidate within 90% of the best index-of-coincidence score, preserves the exact requested recovered-key length, and resolves score ties toward the smallest shift.

A pure Haskell implementation of the Vigenere polyalphabetic substitution
cipher and its classic statistical attack.

## API

- `encrypt` and `decrypt` preserve ASCII letter case, pass every other
  character through unchanged, and advance the key only for ASCII letters.
- `findKeyLength` estimates a key length up to 20 with the index of
  coincidence.
- `findKeyLengthWithLimit` applies the same analysis with an explicit limit.
- `findKey` recovers key letters with chi-squared English-frequency scoring.
- `breakCipher` combines length estimation, key recovery, and decryption.
- `englishFrequencies` exposes the A-through-Z reference distribution.

Keys are case-insensitive but must be non-empty and contain only ASCII
letters. Statistical recovery needs a sufficiently long sample of ordinary
English prose.

## Running the tests

```sh
cabal test all
```
