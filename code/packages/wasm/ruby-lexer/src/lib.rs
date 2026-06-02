use grammar_wasm_support::tokens_to_json_string;
use language_lexer::tokenize_ruby;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn tokenize(source: &str) -> Result<String, JsValue> {
    // The Ruby lexer is an era-aware state machine rather than a generic
    // grammar-driven lexer, so it exposes `tokenize_ruby` (infallible — it
    // emits error tokens instead of raising) rather than the
    // `create_*_lexer(...).tokenize()` shape used by the grammar-backed
    // wasm wrappers.  Both crates share the same `lexer::token::Token`, so
    // the token vector feeds straight into the shared JSON serializer.
    let tokens = tokenize_ruby(source);
    tokens_to_json_string(tokens).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_tokenizes_to_json() {
        let json = tokenize("def add(a, b)\n  a + b\nend").unwrap();
        assert!(json.contains("\"type_name\""));
    }
}
