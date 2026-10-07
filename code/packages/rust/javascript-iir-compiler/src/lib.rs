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

/// Parse source and compile the currently supported JavaScript subset.
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, String> {
    let ast = parse_javascript_program(source, EsVersion::Es2020)?;
    compile_ast(&ast, module_name)
}

/// Compile a typed JavaScript AST directly to InterpreterIR.
pub fn compile_ast(ast: &Program, module_name: &str) -> Result<IIRModule, String> {
    let mut compiler = Compiler::default();
    for item in &ast.body {
        match item {
            ProgramItem::Statement(Statement::Tagged(TaggedStatement::ExpressionStatement(
                stmt,
            ))) => compiler.compile_statement(&stmt.expression)?,
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

/// Execute source on this repository's Rust VM and collect console output.
pub fn run_source(source: &str) -> Result<String, String> {
    let mut module = compile_source(source, "javascript-script")?;
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
        if sink.len() >= 1_000_000 {
            return Err(vm_core::errors::VMError::Custom(
                "JavaScript console output limit exceeded".into(),
            ));
        }
        sink.push_str(&format_js_number(*number).map_err(vm_core::errors::VMError::Custom)?);
        sink.push('\n');
        Ok(Value::Null)
    });
    vm.execute(&mut module, "main", &[])
        .map_err(|e| e.to_string())?;
    let result = output
        .lock()
        .map_err(|_| "JavaScript console lock poisoned")?
        .clone();
    Ok(result)
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
    fn unsupported_semantics_are_rejected() {
        assert!(compile_source("console.log('a' + 'b');", "bad").is_err());
        assert!(compile_source("let x = 1;", "bad").is_err());
        assert!(run_source("console.log(1e21);").is_err());
    }
}
