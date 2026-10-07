//! Direct Perl grammar-tree to InterpreterIR lowering for LANG81.
//!
//! The production path uses this repository's Rust parser and VM. Semantic IR
//! and a host Perl interpreter are not stages in that path.

use coding_adventures_perl_parser::parse_perl;
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::sync::{Arc, Mutex};
use vm_core::{errors::VMError, value::Value, VMCore};

const MAX_AST_ITEMS: usize = 16_384;
const MAX_AST_DEPTH: usize = 64;
const MAX_OUTPUT_BYTES: usize = 1_000_000;

/// Parse and compile the bounded Perl 5.38 subset to InterpreterIR.
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, String> {
    let tree = parse_perl(source)?;
    compile_ast(&tree, module_name)
}

/// Compile a Perl parser tree supplied directly by a caller.
pub fn compile_ast(tree: &GrammarASTNode, module_name: &str) -> Result<IIRModule, String> {
    check_ast_budget(tree)?;
    if tree.rule_name != "program" || tree.children.is_empty() {
        return Err("expected a Perl print program".into());
    }
    let mut compiler = Compiler::default();
    for child in &tree.children {
        let ASTNodeOrToken::Node(statement) = child else {
            return Err("unsupported Perl program item".into());
        };
        compiler.compile_print(statement)?;
    }
    compiler.emit("ret_void", None, vec![], "void");
    let mut module = IIRModule::new(module_name, "perl");
    module.entry_point = Some("main".into());
    module.add_or_replace(IIRFunction::new(
        "main",
        vec![],
        "void",
        compiler.instructions,
    ));
    let errors = module.validate();
    if !errors.is_empty() {
        return Err(format!("invalid Perl IIR: {}", errors.join("; ")));
    }
    Ok(module)
}

/// Run Perl source on the native Rust VM and capture its `print` output.
pub fn run_source(source: &str) -> Result<String, String> {
    let mut module = compile_source(source, "perl-script")?;
    let output = Arc::new(Mutex::new(String::new()));
    let captured = Arc::clone(&output);
    let mut vm = VMCore::new();
    vm.max_instructions = Some(100_000);
    vm.builtins_mut().register("pl_print_int", move |args| {
        let [Value::Int(number)] = args else {
            return Err(VMError::Custom("pl_print_int expects one integer".into()));
        };
        let rendered = number.to_string();
        let mut sink = captured
            .lock()
            .map_err(|_| VMError::Custom("Perl output lock poisoned".into()))?;
        if sink
            .len()
            .checked_add(rendered.len())
            .is_none_or(|length| length > MAX_OUTPUT_BYTES)
        {
            return Err(VMError::Custom("Perl output limit exceeded".into()));
        }
        sink.push_str(&rendered);
        Ok(Value::Null)
    });
    vm.execute(&mut module, "main", &[])
        .map_err(|error| error.to_string())?;
    output
        .lock()
        .map_err(|_| "Perl output lock poisoned".to_string())
        .map(|sink| sink.clone())
}

fn check_ast_budget(root: &GrammarASTNode) -> Result<(), String> {
    let mut pending = vec![(root, 1_usize)];
    let mut remaining = MAX_AST_ITEMS;
    while let Some((node, depth)) = pending.pop() {
        if depth > MAX_AST_DEPTH {
            return Err("Perl AST exceeds the native pilot depth limit".into());
        }
        if remaining == 0 {
            return Err("Perl AST exceeds the native pilot item limit".into());
        }
        remaining -= 1;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(inner) => pending.push((inner, depth + 1)),
                ASTNodeOrToken::Token(_) => {
                    if remaining == 0 {
                        return Err("Perl AST exceeds the native pilot item limit".into());
                    }
                    remaining -= 1;
                }
            }
        }
    }
    Ok(())
}

fn token_value(child: &ASTNodeOrToken) -> Option<&str> {
    match child {
        ASTNodeOrToken::Token(token) => Some(&token.value),
        ASTNodeOrToken::Node(_) => None,
    }
}

fn checked_i32(value: i64) -> Result<i32, String> {
    i32::try_from(value).map_err(|_| "Perl integer exceeds native pilot range".into())
}

struct Compiled {
    operand: Operand,
    value: i32,
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
        let name = format!("pl{}", self.next_register);
        self.next_register += 1;
        name
    }

    fn compile_print(&mut self, node: &GrammarASTNode) -> Result<(), String> {
        let [name, open, ASTNodeOrToken::Node(expr), close, semicolon] = node.children.as_slice()
        else {
            return Err("unsupported Perl print statement".into());
        };
        if node.rule_name != "print_statement"
            || token_value(name) != Some("print")
            || token_value(open) != Some("(")
            || expr.rule_name != "expr"
            || token_value(close) != Some(")")
            || token_value(semicolon) != Some(";")
        {
            return Err("native Perl pilot requires print(one expression);".into());
        }
        let value = self.compile_expression(expr)?;
        self.emit(
            "call_builtin",
            None,
            vec![Operand::Var("pl_print_int".into()), value.operand],
            "void",
        );
        Ok(())
    }

    fn compile_expression(&mut self, node: &GrammarASTNode) -> Result<Compiled, String> {
        match node.rule_name.as_str() {
            "expr" | "term" => self.compile_chain(node),
            "unary" => match node.children.as_slice() {
                [ASTNodeOrToken::Node(atom)] if atom.rule_name == "atom" => {
                    self.compile_expression(atom)
                }
                [sign, ASTNodeOrToken::Node(inner)]
                    if token_value(sign) == Some("-") && inner.rule_name == "unary" =>
                {
                    let value = self.compile_expression(inner)?;
                    let checked = checked_i32(-i64::from(value.value))?;
                    let dest = self.register();
                    self.emit("neg", Some(dest.clone()), vec![value.operand], "i64");
                    Ok(Compiled {
                        operand: Operand::Var(dest),
                        value: checked,
                    })
                }
                _ => Err("unsupported Perl unary expression".into()),
            },
            "atom" => match node.children.as_slice() {
                [ASTNodeOrToken::Token(token)] if token.type_name.as_deref() == Some("INT") => {
                    let parsed = token
                        .value
                        .parse::<i64>()
                        .map_err(|_| "Perl integer exceeds native pilot range")?;
                    let value = checked_i32(parsed)?;
                    let dest = self.register();
                    self.emit(
                        "const",
                        Some(dest.clone()),
                        vec![Operand::Int(i64::from(value))],
                        "i64",
                    );
                    Ok(Compiled {
                        operand: Operand::Var(dest),
                        value,
                    })
                }
                [open, ASTNodeOrToken::Node(expr), close]
                    if token_value(open) == Some("(")
                        && expr.rule_name == "expr"
                        && token_value(close) == Some(")") =>
                {
                    self.compile_expression(expr)
                }
                _ => Err("unsupported Perl atom".into()),
            },
            _ => Err(format!("unsupported Perl {} expression", node.rule_name)),
        }
    }

    fn compile_chain(&mut self, node: &GrammarASTNode) -> Result<Compiled, String> {
        let child_rule = if node.rule_name == "expr" {
            "term"
        } else {
            "unary"
        };
        let Some(ASTNodeOrToken::Node(first)) = node.children.first() else {
            return Err("empty Perl arithmetic expression".into());
        };
        if first.rule_name != child_rule {
            return Err("unsupported Perl arithmetic chain".into());
        }
        let mut acc = self.compile_expression(first)?;
        let mut chunks = node.children[1..].chunks_exact(2);
        for pair in &mut chunks {
            let [operator, ASTNodeOrToken::Node(right)] = pair else {
                return Err("unsupported Perl arithmetic chain".into());
            };
            if right.rule_name != child_rule {
                return Err("unsupported Perl arithmetic chain".into());
            }
            let rhs = self.compile_expression(right)?;
            let (opcode, value) = match (node.rule_name.as_str(), token_value(operator)) {
                ("expr", Some("+")) => ("add", i64::from(acc.value) + i64::from(rhs.value)),
                ("expr", Some("-")) => ("sub", i64::from(acc.value) - i64::from(rhs.value)),
                ("term", Some("*")) => ("mul", i64::from(acc.value) * i64::from(rhs.value)),
                _ => return Err("unsupported Perl arithmetic operator".into()),
            };
            let value = checked_i32(value)?;
            let dest = self.register();
            self.emit(
                opcode,
                Some(dest.clone()),
                vec![acc.operand, rhs.operand],
                "i64",
            );
            acc = Compiled {
                operand: Operand::Var(dest),
                value,
            };
        }
        if !chunks.remainder().is_empty() {
            return Err("incomplete Perl arithmetic expression".into());
        }
        Ok(acc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_executes_on_native_vm() {
        assert_eq!(run_source("print(1 + 2);").unwrap(), "3");
        assert_eq!(run_source("print(7 - 2 * 3);").unwrap(), "1");
        assert_eq!(run_source("print(-4); print(2);").unwrap(), "-42");
        assert_eq!(run_source("print((1 + 2) * 3);").unwrap(), "9");
    }

    #[test]
    fn emits_arithmetic_and_print_calls() {
        let module = compile_source("print(1 + 2 * 3);", "ir").unwrap();
        let ops: Vec<_> = module.functions[0]
            .instructions
            .iter()
            .map(|instruction| instruction.op.as_str())
            .collect();
        assert!(ops.contains(&"mul"));
        assert!(ops.contains(&"add"));
        assert!(ops.contains(&"call_builtin"));
    }

    #[test]
    fn rejects_unsupported_or_out_of_range_source() {
        for source in [
            "print(x);",
            "print(1 / 2);",
            "print(1, 2);",
            "print(2147483648);",
            "print(2147483647 + 1);",
            "print(50000 * 50000);",
        ] {
            assert!(compile_source(source, "negative").is_err(), "{source}");
        }
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
    fn direct_ast_budget_rejects_deep_and_wide_input() {
        let mut deep = empty_node("leaf");
        for _ in 0..MAX_AST_DEPTH {
            let mut parent = empty_node("wrapper");
            parent.children.push(ASTNodeOrToken::Node(deep));
            deep = parent;
        }
        deep.rule_name = "program".into();
        assert!(compile_ast(&deep, "deep")
            .unwrap_err()
            .contains("depth limit"));

        let mut wide = empty_node("program");
        wide.children = (0..MAX_AST_ITEMS)
            .map(|_| ASTNodeOrToken::Node(empty_node("print_statement")))
            .collect();
        assert!(compile_ast(&wide, "wide")
            .unwrap_err()
            .contains("item limit"));
    }
}
