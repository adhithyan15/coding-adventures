## 0.1.0

- Initial Dart lane of the aes identity: `encryptBlock`/
  `decryptBlock` over flat column-major state, `expandKey` into
  16-byte round keys, `sbox`/`invSbox` tables built from
  lane-local 0x11B field arithmetic, and the fixed-memory
  `aes128EncryptBlockFixed` with a table-free S-box circuit and
  wipe-on-drop scratch; `AesInvalidKeyLength` carries the
  reference's exact message.
- Tests mirror the reference suite (38 unit tests incl. the FIPS
  vectors), add 3 extras, 4 differential tests over 90 seeded
  Node.js-OpenSSL vectors, 5 seeded property tests over the
  in-lane `pbt` package, and a 2,000-iteration differential fuzz
  campaign over the in-lane `fuzz` package; 100% line/function
  coverage.
