# Changelog — RespProtocol (Swift)

## 0.1.0 — Initial release

- Add all seven distinct RESP2 value variants with binary-safe bulk strings.
- Add deterministic bounded encoding and strict failure-atomic decoding.
- Add incomplete-frame results, multi-frame decoding, and a bounded fragmented
  stream decoder with retained-tail ownership.
- Add stable resource and syntax errors, exact DT23 case fixtures, native tests,
  cross-platform build fronts, documentation, and an empty capability profile.
