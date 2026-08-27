# CodingAdventures.VigenereCipher.FSharp

## CR03 conformance

This implementation follows [CR03](../../../specs/CR03-vigenere-cipher.md): keys and cipher transforms use ASCII letters only, analysis ignores non-ASCII letters, and non-ASCII Unicode scalars pass through without advancing the key. Analysis accepts at most 8,192 Unicode scalars and key-length bounds up to 40, uses the smallest candidate within 90% of the best index-of-coincidence score, preserves the exact requested recovered-key length, and resolves score ties toward the smallest shift.

Vigenere cipher helpers for encrypting, decrypting, estimating key length,
recovering keys with frequency analysis, and breaking sufficiently long
ciphertexts.
