//! Ruby's bounded integer-expression grammar tree lowered directly to IIR.
//!
//! The compiler follows Ruby's existing parser tree. It proves every value
//! fits in `i64` before emitting arithmetic, so accepted expressions agree
//! with Ruby even though Ruby's full integer tower is not implemented yet.

use coding_adventures_ruby_parser::create_ruby_parser;
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::sync::{Arc, Mutex};
use vm_core::{errors::VMError, value::Value, VMCore};

/// Maximum source size accepted by the first native Ruby pilot.
pub const MAX_SOURCE_BYTES: usize = 64 * 1024;
const MAX_AST_ITEMS: usize = 16_384;
const MAX_AST_DEPTH: usize = 256;
const MAX_AST_TEXT_BYTES: usize = 1024 * 1024;
const MAX_AST_FIELD_BYTES: usize = 64 * 1024;
const MAX_OUTPUT_BYTES: usize = 1_000_000;

/// Parse Ruby 3.0 source and lower the supported subset directly to IIR.
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err("Ruby source exceeds the native pilot limit".into());
    }
    let mut parser = create_ruby_parser(source);
    let ast = parser.parse().map_err(|error| error.to_string())?;
    compile_ast(&ast, module_name)
}

/// Lower a Ruby parser tree without passing through Semantic IR.
pub fn compile_ast(ast: &GrammarASTNode, module_name: &str) -> Result<IIRModule, String> {
    if module_name.len() > MAX_AST_FIELD_BYTES {
        return Err("Ruby IIR module name exceeds the native pilot limit".into());
    }
    check_ast_budget(ast)?;
    if ast.rule_name != "program" || ast.children.is_empty() {
        return Err("expected Ruby statements in a program".into());
    }
    let mut compiler = Compiler::default();
    for child in &ast.children {
        let ASTNodeOrToken::Node(statement) = child else {
            return Err("unsupported Ruby program item".into());
        };
        let call = only_node(statement, "method_call")?;
        if call.children.len() != 4
            || token_value(&call.children[0]) != Some("puts")
            || token_value(&call.children[1]) != Some("(")
            || token_value(&call.children[3]) != Some(")")
        {
            return Err("native Ruby pilot requires puts(one expression)".into());
        }
        let ASTNodeOrToken::Node(argument) = &call.children[2] else {
            return Err("unsupported Ruby puts argument".into());
        };
        let expression = only_node(argument, "expression")?;
        let compiled = compiler.compile_expression(expression)?;
        compiler.emit(
            "call_builtin",
            None,
            vec![Operand::Var("rb_puts_int".into()), compiled.operand],
            "void",
        );
    }
    compiler.emit("ret_void", None, vec![], "void");
    let mut module = IIRModule::new(module_name, "ruby");
    module.entry_point = Some("main".into());
    module.add_or_replace(IIRFunction::new(
        "main",
        vec![],
        "void",
        compiler.instructions,
    ));
    let errors = module.validate();
    if !errors.is_empty() {
        return Err(format!("invalid Ruby IIR: {}", errors.join("; ")));
    }
    Ok(module)
}

/// Execute Ruby source on the repository's Rust VM and capture `puts` output.
pub fn run_source(source: &str) -> Result<String, String> {
    let mut module = compile_source(source, "ruby-script")?;
    let output = Arc::new(Mutex::new(String::new()));
    let captured = Arc::clone(&output);
    let mut vm = VMCore::new();
    vm.max_instructions = Some(100_000);
    vm.builtins_mut().register("rb_int_div", |args| {
        let [Value::Int(left), Value::Int(right)] = args else {
            return Err(VMError::Custom("rb_int_div expects two integers".into()));
        };
        if *right == 0 {
            return Err(VMError::Custom("ZeroDivisionError: divided by 0".into()));
        }
        let quotient = floor_div(i128::from(*left), i128::from(*right));
        let value = i64::try_from(quotient)
            .map_err(|_| VMError::Custom("Ruby integer exceeds native pilot range".into()))?;
        Ok(Value::Int(value))
    });
    vm.builtins_mut().register("rb_puts_int", move |args| {
        let [Value::Int(number)] = args else {
            return Err(VMError::Custom("rb_puts_int expects one integer".into()));
        };
        let rendered = number.to_string();
        let mut sink = captured
            .lock()
            .map_err(|_| VMError::Custom("Ruby output lock poisoned".into()))?;
        if sink
            .len()
            .checked_add(rendered.len() + 1)
            .is_none_or(|size| size > MAX_OUTPUT_BYTES)
        {
            return Err(VMError::Custom("Ruby output limit exceeded".into()));
        }
        sink.push_str(&rendered);
        sink.push('\n');
        Ok(Value::Null)
    });
    vm.execute(&mut module, "main", &[])
        .map_err(|error| error.to_string())?;
    let result = output
        .lock()
        .map_err(|_| "Ruby output lock poisoned")?
        .clone();
    Ok(result)
}

fn floor_div(left: i128, right: i128) -> i128 {
    let quotient = left / right;
    if left % right != 0 && (left < 0) != (right < 0) {
        quotient - 1
    } else {
        quotient
    }
}

fn check_ast_budget(root: &GrammarASTNode) -> Result<(), String> {
    let mut pending = vec![(root, 1_usize)];
    let mut visited = 1_usize;
    let mut text_bytes = 0_usize;
    while let Some((node, depth)) = pending.pop() {
        if depth > MAX_AST_DEPTH {
            return Err("Ruby AST exceeds the native pilot depth limit".into());
        }
        count_ast_text(&node.rule_name, &mut text_bytes)?;
        for child in &node.children {
            if visited == MAX_AST_ITEMS {
                return Err("Ruby AST exceeds the native pilot AST item limit".into());
            }
            visited += 1;
            match child {
                ASTNodeOrToken::Node(inner) => {
                    if depth == MAX_AST_DEPTH {
                        return Err("Ruby AST exceeds the native pilot depth limit".into());
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
        return Err("Ruby AST exceeds the native pilot AST text limit".into());
    }
    *total += field.len();
    Ok(())
}

fn token_value(child: &ASTNodeOrToken) -> Option<&str> {
    match child {
        ASTNodeOrToken::Token(token) => Some(&token.value),
        ASTNodeOrToken::Node(_) => None,
    }
}

fn only_node<'a>(parent: &'a GrammarASTNode, rule: &str) -> Result<&'a GrammarASTNode, String> {
    let [ASTNodeOrToken::Node(child)] = parent.children.as_slice() else {
        return Err(format!("unsupported Ruby {} shape", parent.rule_name));
    };
    if child.rule_name != rule {
        return Err(format!("unsupported Ruby {} shape", parent.rule_name));
    }
    Ok(child)
}

struct Compiled {
    operand: Operand,
    value: i128,
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
        let result = format!("rb{}", self.next_register);
        self.next_register += 1;
        result
    }

    fn compile_expression(&mut self, node: &GrammarASTNode) -> Result<Compiled, String> {
        match node.rule_name.as_str() {
            "expression" => self.compile_expression(only_node(node, "ternary")?),
            "ternary" => self.compile_expression(only_node(node, "range")?),
            "range" => self.compile_expression(only_node(node, "logical_or")?),
            "logical_or" => self.compile_expression(only_node(node, "logical_and")?),
            "logical_and" => self.compile_expression(only_node(node, "logical_not")?),
            "logical_not" => self.compile_expression(only_node(node, "comparison")?),
            "comparison" => self.compile_expression(only_node(node, "shift")?),
            "shift" => self.compile_expression(only_node(node, "sum")?),
            "sum" | "term" => self.compile_chain(node),
            "factor" => self.compile_factor(node),
            "unary_minus" => {
                let [sign, ASTNodeOrToken::Node(factor)] = node.children.as_slice() else {
                    return Err("unsupported Ruby unary expression".into());
                };
                if token_value(sign) != Some("-") || factor.rule_name != "factor" {
                    return Err("unsupported Ruby unary expression".into());
                }
                let inner = self.compile_expression(factor)?;
                let value = checked_i64(-inner.value)?;
                let dest = self.register();
                self.emit("neg", Some(dest.clone()), vec![inner.operand], "i64");
                Ok(Compiled {
                    operand: Operand::Var(dest),
                    value,
                })
            }
            _ => Err(format!("unsupported Ruby {} expression", node.rule_name)),
        }
    }

    fn compile_factor(&mut self, node: &GrammarASTNode) -> Result<Compiled, String> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Token(token)] if token.value.bytes().all(|c| c.is_ascii_digit()) => {
                let value = token
                    .value
                    .parse::<i128>()
                    .map_err(|_| "Ruby integer exceeds native pilot range")?;
                let value = checked_i64(value)?;
                let dest = self.register();
                self.emit(
                    "const",
                    Some(dest.clone()),
                    vec![Operand::Int(value as i64)],
                    "i64",
                );
                Ok(Compiled {
                    operand: Operand::Var(dest),
                    value,
                })
            }
            [ASTNodeOrToken::Node(unary)] if unary.rule_name == "unary_minus" => {
                self.compile_expression(unary)
            }
            [open, ASTNodeOrToken::Node(inner), close]
                if token_value(open) == Some("(")
                    && token_value(close) == Some(")")
                    && inner.rule_name == "expression" =>
            {
                self.compile_expression(inner)
            }
            _ => Err("unsupported Ruby factor in native integer pilot".into()),
        }
    }

    fn compile_chain(&mut self, node: &GrammarASTNode) -> Result<Compiled, String> {
        let expected_child = if node.rule_name == "sum" {
            "term"
        } else {
            "factor"
        };
        let Some(ASTNodeOrToken::Node(first)) = node.children.first() else {
            return Err("Ruby arithmetic chain has no operand".into());
        };
        if first.rule_name != expected_child {
            return Err("unsupported Ruby arithmetic chain".into());
        }
        let mut acc = self.compile_expression(first)?;
        let (pairs, remainder) = node.children[1..].as_chunks::<2>();
        for pair in pairs {
            let [op_token, ASTNodeOrToken::Node(right)] = pair else {
                return Err("unsupported Ruby arithmetic chain".into());
            };
            if right.rule_name != expected_child {
                return Err("unsupported Ruby arithmetic chain".into());
            }
            let op = token_value(op_token).ok_or("missing Ruby arithmetic operator")?;
            let rhs = self.compile_expression(right)?;
            let (iir_op, value) = match op {
                "+" if node.rule_name == "sum" => ("add", acc.value + rhs.value),
                "-" if node.rule_name == "sum" => ("sub", acc.value - rhs.value),
                "*" if node.rule_name == "term" => ("mul", acc.value * rhs.value),
                "/" if node.rule_name == "term" && rhs.value != 0 => {
                    ("call_builtin", floor_div(acc.value, rhs.value))
                }
                "/" if node.rule_name == "term" => {
                    return Err("ZeroDivisionError: divided by 0".into());
                }
                _ => return Err("unsupported Ruby arithmetic operator".into()),
            };
            let value = checked_i64(value)?;
            let dest = self.register();
            let mut srcs = vec![acc.operand, rhs.operand];
            if iir_op == "call_builtin" {
                srcs.insert(0, Operand::Var("rb_int_div".into()));
            }
            self.emit(iir_op, Some(dest.clone()), srcs, "i64");
            acc = Compiled {
                operand: Operand::Var(dest),
                value,
            };
        }
        if !remainder.is_empty() {
            return Err("unsupported Ruby arithmetic chain".into());
        }
        Ok(acc)
    }
}

fn checked_i64(value: i128) -> Result<i128, String> {
    i64::try_from(value)
        .map(|_| value)
        .map_err(|_| "Ruby integer exceeds native pilot range".into())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn number_token(node: &GrammarASTNode) -> Option<ASTNodeOrToken> {
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(token) if token.value == "1" => return Some(child.clone()),
                ASTNodeOrToken::Node(inner) => {
                    if let Some(token) = number_token(inner) {
                        return Some(token);
                    }
                }
                _ => {}
            }
        }
        None
    }

    #[test]
    fn executes_ruby_integer_expressions_on_our_vm() {
        assert_eq!(
            run_source("puts(1 + 2)\nputs(7 / 2)\nputs(-7 / 2)").unwrap(),
            "3\n3\n-4\n"
        );
        assert_eq!(run_source("puts((1 + 2) * 3)").unwrap(), "9\n");
        assert_eq!(run_source("puts(7 / -2)").unwrap(), "-4\n");
    }

    #[test]
    fn emitted_ir_executes_arithmetic_and_ruby_builtins() {
        let module = compile_source("puts(7 / 2 + 1)", "ir").unwrap();
        let ops: Vec<_> = module.functions[0]
            .instructions
            .iter()
            .map(|instr| instr.op.as_str())
            .collect();
        assert!(ops.contains(&"add"));
        assert_eq!(ops.iter().filter(|op| **op == "call_builtin").count(), 2);
    }

    #[test]
    fn unsupported_and_out_of_range_inputs_are_rejected() {
        for source in [
            "puts(x)",
            "puts(1, 2)",
            "x = 1",
            "puts('hello')",
            "puts(9223372036854775808)",
            "puts(9223372036854775807 + 1)",
            "puts(1 / 0)",
            "puts(-9223372036854775807 - 2)",
        ] {
            assert!(run_source(source).is_err(), "{source}");
        }
        assert!(compile_source(&" ".repeat(MAX_SOURCE_BYTES + 1), "oversized").is_err());
    }

    #[test]
    fn direct_ast_item_and_text_limits_apply_before_lowering() {
        let mut parser = create_ruby_parser("puts(1)");
        let parsed = parser.parse().unwrap();
        let token = number_token(&parsed).unwrap();

        let mut too_many_tokens = empty_node("program");
        too_many_tokens.children = vec![token.clone(); MAX_AST_ITEMS];
        assert!(compile_ast(&too_many_tokens, "test")
            .unwrap_err()
            .contains("AST item limit"));

        let mut huge_token = token.clone();
        if let ASTNodeOrToken::Token(value) = &mut huge_token {
            value.value = "9".repeat(64 * 1024 + 1);
        }
        let mut oversized_text = empty_node("program");
        oversized_text.children.push(huge_token);
        assert!(compile_ast(&oversized_text, "test")
            .unwrap_err()
            .contains("AST text limit"));

        let mut huge_cv = token;
        if let ASTNodeOrToken::Token(value) = &mut huge_cv {
            value.cv = Some("c".repeat(64 * 1024 + 1));
        }
        let mut oversized_cv = empty_node("program");
        oversized_cv.children.push(huge_cv);
        assert!(compile_ast(&oversized_cv, "test")
            .unwrap_err()
            .contains("AST text limit"));

        let mut aggregate = empty_node("program");
        aggregate.children = (0..18)
            .map(|_| {
                let mut node = empty_node("child");
                node.rule_name = "r".repeat(60_000);
                ASTNodeOrToken::Node(node)
            })
            .collect();
        assert!(compile_ast(&aggregate, "test")
            .unwrap_err()
            .contains("AST text limit"));

        assert!(
            compile_ast(&empty_node("program"), &"m".repeat(64 * 1024 + 1))
                .unwrap_err()
                .contains("module name")
        );
    }
}
