---
category: Rust
---

# wasm-bindgen `JsValue::from_str` aborts on native test targets

Gate behind `#[cfg(target_arch = "wasm32")]`; use `JsValue::NULL` placeholders for native error-path tests.
