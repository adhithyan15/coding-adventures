use grammar_wasm_support::ast_to_json_string;
use language_parser::create_excel_parser;
use wasm_bindgen::prelude::*;

fn to_js_error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}

#[wasm_bindgen]
pub fn parse(source: &str) -> Result<String, JsValue> {
    let mut parser = create_excel_parser(source);
    let ast = parser
        .parse()
        .map_err(|e| to_js_error(format!("Excel parse failed: {e}")))?;
    ast_to_json_string(ast).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_parses_to_json() {
        let json = parse("=SUM(A1:B2)").unwrap();
        assert!(json.contains("\"rule_name\""));
    }
}
