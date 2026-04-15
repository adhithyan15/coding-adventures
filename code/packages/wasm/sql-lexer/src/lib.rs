use grammar_wasm_support::tokens_to_json_string;
use language_lexer::create_sql_lexer;
use wasm_bindgen::prelude::*;

fn to_js_error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}

#[wasm_bindgen]
pub fn tokenize(source: &str) -> Result<String, JsValue> {
    let mut lexer = create_sql_lexer(source);
    let tokens = lexer
        .tokenize()
        .map_err(|e| to_js_error(format!("SQL tokenization failed: {e}")))?;
    tokens_to_json_string(tokens).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_tokenizes_to_json() {
        let json = tokenize("SELECT 1 FROM t").unwrap();
        assert!(json.contains("\"type_name\""));
    }
}
