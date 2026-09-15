## 0.55.0 — 2026-08-02 — runtime string procedure results in string arrays

Regression coverage now proves that a runtime `string procedure` result can
populate `array<str>` storage, be read back for lexical ordering, and be printed
through the shared string runtime path.

