//! # HTML Renderer — self-contained HTML reports for the computing stack.
//!
//! The key architectural idea of this package is that the renderer is
//! **language-agnostic**. It does not know whether the report was produced by
//! Python, Ruby, TypeScript, or Rust. It only knows the shared JSON contract.
//!
//! This crate provides:
//! - serde-backed contract types
//! - HTML escaping
//! - stage-aware rendering for common pipeline stages
//! - a JSON fallback for unknown stages
//! - file I/O helpers for reading reports and writing HTML

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;

/// Errors that can happen while loading or rendering reports.
#[derive(Debug)]
pub enum RenderError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl Display for RenderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Json(err) => write!(f, "JSON error: {err}"),
        }
    }
}

impl std::error::Error for RenderError {}

impl From<std::io::Error> for RenderError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for RenderError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

/// Root report object shared across language implementations.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PipelineReport {
    pub source: String,
    pub language: String,
    pub target: String,
    pub metadata: ReportMetadata,
    pub stages: Vec<StageReport>,
}

/// Metadata about when and how the report was generated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub generated_at: String,
    pub generator_version: String,
    pub packages: BTreeMap<String, String>,
}

/// One stage in the pipeline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StageReport {
    pub name: String,
    pub display_name: String,
    pub input_repr: String,
    pub output_repr: String,
    pub duration_ms: f64,
    pub data: Value,
}

impl StageReport {
    pub fn new(
        name: impl Into<String>,
        display_name: impl Into<String>,
        input_repr: impl Into<String>,
        output_repr: impl Into<String>,
        duration_ms: f64,
        data: Value,
    ) -> Self {
        Self {
            name: name.into(),
            display_name: display_name.into(),
            input_repr: input_repr.into(),
            output_repr: output_repr.into(),
            duration_ms,
            data,
        }
    }
}

/// Main renderer for pipeline reports.
#[derive(Clone, Debug, Default)]
pub struct HTMLRenderer;

impl HTMLRenderer {
    pub fn new() -> Self {
        Self
    }

    /// Render a report to a complete self-contained HTML document.
    pub fn render(&self, report: &PipelineReport) -> Result<String, RenderError> {
        let header = render_header(report);
        let stages = report
            .stages
            .iter()
            .map(render_stage)
            .collect::<Vec<_>>()
            .join("\n");
        let footer = render_footer(report);

        Ok(format!(
            "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"UTF-8\">\n  \
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n  \
             <title>Computing Stack Report — {}</title>\n  <style>{}</style>\n</head>\n<body>\n{}\n{}\n{}\n</body>\n</html>",
            escape_html(&report.source),
            styles(),
            header,
            stages,
            footer
        ))
    }

    /// Load a JSON file from disk and render it.
    pub fn render_from_json<P: AsRef<Path>>(&self, json_path: P) -> Result<String, RenderError> {
        let raw = fs::read_to_string(json_path)?;
        let report: PipelineReport = serde_json::from_str(&raw)?;
        self.render(&report)
    }

    /// Render a report and write it to a file.
    pub fn render_to_file<P: AsRef<Path>>(
        &self,
        report: &PipelineReport,
        output_path: P,
    ) -> Result<(), RenderError> {
        let html = self.render(report)?;
        fs::write(output_path, html)?;
        Ok(())
    }
}

fn render_header(report: &PipelineReport) -> String {
    let date = report
        .metadata
        .generated_at
        .split('T')
        .next()
        .unwrap_or(report.metadata.generated_at.as_str());

    format!(
        "<header>\n  <h1>Computing Stack Report</h1>\n  <pre class=\"source-code\"><code>{}</code></pre>\n  \
         <p class=\"meta-info\">Implementation: {} | Target: {} | Generated: {}</p>\n</header>",
        escape_html(&report.source),
        escape_html(&report.language),
        escape_html(&report.target),
        escape_html(date)
    )
}

fn render_stage(stage: &StageReport) -> String {
    let content = match stage.name.as_str() {
        "lexer" => render_lexer_stage(&stage.data),
        "parser" => render_parser_stage(&stage.data),
        "compiler" => render_compiler_stage(&stage.data),
        "vm" => render_vm_stage(&stage.data),
        "assembler" => render_assembler_stage(&stage.data),
        "riscv" | "arm" => render_hardware_execution_stage(&stage.data),
        "alu" => render_alu_stage(&stage.data),
        "gates" => render_gate_stage(&stage.data),
        _ => render_fallback_stage(&stage.data),
    };

    format!(
        "<section id=\"{}\">\n  <h2>{}</h2>\n  <div class=\"stage-meta\">\n    \
         <span>Input: {}</span>\n    <span>Output: {}</span>\n    \
         <span>Duration: {:.2}ms</span>\n  </div>\n  {}\n</section>",
        escape_html(&stage.name),
        escape_html(&stage.display_name),
        escape_html(&stage.input_repr),
        escape_html(&stage.output_repr),
        stage.duration_ms,
        content
    )
}

fn render_footer(report: &PipelineReport) -> String {
    format!(
        "<footer>\n  <p>Generated by coding-adventures HTML Visualizer v{}</p>\n</footer>",
        escape_html(&report.metadata.generator_version)
    )
}

fn render_lexer_stage(data: &Value) -> String {
    let tokens = data
        .get("tokens")
        .and_then(Value::as_array)
        .map(|tokens| {
            tokens
                .iter()
                .map(|token| {
                    let token_type = token.get("type").and_then(Value::as_str).unwrap_or("UNKNOWN");
                    let value = token.get("value").and_then(Value::as_str).unwrap_or("");
                    format!(
                        "<div class=\"token {}\"><span class=\"token-type\">{}</span><span class=\"token-value\">{}</span></div>",
                        classify_token(token_type),
                        escape_html(token_type),
                        if value.is_empty() {
                            "\"\"".to_string()
                        } else {
                            escape_html(value)
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    format!("<div class=\"token-list\">{tokens}</div>")
}

fn render_parser_stage(data: &Value) -> String {
    let ast = data.get("ast").unwrap_or(&Value::Null);
    let pretty = serde_json::to_string_pretty(ast).unwrap_or_else(|_| "null".to_string());
    format!(
        "<div class=\"ast-view\"><pre><code>{}</code></pre></div>",
        escape_html(&pretty)
    )
}

fn render_compiler_stage(data: &Value) -> String {
    let rows = data
        .get("instructions")
        .and_then(Value::as_array)
        .map(|instructions| {
            instructions
                .iter()
                .map(|inst| {
                    let index = inst.get("index").map(value_to_inline).unwrap_or_default();
                    let opcode = inst.get("opcode").map(value_to_inline).unwrap_or_default();
                    let arg = inst.get("arg").map(value_to_inline).unwrap_or_else(|| "null".to_string());
                    let stack_effect = inst
                        .get("stack_effect")
                        .map(value_to_inline)
                        .unwrap_or_default();
                    format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                        escape_html(&index),
                        escape_html(&opcode),
                        escape_html(&arg),
                        escape_html(&stack_effect)
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    format!(
        "<table class=\"data-table\"><thead><tr><th>Index</th><th>Opcode</th><th>Arg</th><th>Stack Effect</th></tr></thead><tbody>{rows}</tbody></table>"
    )
}

fn render_vm_stage(data: &Value) -> String {
    let rows = data
        .get("steps")
        .and_then(Value::as_array)
        .map(|steps| {
            steps
                .iter()
                .map(|step| {
                    let index = step.get("index").map(value_to_inline).unwrap_or_default();
                    let instruction = step.get("instruction").map(value_to_inline).unwrap_or_default();
                    let stack_before = pretty_json(step.get("stack_before").unwrap_or(&Value::Null));
                    let stack_after = pretty_json(step.get("stack_after").unwrap_or(&Value::Null));
                    let variables = pretty_json(step.get("variables").unwrap_or(&Value::Null));
                    format!(
                        "<tr><td>{}</td><td>{}</td><td><pre>{}</pre></td><td><pre>{}</pre></td><td><pre>{}</pre></td></tr>",
                        escape_html(&index),
                        escape_html(&instruction),
                        escape_html(&stack_before),
                        escape_html(&stack_after),
                        escape_html(&variables),
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    format!(
        "<table class=\"data-table\"><thead><tr><th>Step</th><th>Instruction</th><th>Stack Before</th><th>Stack After</th><th>Variables</th></tr></thead><tbody>{rows}</tbody></table>"
    )
}

fn render_assembler_stage(data: &Value) -> String {
    let rows = data
        .get("lines")
        .and_then(Value::as_array)
        .map(|lines| {
            lines.iter()
                .map(|line| {
                    let address = line.get("address").map(value_to_inline).unwrap_or_default();
                    let assembly = line.get("assembly").map(value_to_inline).unwrap_or_default();
                    let binary = line.get("binary").map(value_to_inline).unwrap_or_default();
                    format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                        escape_html(&address),
                        escape_html(&assembly),
                        escape_html(&binary),
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    format!(
        "<table class=\"data-table\"><thead><tr><th>Address</th><th>Assembly</th><th>Binary</th></tr></thead><tbody>{rows}</tbody></table>"
    )
}

fn render_hardware_execution_stage(data: &Value) -> String {
    let rows = data
        .get("steps")
        .and_then(Value::as_array)
        .map(|steps| {
            steps.iter()
                .map(|step| {
                    let address = step.get("address").map(value_to_inline).unwrap_or_default();
                    let instruction = step.get("instruction").map(value_to_inline).unwrap_or_default();
                    let changed = pretty_json(step.get("registers_changed").unwrap_or(&Value::Null));
                    let registers = pretty_json(step.get("registers").unwrap_or(&Value::Null));
                    format!(
                        "<tr><td>{}</td><td>{}</td><td><pre>{}</pre></td><td><pre>{}</pre></td></tr>",
                        escape_html(&address),
                        escape_html(&instruction),
                        escape_html(&changed),
                        escape_html(&registers),
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    format!(
        "<table class=\"data-table\"><thead><tr><th>Address</th><th>Instruction</th><th>Registers Changed</th><th>Registers</th></tr></thead><tbody>{rows}</tbody></table>"
    )
}

fn render_alu_stage(data: &Value) -> String {
    let rows = data
        .get("operations")
        .and_then(Value::as_array)
        .map(|ops| {
            ops.iter()
                .map(|op| {
                    let opname = op.get("op").map(value_to_inline).unwrap_or_default();
                    let a = op.get("a").map(value_to_inline).unwrap_or_default();
                    let b = op.get("b").map(value_to_inline).unwrap_or_default();
                    let result = op.get("result").map(value_to_inline).unwrap_or_default();
                    format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                        escape_html(&opname),
                        escape_html(&a),
                        escape_html(&b),
                        escape_html(&result)
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    format!(
        "<table class=\"data-table\"><thead><tr><th>Op</th><th>A</th><th>B</th><th>Result</th></tr></thead><tbody>{rows}</tbody></table>"
    )
}

fn render_gate_stage(data: &Value) -> String {
    let html = data
        .get("operations")
        .and_then(Value::as_array)
        .map(|ops| {
            ops.iter()
                .map(|op| {
                    let description = op
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or("Gate operation");
                    let gates = op.get("gates").and_then(Value::as_array).cloned().unwrap_or_default();
                    let list = gates
                        .iter()
                        .map(|gate| {
                            let gate_name = gate.get("gate").map(value_to_inline).unwrap_or_default();
                            let label = gate.get("label").map(value_to_inline).unwrap_or_default();
                            let output = gate.get("output").map(value_to_inline).unwrap_or_default();
                            format!(
                                "<li><strong>{}</strong> — {} → {}</li>",
                                escape_html(&gate_name),
                                escape_html(&label),
                                escape_html(&output)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("");
                    format!(
                        "<div class=\"gate-op\"><h3>{}</h3><ul>{}</ul></div>",
                        escape_html(description),
                        list
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    format!("<div class=\"gate-list\">{html}</div>")
}

fn render_fallback_stage(data: &Value) -> String {
    let pretty = pretty_json(data);
    format!(
        "<div class=\"fallback-json\"><pre><code>{}</code></pre></div>",
        escape_html(&pretty)
    )
}

fn classify_token(token_type: &str) -> &'static str {
    let upper = token_type.to_ascii_uppercase();
    if upper.contains("NAME") || upper.contains("IDENT") || upper.contains("VARIABLE") {
        return "token-name";
    }
    if upper.contains("NUMBER") || upper.contains("INT") || upper.contains("FLOAT") || upper.contains("DIGIT") {
        return "token-number";
    }
    if upper.contains("PLUS")
        || upper.contains("MINUS")
        || upper.contains("STAR")
        || upper.contains("SLASH")
        || upper.contains("EQUAL")
        || upper.contains("PAREN")
        || upper.contains("BRACE")
        || upper.contains("BRACKET")
        || upper.contains("COMMA")
        || upper.contains("SEMICOL")
        || upper.contains("COLON")
        || upper.contains("DOT")
        || upper.contains("OPERATOR")
        || upper.contains("ASSIGN")
    {
        return "token-operator";
    }
    if upper.contains("KEYWORD")
        || matches!(
            upper.as_str(),
            "IF" | "ELSE" | "WHILE" | "FOR" | "DEF" | "RETURN" | "CLASS" | "IMPORT"
        )
    {
        return "token-keyword";
    }
    if upper.contains("STRING") || upper.contains("STR") {
        return "token-string";
    }
    "token-default"
}

fn value_to_inline(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::String(s) => s.clone(),
        _ => value.to_string(),
    }
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".to_string())
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn styles() -> &'static str {
    r#"
body { background-color: #111827; color: #e5e7eb; font-family: ui-sans-serif, system-ui, sans-serif; margin: 0; padding: 2rem; line-height: 1.5; }
header, footer, section { max-width: 1100px; margin: 0 auto 1.5rem auto; }
h1, h2, h3 { color: #f9fafb; }
section { background: #1f2937; border: 1px solid #374151; border-radius: 12px; padding: 1rem 1.25rem; }
.source-code, pre { background: #0f172a; color: #e5e7eb; border-radius: 8px; padding: 0.75rem; overflow-x: auto; }
.meta-info, .stage-meta { color: #cbd5e1; font-size: 0.95rem; }
.stage-meta { display: flex; gap: 1rem; flex-wrap: wrap; margin-bottom: 1rem; }
.token-list { display: flex; gap: 0.5rem; flex-wrap: wrap; }
.token { border-radius: 999px; padding: 0.35rem 0.7rem; display: inline-flex; gap: 0.5rem; font-family: monospace; border: 1px solid transparent; }
.token-name { background: #1d4ed8; border-color: #60a5fa; }
.token-number { background: #166534; border-color: #4ade80; }
.token-operator { background: #991b1b; border-color: #f87171; }
.token-keyword { background: #854d0e; border-color: #facc15; }
.token-string { background: #6d28d9; border-color: #c084fc; }
.token-default { background: #374151; border-color: #9ca3af; }
.token-type { font-weight: 700; }
.data-table { width: 100%; border-collapse: collapse; }
.data-table th, .data-table td { border: 1px solid #4b5563; padding: 0.55rem 0.7rem; text-align: left; vertical-align: top; }
.data-table th { background: #111827; }
.gate-op { margin-bottom: 1rem; }
pre code { font-family: ui-monospace, SFMono-Regular, monospace; }
"#
}

#[cfg(test)]
mod tests {
    use super::{HTMLRenderer, PipelineReport, ReportMetadata, StageReport};
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn metadata() -> ReportMetadata {
        ReportMetadata {
            generated_at: "2026-03-18T12:00:00Z".to_string(),
            generator_version: "0.1.0".to_string(),
            packages: BTreeMap::from([
                ("lexer".to_string(), "0.1.0".to_string()),
                ("parser".to_string(), "0.1.0".to_string()),
            ]),
        }
    }

    fn vm_report() -> PipelineReport {
        PipelineReport {
            source: "x = 1 + 2".to_string(),
            language: "python".to_string(),
            target: "vm".to_string(),
            metadata: metadata(),
            stages: vec![
                StageReport::new(
                    "lexer",
                    "Tokenization",
                    "x = 1 + 2",
                    "6 tokens",
                    0.12,
                    json!({
                        "tokens": [
                            {"type": "NAME", "value": "x", "line": 1, "column": 1},
                            {"type": "NUMBER", "value": "1", "line": 1, "column": 5},
                            {"type": "EOF", "value": "", "line": 1, "column": 10}
                        ]
                    }),
                ),
                StageReport::new(
                    "parser",
                    "Parsing",
                    "6 tokens",
                    "AST with 5 nodes",
                    0.08,
                    json!({
                        "ast": {
                            "type": "Assignment",
                            "children": [
                                {"type": "Name", "value": "x", "children": []},
                                {"type": "Number", "value": "1", "children": []}
                            ]
                        }
                    }),
                ),
                StageReport::new(
                    "compiler",
                    "Bytecode Compilation",
                    "AST with 5 nodes",
                    "5 instructions",
                    0.05,
                    json!({
                        "instructions": [
                            {"index": 0, "opcode": "LOAD_CONST", "arg": "1", "stack_effect": "→ 1"},
                            {"index": 1, "opcode": "HALT", "arg": null, "stack_effect": ""}
                        ]
                    }),
                ),
                StageReport::new(
                    "vm",
                    "VM Execution",
                    "5 instructions",
                    "2 steps",
                    0.03,
                    json!({
                        "steps": [
                            {"index": 0, "instruction": "LOAD_CONST 1", "stack_before": [], "stack_after": [1], "variables": {}},
                            {"index": 1, "instruction": "HALT", "stack_before": [1], "stack_after": [1], "variables": {"x": 1}}
                        ]
                    }),
                ),
            ],
        }
    }

    fn riscv_report() -> PipelineReport {
        PipelineReport {
            source: "x = 1 + 2".to_string(),
            language: "python".to_string(),
            target: "riscv".to_string(),
            metadata: metadata(),
            stages: vec![
                StageReport::new(
                    "assembler",
                    "Assembly",
                    "AST",
                    "4 lines",
                    0.04,
                    json!({
                        "lines": [
                            {"address": 0, "assembly": "addi x1, x0, 1", "binary": "0x00100093"},
                            {"address": 4, "assembly": "ecall", "binary": "0x00000073"}
                        ]
                    }),
                ),
                StageReport::new(
                    "riscv",
                    "RISC-V Execution",
                    "4 lines",
                    "2 steps",
                    0.07,
                    json!({
                        "steps": [
                            {"address": 0, "instruction": "addi x1, x0, 1", "registers_changed": {"x1": 1}, "registers": {"x1": 1}},
                            {"address": 4, "instruction": "ecall", "registers_changed": {}, "registers": {"x1": 1}}
                        ]
                    }),
                ),
                StageReport::new(
                    "alu",
                    "ALU Operations",
                    "2 steps",
                    "1 operation",
                    0.02,
                    json!({
                        "operations": [
                            {"op": "ADD", "a": 1, "b": 2, "result": 3}
                        ]
                    }),
                ),
                StageReport::new(
                    "gates",
                    "Gate Operations",
                    "1 operation",
                    "2 gates",
                    0.02,
                    json!({
                        "operations": [
                            {"description": "Full adder bit 0", "gates": [
                                {"gate": "XOR", "label": "A XOR B", "output": 1},
                                {"gate": "AND", "label": "A AND B", "output": 0}
                            ]}
                        ]
                    }),
                ),
            ],
        }
    }

    #[test]
    fn render_vm_report_contains_expected_sections() {
        let renderer = HTMLRenderer::new();
        let html = renderer.render(&vm_report()).expect("render should succeed");

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Computing Stack Report"));
        assert!(html.contains("id=\"lexer\""));
        assert!(html.contains("id=\"parser\""));
        assert!(html.contains("id=\"compiler\""));
        assert!(html.contains("id=\"vm\""));
        assert!(!html.contains("id=\"assembler\""));
    }

    #[test]
    fn render_hardware_report_contains_hardware_sections() {
        let renderer = HTMLRenderer::new();
        let html = renderer.render(&riscv_report()).expect("render should succeed");

        assert!(html.contains("id=\"assembler\""));
        assert!(html.contains("id=\"riscv\""));
        assert!(html.contains("id=\"alu\""));
        assert!(html.contains("id=\"gates\""));
        assert!(!html.contains("id=\"vm\""));
    }

    #[test]
    fn escapes_source_code_and_stage_data() {
        let renderer = HTMLRenderer::new();
        let mut report = vm_report();
        report.source = "<script>alert('xss')</script>".to_string();
        report.stages.push(StageReport::new(
            "quantum",
            "Quantum",
            "input",
            "output",
            1.0,
            json!({"qubits": ["<unsafe>"]}),
        ));

        let html = renderer.render(&report).expect("render should succeed");
        assert!(html.contains("&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;"));
        assert!(!html.contains("<script>alert('xss')</script>"));
        assert!(html.contains("&lt;unsafe&gt;"));
    }

    #[test]
    fn render_from_json_and_render_to_file_round_trip() {
        let renderer = HTMLRenderer::new();
        let report = vm_report();

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let base = std::env::temp_dir().join(format!("html-renderer-{unique}"));
        fs::create_dir_all(&base).expect("temp dir should be creatable");

        let json_path: PathBuf = base.join("report.json");
        let html_path: PathBuf = base.join("report.html");

        fs::write(
            &json_path,
            serde_json::to_string(&report).expect("report should serialize"),
        )
        .expect("json file should be writable");

        let from_json = renderer
            .render_from_json(&json_path)
            .expect("render_from_json should succeed");
        assert!(from_json.contains("Computing Stack Report"));

        renderer
            .render_to_file(&report, &html_path)
            .expect("render_to_file should succeed");
        let written = fs::read_to_string(&html_path).expect("html file should exist");
        assert!(written.contains("VM Execution"));
    }
}
