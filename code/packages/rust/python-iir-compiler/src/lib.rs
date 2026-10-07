//! Native Python grammar-tree to InterpreterIR lowering for LANG79's float pilot.
//!
//! The source is parsed by the Rust Python parser and executed by `vm-core`.
//! Semantic IR and a host Python runtime are not part of the execution path.

use coding_adventures_python_parser::parse_python;
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::sync::{Arc, Mutex};
use vm_core::{errors::VMError, value::Value, VMCore};

/// Bounds for the initial native frontend and its generated VM program.
pub const MAX_SOURCE_BYTES: usize = 64 * 1024;
const MAX_AST_ITEMS: usize = 16_384;
const MAX_AST_DEPTH: usize = 64;
const MAX_AST_TEXT_BYTES: usize = 1024 * 1024;
const MAX_AST_FIELD_BYTES: usize = 64 * 1024;

/// Parse Python 3.12 source and lower the supported subset directly to IIR.
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "Python source exceeds the {MAX_SOURCE_BYTES}-byte native pilot limit"
        ));
    }
    let tree = parse_python(source, "3.12")?;
    compile_ast(&tree, module_name)
}

/// Lower a Python parser tree to IIR, rejecting all syntax outside LANG79.
pub fn compile_ast(tree: &GrammarASTNode, module_name: &str) -> Result<IIRModule, String> {
    if module_name.len() > MAX_AST_FIELD_BYTES {
        return Err("Python IIR module name exceeds the native pilot limit".into());
    }
    check_ast_budget(tree)?;
    if tree.rule_name != "file" {
        return Err("expected a Python file grammar root".into());
    }
    let mut compiler = Compiler::default();
    for child in &tree.children {
        match child {
            ASTNodeOrToken::Node(node) if node.rule_name == "statement" => {
                compiler.compile_statement(node)?;
            }
            ASTNodeOrToken::Token(token) if token.value == "\\n" => {}
            _ => return Err("unsupported Python file item in native float pilot".into()),
        }
    }
    compiler.emit("ret_void", None, vec![], "void");
    let mut module = IIRModule::new(module_name, "python");
    module.entry_point = Some("main".into());
    module.add_or_replace(IIRFunction::new(
        "main",
        vec![],
        "void",
        compiler.instructions,
    ));
    let errors = module.validate();
    if !errors.is_empty() {
        return Err(format!("invalid Python IIR: {}", errors.join("; ")));
    }
    Ok(module)
}

// Callers may pass their own grammar tree, so check it iteratively before the
// recursive lowering path. Count tokens as well as nodes to bound wide input.
fn check_ast_budget(tree: &GrammarASTNode) -> Result<(), String> {
    let mut pending = vec![(tree, 1_usize)];
    let mut seen = 1_usize;
    let mut text_bytes = 0_usize;
    while let Some((node, depth)) = pending.pop() {
        if depth > MAX_AST_DEPTH {
            return Err("Python AST exceeds the native pilot depth limit".into());
        }
        count_ast_text(&node.rule_name, &mut text_bytes)?;
        for child in &node.children {
            if seen == MAX_AST_ITEMS {
                return Err("Python AST exceeds the native pilot item limit".into());
            }
            seen += 1;
            match child {
                ASTNodeOrToken::Node(inner) => {
                    if depth == MAX_AST_DEPTH {
                        return Err("Python AST exceeds the native pilot depth limit".into());
                    }
                    pending.push((inner, depth + 1));
                }
                ASTNodeOrToken::Token(token) => {
                    count_ast_text(&token.value, &mut text_bytes)?;
                    if let Some(type_name) = &token.type_name {
                        count_ast_text(type_name, &mut text_bytes)?;
                    }
                    if let Some(cv) = &token.cv {
                        count_ast_text(cv, &mut text_bytes)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn count_ast_text(field: &str, total: &mut usize) -> Result<(), String> {
    if field.len() > MAX_AST_FIELD_BYTES
        || total
            .checked_add(field.len())
            .is_none_or(|next| next > MAX_AST_TEXT_BYTES)
    {
        return Err("Python AST exceeds the native pilot text limit".into());
    }
    *total += field.len();
    Ok(())
}

/// VM failure with stdout already emitted by earlier Python statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonRunError {
    pub output: String,
    pub message: String,
}

impl std::fmt::Display for PythonRunError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PythonRunError {}

/// Execute Python source on the Rust VM and return its captured `print` output.
pub fn run_source(source: &str) -> Result<String, PythonRunError> {
    let mut module = compile_source(source, "python-script").map_err(|message| PythonRunError {
        output: String::new(),
        message,
    })?;
    let output = Arc::new(Mutex::new(String::new()));
    let captured = Arc::clone(&output);
    let mut vm = VMCore::new();
    vm.max_instructions = Some(100_000);
    vm.builtins_mut().register("py_float_div", |args| {
        let [Value::Float(left), Value::Float(right)] = args else {
            return Err(VMError::Custom("py_float_div expects two floats".into()));
        };
        if *right == 0.0 {
            return Err(VMError::Custom(
                "ZeroDivisionError: float division by zero".into(),
            ));
        }
        Ok(Value::Float(*left / *right))
    });
    vm.builtins_mut().register("py_print_float", move |args| {
        let [Value::Float(number)] = args else {
            return Err(VMError::Custom("py_print_float expects one float".into()));
        };
        let displayed = format_python_float(*number).map_err(VMError::Custom)?;
        let mut sink = captured
            .lock()
            .map_err(|_| VMError::Custom("Python output lock poisoned".into()))?;
        if sink.len() + displayed.len() + 1 > 1_000_000 {
            return Err(VMError::Custom("Python output limit exceeded".into()));
        }
        sink.push_str(&displayed);
        sink.push('\n');
        Ok(Value::Null)
    });
    let execution = vm.execute(&mut module, "main", &[]);
    let result = output
        .lock()
        .map_err(|_| PythonRunError {
            output: String::new(),
            message: "Python output lock poisoned".into(),
        })?
        .clone();
    match execution {
        Ok(_) => Ok(result),
        Err(error) => Err(PythonRunError {
            output: result,
            message: error.to_string(),
        }),
    }
}

fn format_python_float(number: f64) -> Result<String, String> {
    if !number.is_finite() || (number != 0.0 && !(1e-4..=1e15).contains(&number.abs())) {
        return Err("Python float display outside the native pilot's supported range".into());
    }
    let mut displayed = number.to_string();
    if !displayed.contains('.') && !displayed.contains('e') {
        displayed.push_str(".0");
    }
    Ok(displayed)
}

#[derive(Default)]
struct Compiler {
    instructions: Vec<IIRInstr>,
    next_register: usize,
}

impl Compiler {
    fn emit(&mut self, op: &str, dest: Option<String>, srcs: Vec<Operand>, type_hint: &str) {
        self.instructions
            .push(IIRInstr::new(op, dest, srcs, type_hint));
    }

    fn register(&mut self) -> String {
        let name = format!("py{}", self.next_register);
        self.next_register += 1;
        name
    }

    fn compile_statement(&mut self, statement: &GrammarASTNode) -> Result<(), String> {
        let simple = only_node(statement, "simple_stmt")?;
        if simple.children.len() != 2 {
            return Err("unsupported Python statement separator".into());
        }
        let ASTNodeOrToken::Node(small) = &simple.children[0] else {
            return Err("unsupported Python statement".into());
        };
        if small.rule_name != "small_stmt" {
            return Err("unsupported Python statement".into());
        }
        let ASTNodeOrToken::Token(newline) = &simple.children[1] else {
            return Err("Python statement requires a newline".into());
        };
        if newline.value != "\\n" {
            return Err("unsupported Python statement separator".into());
        }
        let assign = only_node(small, "assign_stmt")?;
        let expression_list = only_node(assign, "expression_list")?;
        let expression = only_node(expression_list, "expression")?;
        if let Some(argument) = print_argument(expression)? {
            let value = self.compile_number(argument)?;
            self.emit(
                "call_builtin",
                None,
                vec![Operand::Var("py_print_float".into()), value],
                "void",
            );
        } else {
            self.compile_number(expression)?;
        }
        Ok(())
    }

    fn compile_number(&mut self, node: &GrammarASTNode) -> Result<Operand, String> {
        match node.rule_name.as_str() {
            "arith" => self.compile_chain(node, "term", &[('+', "add"), ('-', "sub")]),
            "term" => self.compile_chain(node, "factor", &[('*', "mul"), ('/', "py_float_div")]),
            "factor" if node.children.len() == 2 => {
                let (ASTNodeOrToken::Token(sign), ASTNodeOrToken::Node(inner)) =
                    (&node.children[0], &node.children[1])
                else {
                    return Err("unsupported Python unary expression".into());
                };
                if inner.rule_name != "factor" {
                    return Err("unsupported Python unary expression".into());
                }
                let value = self.compile_number(inner)?;
                match sign.value.as_str() {
                    "+" => Ok(value),
                    "-" => {
                        let dest = self.register();
                        self.emit("neg", Some(dest.clone()), vec![value], "f64");
                        Ok(Operand::Var(dest))
                    }
                    _ => Err("unsupported Python unary operator".into()),
                }
            }
            "atom" => self.compile_atom(node),
            "paren_expr" => {
                if node.children.len() != 3 {
                    return Err("unsupported Python parenthesized expression".into());
                }
                let ASTNodeOrToken::Node(body) = &node.children[1] else {
                    return Err("unsupported Python parenthesized expression".into());
                };
                self.compile_number(only_node(body, "expression")?)
            }
            _ => self.compile_number(only_child(node)?),
        }
    }

    fn compile_chain(
        &mut self,
        node: &GrammarASTNode,
        operand_rule: &str,
        operators: &[(char, &str)],
    ) -> Result<Operand, String> {
        let Some(ASTNodeOrToken::Node(first)) = node.children.first() else {
            return Err("empty Python arithmetic expression".into());
        };
        if first.rule_name != operand_rule {
            return Err("unsupported Python arithmetic expression".into());
        }
        let mut left = self.compile_number(first)?;
        let mut rest = node.children[1..].chunks_exact(2);
        for pair in &mut rest {
            let (ASTNodeOrToken::Token(operator), ASTNodeOrToken::Node(right_node)) =
                (&pair[0], &pair[1])
            else {
                return Err("unsupported Python arithmetic expression".into());
            };
            if right_node.rule_name != operand_rule {
                return Err("unsupported Python arithmetic expression".into());
            }
            let Some((_, op)) = operators
                .iter()
                .find(|(symbol, _)| operator.value == symbol.to_string())
            else {
                return Err(format!("unsupported Python operator {}", operator.value));
            };
            let right = self.compile_number(right_node)?;
            let dest = self.register();
            if *op == "py_float_div" {
                self.emit(
                    "call_builtin",
                    Some(dest.clone()),
                    vec![Operand::Var((*op).into()), left, right],
                    "f64",
                );
            } else {
                self.emit(op, Some(dest.clone()), vec![left, right], "f64");
            }
            left = Operand::Var(dest);
        }
        if !rest.remainder().is_empty() {
            return Err("incomplete Python arithmetic expression".into());
        }
        Ok(left)
    }

    fn compile_atom(&mut self, node: &GrammarASTNode) -> Result<Operand, String> {
        if node.children.len() != 1 {
            return Err("unsupported Python atom".into());
        }
        match &node.children[0] {
            ASTNodeOrToken::Token(token) if token.type_name.as_deref() == Some("FLOAT") => {
                let spelling = token.value.replace('_', "");
                let value = spelling
                    .parse::<f64>()
                    .map_err(|_| format!("invalid Python float literal {}", token.value))?;
                let dest = self.register();
                self.emit(
                    "const",
                    Some(dest.clone()),
                    vec![Operand::Float(value)],
                    "f64",
                );
                Ok(Operand::Var(dest))
            }
            ASTNodeOrToken::Token(token) if token.type_name.as_deref() == Some("INT") => {
                Err("Python int requires arbitrary-precision runtime support".into())
            }
            ASTNodeOrToken::Node(paren) if paren.rule_name == "paren_expr" => {
                self.compile_number(paren)
            }
            _ => Err("unsupported Python atom in native float pilot".into()),
        }
    }
}

fn only_child(node: &GrammarASTNode) -> Result<&GrammarASTNode, String> {
    let [ASTNodeOrToken::Node(inner)] = node.children.as_slice() else {
        return Err(format!("unsupported Python {} syntax", node.rule_name));
    };
    Ok(inner)
}

fn only_node<'a>(node: &'a GrammarASTNode, rule: &str) -> Result<&'a GrammarASTNode, String> {
    let inner = only_child(node)?;
    if inner.rule_name != rule {
        return Err(format!("unsupported Python {} syntax", node.rule_name));
    }
    Ok(inner)
}

fn print_argument(expression: &GrammarASTNode) -> Result<Option<&GrammarASTNode>, String> {
    let mut node = expression;
    while node.rule_name != "primary" {
        let Ok(inner) = only_child(node) else {
            return Ok(None);
        };
        node = inner;
    }
    let [ASTNodeOrToken::Node(atom), ASTNodeOrToken::Node(suffix)] = node.children.as_slice()
    else {
        return Ok(None);
    };
    let [ASTNodeOrToken::Token(callee)] = atom.children.as_slice() else {
        return Ok(None);
    };
    if callee.value != "print" || suffix.rule_name != "suffix" {
        return Ok(None);
    }
    let [ASTNodeOrToken::Token(open), ASTNodeOrToken::Node(args), ASTNodeOrToken::Token(close)] =
        suffix.children.as_slice()
    else {
        return Err("native Python print requires exactly one argument".into());
    };
    if open.value != "(" || close.value != ")" {
        return Err("native Python print requires a call".into());
    }
    let argument = only_node(args, "argument")?;
    Ok(Some(only_node(argument, "expression")?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_executes_float_arithmetic_on_native_vm() {
        assert_eq!(run_source("print(1.0 + 2.0)\n").unwrap(), "3.0\n");
        assert_eq!(run_source("print(1.0 / 2.0)\n").unwrap(), "0.5\n");
        assert_eq!(run_source("print(-0.0)\n").unwrap(), "-0.0\n");
        assert_eq!(
            run_source("print(1.0 - 2.0 * 3.0)\nprint((1.0 + 2.0) * 3.0)\n").unwrap(),
            "-5.0\n9.0\n"
        );
        assert_eq!(run_source("print(1_0.0 / 4.0)\n").unwrap(), "2.5\n");
    }

    #[test]
    fn unsupported_python_semantics_are_rejected() {
        for source in [
            "print(1 + 2)\n",
            "print(1.0 // 2.0)\n",
            "x = 1.0\n",
            "print(1.0, 2.0)\n",
        ] {
            assert!(compile_source(source, "negative").is_err(), "{source}");
        }
        assert!(run_source("print(1.0 / 0.0)\n")
            .unwrap_err()
            .message
            .contains("ZeroDivisionError"));
    }

    fn empty_node(rule_name: &str) -> GrammarASTNode {
        GrammarASTNode {
            rule_name: rule_name.into(),
            children: vec![],
            start_line: None,
            start_column: None,
            end_line: None,
            end_column: None,
        }
    }

    #[test]
    fn source_and_direct_ast_budgets_reject_oversized_inputs() {
        let source = " ".repeat(MAX_SOURCE_BYTES + 1);
        assert!(compile_source(&source, "oversized")
            .unwrap_err()
            .contains("source exceeds"));

        let mut wide = empty_node("file");
        wide.children = (0..MAX_AST_ITEMS)
            .map(|_| ASTNodeOrToken::Node(empty_node("statement")))
            .collect();
        assert!(compile_ast(&wide, "wide")
            .unwrap_err()
            .contains("item limit"));

        let mut deep = empty_node("leaf");
        for _ in 0..MAX_AST_DEPTH {
            let mut parent = empty_node("wrapper");
            parent.children.push(ASTNodeOrToken::Node(deep));
            deep = parent;
        }
        deep.rule_name = "file".into();
        assert!(compile_ast(&deep, "deep")
            .unwrap_err()
            .contains("depth limit"));
    }

    #[test]
    fn direct_ast_text_and_module_names_are_bounded_before_lowering() {
        fn enlarge_float_token(node: &mut GrammarASTNode, cv: bool) -> bool {
            for child in &mut node.children {
                match child {
                    ASTNodeOrToken::Token(token) if token.type_name.as_deref() == Some("FLOAT") => {
                        if cv {
                            token.cv = Some("c".repeat(MAX_AST_FIELD_BYTES + 1));
                        } else {
                            token.value = "9".repeat(MAX_AST_FIELD_BYTES + 1);
                        }
                        return true;
                    }
                    ASTNodeOrToken::Node(inner) => {
                        if enlarge_float_token(inner, cv) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            false
        }

        let mut oversized_token = parse_python("print(1.0)\n", "3.12").unwrap();
        assert!(enlarge_float_token(&mut oversized_token, false));
        assert!(compile_ast(&oversized_token, "test")
            .unwrap_err()
            .contains("text limit"));

        let mut oversized_cv = parse_python("print(1.0)\n", "3.12").unwrap();
        assert!(enlarge_float_token(&mut oversized_cv, true));
        assert!(compile_ast(&oversized_cv, "test")
            .unwrap_err()
            .contains("text limit"));

        let mut oversized_name = empty_node("file");
        oversized_name.rule_name = "x".repeat(64 * 1024 + 1);
        assert!(compile_ast(&oversized_name, "test")
            .unwrap_err()
            .contains("text limit"));

        let mut aggregate = empty_node("file");
        aggregate.children = (0..18)
            .map(|_| {
                let mut node = empty_node("child");
                node.rule_name = "x".repeat(60_000);
                ASTNodeOrToken::Node(node)
            })
            .collect();
        assert!(compile_ast(&aggregate, "test")
            .unwrap_err()
            .contains("text limit"));

        assert!(compile_ast(&empty_node("file"), &"m".repeat(64 * 1024 + 1))
            .unwrap_err()
            .contains("module name"));
    }

    #[test]
    fn runtime_error_retains_prior_print_output() {
        let error = run_source("print(1.0)\nprint(1.0 / 0.0)\n").unwrap_err();
        assert_eq!(error.output, "1.0\n");
        assert!(error.message.contains("ZeroDivisionError"));
    }
}
