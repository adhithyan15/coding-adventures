//! Browser/WASM-facing JSON facade for the Rust MACSYMA runtime.
//!
//! The core runtime stays typed and Rust-native. This crate provides a stable
//! string boundary for browser embeddings: source text in, JSON result out.

use cas_pretty_printer::{format_lisp, pretty, MacsymaDialect};
use coding_adventures_macsyma_runtime::{EvalResult, History, MacsymaSession};
use serde::Serialize;
use std::any::Any;
use std::panic::{catch_unwind, set_hook, take_hook, AssertUnwindSafe};
use std::sync::{Mutex, OnceLock};
use symbolic_ir::IRNode;

/// Stateful MACSYMA session with JSON result helpers.
pub struct MacsymaWasmSession {
    session: MacsymaSession,
}

impl MacsymaWasmSession {
    pub fn new() -> Self {
        Self {
            session: MacsymaSession::new(),
        }
    }

    /// Evaluate MACSYMA source and return a JSON string.
    ///
    /// The returned JSON always has an `ok` boolean. Compile errors are encoded
    /// as `{ "ok": false, "error": { ... } }` so JS callers do not need Rust
    /// error plumbing to render diagnostics.
    pub fn eval_json(&mut self, source: &str) -> String {
        match catch_parser_panics(|| self.session.eval_source(source)) {
            Ok(Ok(results)) => encode_response(EvalResponse::ok(results, self.session.history())),
            Ok(Err(error)) => {
                encode_response(EvalResponse::err(error.to_string(), self.session.history()))
            }
            Err(payload) => encode_response(EvalResponse::err(
                panic_payload_to_string(&payload),
                self.session.history(),
            )),
        }
    }

    pub fn history_json(&self) -> String {
        encode_response(HistoryResponse {
            ok: true,
            history: JsonHistory::from_history(self.session.history()),
        })
    }

    pub fn reset_history(&mut self) {
        self.session.history_mut().reset();
    }

    pub fn inner(&self) -> &MacsymaSession {
        &self.session
    }
}

impl Default for MacsymaWasmSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Stateless convenience function for single-shot browser calls.
pub fn eval_source_json(source: &str) -> String {
    let mut session = MacsymaWasmSession::new();
    session.eval_json(source)
}

#[derive(Serialize)]
struct EvalResponse {
    ok: bool,
    results: Vec<JsonEvalResult>,
    visible_outputs: Vec<String>,
    history: JsonHistory,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonError>,
}

impl EvalResponse {
    fn ok(results: Vec<EvalResult>, history: &History) -> Self {
        let results = results
            .into_iter()
            .map(JsonEvalResult::from_result)
            .collect::<Vec<_>>();
        let visible_outputs = results
            .iter()
            .filter(|result| result.display)
            .map(|result| result.output_macsyma.clone())
            .collect();
        Self {
            ok: true,
            results,
            visible_outputs,
            history: JsonHistory::from_history(history),
            error: None,
        }
    }

    fn err(message: String, history: &History) -> Self {
        Self {
            ok: false,
            results: Vec::new(),
            visible_outputs: Vec::new(),
            history: JsonHistory::from_history(history),
            error: Some(JsonError {
                kind: "compile".to_string(),
                message,
            }),
        }
    }
}

#[derive(Serialize)]
struct HistoryResponse {
    ok: bool,
    history: JsonHistory,
}

#[derive(Serialize)]
struct JsonEvalResult {
    input_index: usize,
    output_index: usize,
    display: bool,
    input_macsyma: String,
    output_macsyma: String,
    input_lisp: String,
    output_lisp: String,
    input_ir: JsonIrNode,
    output_ir: JsonIrNode,
}

impl JsonEvalResult {
    fn from_result(result: EvalResult) -> Self {
        Self {
            input_index: result.input_index,
            output_index: result.output_index,
            display: result.display,
            input_macsyma: pretty(&result.input, &MacsymaDialect),
            output_macsyma: result.output_text,
            input_lisp: format_lisp(&result.input),
            output_lisp: format_lisp(&result.output),
            input_ir: JsonIrNode::from_ir(&result.input),
            output_ir: JsonIrNode::from_ir(&result.output),
        }
    }
}

#[derive(Serialize)]
struct JsonHistory {
    input_count: usize,
    output_count: usize,
    next_input_index: usize,
    last_output_macsyma: Option<String>,
    last_output_lisp: Option<String>,
}

impl JsonHistory {
    fn from_history(history: &History) -> Self {
        Self {
            input_count: history.inputs().len(),
            output_count: history.outputs().len(),
            next_input_index: history.next_input_index(),
            last_output_macsyma: history
                .last_output()
                .map(|output| pretty(output, &MacsymaDialect)),
            last_output_lisp: history.last_output().map(format_lisp),
        }
    }
}

#[derive(Serialize)]
struct JsonError {
    kind: String,
    message: String,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum JsonIrNode {
    Symbol {
        name: String,
    },
    Integer {
        value: i64,
    },
    Rational {
        numerator: i64,
        denominator: i64,
    },
    Float {
        value: f64,
    },
    String {
        value: String,
    },
    Apply {
        head: Box<JsonIrNode>,
        args: Vec<JsonIrNode>,
    },
}

impl JsonIrNode {
    fn from_ir(node: &IRNode) -> Self {
        match node {
            IRNode::Symbol(name) => JsonIrNode::Symbol { name: name.clone() },
            IRNode::Integer(value) => JsonIrNode::Integer { value: *value },
            IRNode::Rational(numerator, denominator) => JsonIrNode::Rational {
                numerator: *numerator,
                denominator: *denominator,
            },
            IRNode::Float(value) => JsonIrNode::Float { value: *value },
            IRNode::Str(value) => JsonIrNode::String {
                value: value.clone(),
            },
            IRNode::Apply(apply) => JsonIrNode::Apply {
                head: Box::new(JsonIrNode::from_ir(&apply.head)),
                args: apply.args.iter().map(JsonIrNode::from_ir).collect(),
            },
        }
    }
}

fn encode_response<T: Serialize>(response: T) -> String {
    serde_json::to_string(&response).unwrap_or_else(|error| {
        format!(
            r#"{{"ok":false,"error":{{"kind":"serialization","message":"{}"}}}}"#,
            escape_json_string(&error.to_string())
        )
    })
}

fn escape_json_string(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| ch.escape_default())
        .collect::<String>()
}

fn panic_payload_to_string(payload: &Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "MACSYMA evaluation panicked".to_string()
    }
}

fn catch_parser_panics<F, T>(f: F) -> Result<T, Box<dyn Any + Send>>
where
    F: FnOnce() -> T,
{
    static PANIC_HOOK_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let lock = PANIC_HOOK_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().unwrap();
    let previous_hook = take_hook();
    set_hook(Box::new(|_| {}));
    let result = catch_unwind(AssertUnwindSafe(f));
    set_hook(previous_hook);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn json(raw: &str) -> Value {
        serde_json::from_str(raw).expect("facade should emit valid JSON")
    }

    #[test]
    fn evaluates_single_shot_source_to_json() {
        let payload = json(&eval_source_json("1 + 2 * 3;"));
        assert_eq!(payload["ok"], true);
        assert_eq!(payload["results"][0]["output_macsyma"], "7");
        assert_eq!(payload["results"][0]["output_ir"]["kind"], "integer");
        assert_eq!(payload["results"][0]["output_ir"]["value"], 7);
        assert_eq!(payload["visible_outputs"][0], "7");
    }

    #[test]
    fn preserves_session_bindings_across_calls() {
        let mut session = MacsymaWasmSession::new();
        let first = json(&session.eval_json("x : 5$"));
        assert_eq!(first["visible_outputs"].as_array().unwrap().len(), 0);

        let second = json(&session.eval_json("x + 2;"));
        assert_eq!(second["results"][0]["input_index"], 2);
        assert_eq!(second["results"][0]["output_macsyma"], "7");
        assert_eq!(second["history"]["input_count"], 2);
    }

    #[test]
    fn encodes_symbolic_ir_shape() {
        let payload = json(&eval_source_json("(x + 0) * y^2;"));
        let output = &payload["results"][0]["output_ir"];
        assert_eq!(output["kind"], "apply");
        assert_eq!(output["head"]["name"], "Mul");
        assert_eq!(output["args"][0]["name"], "x");
        assert_eq!(payload["results"][0]["output_lisp"], "(Mul x (Pow y 2))");
    }

    #[test]
    fn uses_runtime_display_text_for_display2d() {
        let payload = json(&eval_source_json("ev(1/(x + 1), display2d);"));
        let output = payload["results"][0]["output_macsyma"]
            .as_str()
            .expect("output_macsyma should be string");

        assert!(output.contains('\n'));
        assert!(output.contains('─'));
        assert!(output.contains("x + 1"));
        assert_eq!(payload["visible_outputs"][0], output);
    }

    #[test]
    fn reports_compile_errors_as_json() {
        let payload = json(&eval_source_json("1 + ;"));
        assert_eq!(payload["ok"], false);
        assert_eq!(payload["error"]["kind"], "compile");
        assert!(payload["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("Incorrect syntax at line 1, column "));
    }

    #[test]
    fn reports_help_queries_as_json_visible_output() {
        let payload = json(&eval_source_json("? solve"));
        assert_eq!(payload["ok"], true);
        assert!(payload["visible_outputs"][0]
            .as_str()
            .unwrap()
            .contains("solve(expr, var)"));
        assert_eq!(payload["results"][0]["output_ir"]["kind"], "string");
    }

    #[test]
    fn can_reset_history_without_recreating_session() {
        let mut session = MacsymaWasmSession::new();
        session.eval_json("1; 2;");
        session.reset_history();
        let payload = json(&session.history_json());
        assert_eq!(payload["history"]["input_count"], 0);
        assert_eq!(payload["history"]["next_input_index"], 1);
        assert!(payload["history"]["last_output_macsyma"].is_null());
    }
}
