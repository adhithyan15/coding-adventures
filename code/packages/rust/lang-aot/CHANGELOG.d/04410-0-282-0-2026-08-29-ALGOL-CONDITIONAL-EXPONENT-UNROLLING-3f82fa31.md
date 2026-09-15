## 0.282.0 - 2026-08-29 (ALGOL conditional exponent unrolling)

The ALGOL matrix now proves on all seven standard backends that a pure runtime
conditional with equal exact tracked integer branches keeps its selector
control flow while the enclosing real power uses bounded multiplication.

