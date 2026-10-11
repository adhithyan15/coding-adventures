---
category: Rust
---

# A Rust graph fixture must compare String parents with str slices rather than references to owned Strings

The chronology red test compared `Vec<String>` with `[&String; N]`; after the
missing constructor was implemented, that unrelated type error hid runtime
validation. Compare with `[a.as_str(), ...]` so the supported `String: PartialEq<str>`
implementation applies. Verify fixture assertions independently of missing APIs.
