---
category: Rust
---

# `array::from_fn` needs an explicit length when borrowed immediately

When a freshly constructed array is passed directly to a function accepting a
slice, the slice type does not tell Rust the array's const-generic length.
Spell the length at the construction site, for example
`std::array::from_fn::<_, 8, _>(...)`, instead of expecting the later slice
coercion to infer it.
