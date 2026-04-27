use grammar_wasm_support::ast_to_json_string;
use language_lexer::{DEFAULT_VERSION, SUPPORTED_VERSIONS};
use language_parser::create_python_parser;
use wasm_bindgen::prelude::*;

fn resolve_version(version: &str) -> &str {
    if version.is_empty() { DEFAULT_VERSION } else { version }
}

fn to_js_error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}

#[wasm_bindgen]
pub fn parse(source: &str, version: &str) -> Result<String, JsValue> {
    let mut parser = create_python_parser(source, resolve_version(version)).map_err(to_js_error)?;
    let ast = parser
        .parse()
        .map_err(|e| to_js_error(format!("Python parse failed: {e}")))?;
    ast_to_json_string(ast).map_err(|e| JsValue::from_str(&e.to_string()))
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
    fn wrapper_parses_to_json() {
        let json = parse("x = 1\n", "").unwrap();
        assert!(json.contains("\"rule_name\""));
    }
}
