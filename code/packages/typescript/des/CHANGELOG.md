# Changelog — @coding-adventures/des

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of DES block cipher (FIPS 46-3).
- `expandKey(key)`: derives 16 round subkeys (each 48 bits) from an 8-byte key using the PC-1/PC-2 key schedule with left-rotation amounts [1,1,2,2,2,2,2,2,1,2,2,2,2,2,2,1].
- `desEncryptBlock(block, key)`: encrypts one 8-byte block through IP → 16 Feistel rounds (E, XOR, S-boxes, P) → FP.
- `desDecryptBlock(block, key)`: decrypts by reversing the subkey order (Feistel self-inverse property).
- `desEcbEncrypt(plain, key)`: ECB mode with PKCS#7 padding (variable-length input).
- `desEcbDecrypt(cipher, key)`: ECB mode decryption with PKCS#7 unpadding and validation.
- `tdeaEncryptBlock(block, k1, k2, k3)`: Triple DES EDE encrypt — `E_K1(D_K2(E_K3(P)))`.
- `tdeaDecryptBlock(block, k1, k2, k3)`: Triple DES EDE decrypt — `D_K3(E_K2(D_K1(C)))`.
- `toHex(bytes)` / `fromHex(hex)`: utility conversions.
- Comprehensive test suite: FIPS 46 / SP 800-20 known-answer tests, round-trip tests, ECB padding edge cases, 3DES backward compatibility, and error handling.
- Coverage >80%.
