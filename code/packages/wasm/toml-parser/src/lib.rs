use grammar_wasm_support::ast_to_json_string;
use language_parser::create_toml_parser;
use wasm_bindgen::prelude::*;

fn to_js_error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}

#[wasm_bindgen]
pub fn parse(source: &str) -> Result<String, JsValue> {
    let mut parser = create_toml_parser(source);
    let ast = parser
        .parse()
        .map_err(|e| to_js_error(format!("TOML parse failed: {e}")))?;
    ast_to_json_string(ast).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_parses_to_json() {
        let json = parse("title = \"TOML\"\n").unwrap();
        assert!(json.contains("\"rule_name\""));
    }
}
