use grammar_wasm_support::tokens_to_json_string;
use language_lexer::{create_javascript_lexer, DEFAULT_VERSION, SUPPORTED_VERSIONS};
use wasm_bindgen::prelude::*;

fn resolve_version(version: &str) -> &str {
    if version.is_empty() { DEFAULT_VERSION } else { version }
}

fn to_js_error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}

#[wasm_bindgen]
pub fn tokenize(source: &str, version: &str) -> Result<String, JsValue> {
    let mut lexer = create_javascript_lexer(source, resolve_version(version)).map_err(to_js_error)?;
    let tokens = lexer
        .tokenize()
        .map_err(|e| to_js_error(format!("JavaScript tokenization failed: {e}")))?;
    tokens_to_json_string(tokens).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "supportedVersions")]
pub fn supported_versions() -> Vec<String> {
    SUPPORTED_VERSIONS.iter().map(|version| (*version).to_string()).collect()
}

#[wasm_bindgen(js_name = "defaultVersion")]
pub fn default_version() -> String {
    DEFAULT_VERSION.to_string()
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_tokenizes_to_json() {
        let json = tokenize("let x = 1;", "").unwrap();
        assert!(json.contains("\"type_name\""));
    }
}
