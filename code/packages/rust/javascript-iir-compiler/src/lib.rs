//! Direct JavaScript AST to InterpreterIR lowering for a bounded numeric slice.
//!
//! Semantic IR is not part of the execution path. Unsupported JavaScript
//! syntax is rejected rather than silently assigned another language's rules.

use coding_adventures_javascript_ast::statement::TaggedStatement;
use coding_adventures_javascript_ast::{
    BinaryOperator, Expression, Program, ProgramItem, Statement, UnaryOperator,
};
use coding_adventures_javascript_parser::parse_javascript_program;
use coding_adventures_javascript_tokens::EsVersion;
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use std::sync::{Arc, Mutex};
use vm_core::{value::Value, VMCore};

/// Bound source read and parse work for the first native interpreter pilot.
pub const MAX_SOURCE_BYTES: usize = 64 * 1024;
const MAX_AST_NODES: usize = 16_384;
const MAX_EXPRESSION_DEPTH: usize = 64;

/// Parse source and compile the currently supported JavaScript subset.
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "JavaScript source exceeds the {MAX_SOURCE_BYTES}-byte native pilot limit"
        ));
    }
    let ast = parse_javascript_program(source, EsVersion::Es2020)?;
    compile_ast(&ast, module_name)
}

/// Compile a typed JavaScript AST directly to InterpreterIR.
pub fn compile_ast(ast: &Program, module_name: &str) -> Result<IIRModule, String> {
    let mut compiler = Compiler::default();
    let mut remaining_nodes = MAX_AST_NODES;
    for item in &ast.body {
        if remaining_nodes == 0 {
            return Err("JavaScript AST exceeds the native pilot node limit".into());
        }
        remaining_nodes -= 1;
        match item {
            ProgramItem::Statement(Statement::Tagged(TaggedStatement::ExpressionStatement(
                stmt,
            ))) => {
                check_expression_budget(&stmt.expression, &mut remaining_nodes)?;
                compiler.compile_statement(&stmt.expression)?;
            }
            _ => return Err("unsupported JavaScript statement in native VM pilot".into()),
        }
    }
    compiler.emit("ret_void", None, vec![], "void");
    let mut module = IIRModule::new(module_name, "javascript");
    module.entry_point = Some("main".into());
    module.add_or_replace(IIRFunction::new(
        "main",
        vec![],
        "void",
        compiler.instructions,
    ));
    let errors = module.validate();
    if !errors.is_empty() {
        return Err(format!("invalid JavaScript IIR: {}", errors.join("; ")));
    }
    Ok(module)
}

// Check an AST supplied directly by a caller without recursively walking it.
// Parser depth limits protect source input, but compile_ast also accepts trees
// built by other code. The node count bounds generated IIR and VM work.
fn check_expression_budget(expr: &Expression, remaining: &mut usize) -> Result<(), String> {
    let mut pending = vec![(expr, 1_usize)];
    while let Some((expr, depth)) = pending.pop() {
        if depth > MAX_EXPRESSION_DEPTH {
            return Err("JavaScript AST exceeds the native pilot depth limit".into());
        }
        if *remaining == 0 {
            return Err("JavaScript AST exceeds the native pilot node limit".into());
        }
        *remaining -= 1;
        match expr {
            Expression::UnaryExpression(unary) => pending.push((&unary.argument, depth + 1)),
            Expression::BinaryExpression(binary) => {
                pending.push((&binary.right, depth + 1));
                pending.push((&binary.left, depth + 1));
            }
            Expression::CallExpression(call) => {
                if let Some(argument) = call.arguments.first() {
                    pending.push((argument, depth + 1));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// VM failure with console output completed by earlier JavaScript statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaScriptRunError {
    pub output: String,
    pub message: String,
}

impl std::fmt::Display for JavaScriptRunError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for JavaScriptRunError {}

/// Execute source on this repository's Rust VM and collect console output.
pub fn run_source(source: &str) -> Result<String, JavaScriptRunError> {
    let mut module =
        compile_source(source, "javascript-script").map_err(|message| JavaScriptRunError {
            output: String::new(),
            message,
        })?;
    let output = Arc::new(Mutex::new(String::new()));
    let captured = Arc::clone(&output);
    let mut vm = VMCore::new();
    vm.max_instructions = Some(100_000);
    vm.builtins_mut().register("js_console_log", move |args| {
        let Some(Value::Float(number)) = args.first() else {
            return Err(vm_core::errors::VMError::Custom(
                "js_console_log expects one JavaScript Number".into(),
            ));
        };
        let mut sink = captured.lock().map_err(|_| {
            vm_core::errors::VMError::Custom("JavaScript console lock poisoned".into())
        })?;
        let rendered = format_js_number(*number).map_err(vm_core::errors::VMError::Custom)?;
        if sink
            .len()
            .checked_add(rendered.len() + 1)
            .is_none_or(|length| length > 1_000_000)
        {
            return Err(vm_core::errors::VMError::Custom(
                "JavaScript console output limit exceeded".into(),
            ));
        }
        sink.push_str(&rendered);
        sink.push('\n');
        Ok(Value::Null)
    });
    let execution = vm.execute(&mut module, "main", &[]);
    let result = output
        .lock()
        .map_err(|_| JavaScriptRunError {
            output: String::new(),
            message: "JavaScript console lock poisoned".into(),
        })?
        .clone();
    match execution {
        Ok(_) => Ok(result),
        Err(error) => Err(JavaScriptRunError {
            output: result,
            message: error.to_string(),
        }),
    }
}

fn format_js_number(number: f64) -> Result<String, String> {
    if number.is_nan() {
        Ok("NaN".into())
    } else if number == f64::INFINITY {
        Ok("Infinity".into())
    } else if number == f64::NEG_INFINITY {
        Ok("-Infinity".into())
    } else if number == 0.0 && number.is_sign_negative() {
        Ok("-0".into())
    } else if number != 0.0 && !(1e-6..1e21).contains(&number.abs()) {
        Err("JavaScript number display outside the native pilot's supported range".into())
    } else {
        Ok(number.to_string())
    }
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
        let result = format!("js{}", self.next_register);
        self.next_register += 1;
        result
    }

    fn compile_statement(&mut self, expr: &Expression) -> Result<(), String> {
        if let Expression::CallExpression(call) = expr {
            if is_console_log(&call.callee) {
                if call.arguments.len() != 1 {
                    return Err("native console.log pilot requires exactly one argument".into());
                }
                let value = self.compile_number(&call.arguments[0])?;
                self.emit(
                    "call_builtin",
                    None,
                    vec![Operand::Var("js_console_log".into()), value],
                    "void",
                );
                return Ok(());
            }
        }
        self.compile_number(expr)?;
        Ok(())
    }

    fn compile_number(&mut self, expr: &Expression) -> Result<Operand, String> {
        match expr {
            Expression::NumericLiteral(number) => {
                let dest = self.register();
                self.emit(
                    "const",
                    Some(dest.clone()),
                    vec![Operand::Float(number.value)],
                    "f64",
                );
                Ok(Operand::Var(dest))
            }
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Negate => {
                let source = self.compile_number(&unary.argument)?;
                let dest = self.register();
                self.emit("neg", Some(dest.clone()), vec![source], "f64");
                Ok(Operand::Var(dest))
            }
            Expression::BinaryExpression(binary) => {
                let op = match binary.operator {
                    BinaryOperator::Add => "add",
                    BinaryOperator::Sub => "sub",
                    BinaryOperator::Mul => "mul",
                    BinaryOperator::Div => "div",
                    _ => {
                        return Err(
                            "unsupported JavaScript numeric operator in native VM pilot".into()
                        )
                    }
                };
                let left = self.compile_number(&binary.left)?;
                let right = self.compile_number(&binary.right)?;
                let dest = self.register();
                self.emit(op, Some(dest.clone()), vec![left, right], "f64");
                Ok(Operand::Var(dest))
            }
            _ => Err("unsupported JavaScript expression in native VM pilot".into()),
        }
    }
}

fn is_console_log(callee: &Expression) -> bool {
    let Expression::MemberExpression(member) = callee else {
        return false;
    };
    !member.computed
        && matches!(member.object.as_ref(), Expression::Identifier(id) if id.name == "console")
        && matches!(member.property.as_ref(), Expression::Identifier(id) if id.name == "log")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executes_javascript_numbers_on_our_vm() {
        assert_eq!(
            run_source("console.log(1 + 2); console.log(1 / 2);").unwrap(),
            "3\n0.5\n"
        );
        assert_eq!(run_source("console.log(-(1 + 2) * 3);").unwrap(), "-9\n");
    }

    #[test]
    fn retains_completed_console_output_before_vm_error() {
        let error = run_source("console.log(7); console.log(1e21);").unwrap_err();
        assert_eq!(error.output, "7\n");
        assert!(error.message.contains("number display outside"));

        let error = run_source("let x = 1;").unwrap_err();
        assert!(error.output.is_empty());
        assert!(error.message.contains("unsupported JavaScript statement"));
    }

    #[test]
    fn unsupported_semantics_are_rejected() {
        assert!(compile_source("console.log('a' + 'b');", "bad").is_err());
        assert!(compile_source("let x = 1;", "bad").is_err());
        assert!(run_source("console.log(1e21);").is_err());
        assert!(compile_source(&" ".repeat(MAX_SOURCE_BYTES + 1), "too-large").is_err());
        let ast =
            parse_javascript_program(&"1;".repeat(MAX_AST_NODES + 1), EsVersion::Es2020).unwrap();
        assert!(compile_ast(&ast, "too-many-nodes").is_err());
    }
}
