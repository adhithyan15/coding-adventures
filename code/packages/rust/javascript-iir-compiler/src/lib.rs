//! Direct JavaScript AST to InterpreterIR lowering for a bounded numeric slice.
//!
//! Semantic IR is not part of the execution path. Unsupported JavaScript
//! syntax is rejected rather than silently assigned another language's rules.
//!
//! # The accepted program shape
//!
//! A program is a flat list of top-level items, each one of:
//!
//! ```text
//! expression statement   1 + 2;   console.log(x, x * 4);
//! lexical declaration    let x = 1 / 2;   const y = -0;
//! ```
//!
//! There is no control flow and no assignment yet. That makes every binding's
//! state *statically* known at every program point, which is what lets the
//! compiler decide ReferenceErrors ahead of time:
//!
//! ```text
//! console.log(a);   // a is declared below → TDZ read  → ReferenceError
//! let a = 2;        // a becomes initialized here
//! console.log(a);   // a is initialized   → plain register read
//! console.log(z);   // z is declared nowhere → ReferenceError
//! ```
//!
//! Each binding lives in an IIR register named `js.<name>`. A `.` can never
//! appear in a JavaScript IdentifierName, so that namespace cannot meet the
//! compiler's temporaries (`js0`, `js1`, ...) or the runner's builtins
//! (`js_console_log`, ...). The table below is the whole register namespace:
//!
//! | Spelling        | Owner                 | Third character |
//! | --------------- | --------------------- | --------------- |
//! | `js<digits>`    | compiler temporaries  | a digit         |
//! | `js_<name>`     | runner builtins       | `_`             |
//! | `js.<name>`     | user `let` / `const`  | `.`             |

use coding_adventures_javascript_ast::statement::TaggedStatement;
use coding_adventures_javascript_ast::{
    BinaryOperator, BindingTarget, Declaration, Expression, Program, ProgramItem, Statement,
    UnaryOperator, VarKind, VariableDeclaration,
};
#[cfg(test)]
use coding_adventures_javascript_parser::parse_javascript_program;
use coding_adventures_javascript_parser::parse_javascript_program_division_only;
use coding_adventures_javascript_tokens::EsVersion;
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use vm_core::{value::Value, VMCore};

/// Bound source read and parse work for the first native interpreter pilot.
pub const MAX_SOURCE_BYTES: usize = 64 * 1024;
const MAX_AST_NODES: usize = 16_384;
const MAX_EXPRESSION_DEPTH: usize = 64;

/// Runner builtin raised by reading a name that no declaration binds.
const NOT_DEFINED_BUILTIN: &str = "js_reference_error_not_defined";
/// Runner builtin raised by reading a lexical binding inside its temporal dead
/// zone: after the script starts but before its declaration has executed.
const BEFORE_INITIALIZATION_BUILTIN: &str = "js_reference_error_before_initialization";

/// Names the native runner's global object is defined to hold.
///
/// ECMAScript resolves an identifier that no `let`/`const` binds against the
/// global object. Every name here *is* a global in a real engine — `NaN`
/// reads as a Number, `Math` as an object — but the pilot implements none of
/// them as values yet. If the compiler treated them as undeclared it would
/// report `ReferenceError: NaN is not defined`, a lie. So reading one, or
/// declaring one with `let`/`const`, is a compile-time "unsupported" error.
///
/// The list is the ECMA-262 global object (value, function, constructor and
/// namespace properties, including recent additions), Annex B's `escape` and
/// `unescape`, ECMA-402's `Intl`, and the runner's own host object `console`.
/// Listing a name that some engine lacks only rejects more programs, so the
/// list errs toward inclusion. Node-only host globals (`process`, `require`,
/// `Buffer`, ...) are deliberately absent: this runner is a different host
/// and does not define them, exactly as a bare engine shell does not.
const RUNNER_GLOBAL_NAMES: &[&str] = &[
    // Host object provided by this runner.
    "console",
    // Value properties of the global object.
    "globalThis",
    "Infinity",
    "NaN",
    "undefined",
    // Function properties.
    "eval",
    "isFinite",
    "isNaN",
    "parseFloat",
    "parseInt",
    "decodeURI",
    "decodeURIComponent",
    "encodeURI",
    "encodeURIComponent",
    "escape",
    "unescape",
    // Constructor properties.
    "AggregateError",
    "Array",
    "ArrayBuffer",
    "AsyncDisposableStack",
    "BigInt",
    "BigInt64Array",
    "BigUint64Array",
    "Boolean",
    "DataView",
    "Date",
    "DisposableStack",
    "Error",
    "EvalError",
    "FinalizationRegistry",
    "Float16Array",
    "Float32Array",
    "Float64Array",
    "Function",
    "Int8Array",
    "Int16Array",
    "Int32Array",
    "Iterator",
    "Map",
    "Number",
    "Object",
    "Promise",
    "Proxy",
    "RangeError",
    "ReferenceError",
    "RegExp",
    "Set",
    "SharedArrayBuffer",
    "String",
    "SuppressedError",
    "Symbol",
    "SyntaxError",
    "TypeError",
    "Uint8Array",
    "Uint8ClampedArray",
    "Uint16Array",
    "Uint32Array",
    "URIError",
    "WeakMap",
    "WeakRef",
    "WeakSet",
    // Namespace objects.
    "Atomics",
    "Intl",
    "JSON",
    "Math",
    "Reflect",
];

/// Parse source and compile the currently supported JavaScript subset.
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "JavaScript source exceeds the {MAX_SOURCE_BYTES}-byte native pilot limit"
        ));
    }
    // Regex literals are outside the native subset. Select the division goal
    // before tokenization so two arithmetic slashes cannot become one regex.
    let ast = parse_javascript_program_division_only(source, EsVersion::Es2020)?;
    compile_ast(&ast, module_name)
}

/// Compile a typed JavaScript AST directly to InterpreterIR.
///
/// Compilation runs in two passes, mirroring how an engine prepares a script:
///
/// 1. **Declaration instantiation.** Collect every top-level lexical name.
///    A duplicate is an early `SyntaxError`, so the program never starts and
///    produces no output. Knowing all names up front is also what separates a
///    temporal-dead-zone read (declared later) from an undeclared one.
/// 2. **Lowering.** Walk the items in source order, charging the node budget
///    and emitting IIR. A declaration marks its name initialized only after
///    its initializer is lowered, so `let a = a;` reads `a` in its dead zone.
pub fn compile_ast(ast: &Program, module_name: &str) -> Result<IIRModule, String> {
    // Every item costs at least one node, so a longer body can never fit the
    // budget. Checking first also bounds the declaration pre-pass below.
    if ast.body.len() > MAX_AST_NODES {
        return Err("JavaScript AST exceeds the native pilot node limit".into());
    }
    let mut compiler = Compiler {
        lexical_names: collect_lexical_names(ast)?,
        ..Compiler::default()
    };
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
            ProgramItem::Statement(Statement::Declaration(Declaration::VariableDeclaration(
                declaration,
            )))
            | ProgramItem::Declaration(Declaration::VariableDeclaration(declaration)) => {
                let (name, init) = lexical_binding(declaration)?;
                // One node for the declarator and one for its binding name,
                // then the initializer pays like any other expression.
                if remaining_nodes < 2 {
                    return Err("JavaScript AST exceeds the native pilot node limit".into());
                }
                remaining_nodes -= 2;
                check_expression_budget(init, &mut remaining_nodes)?;
                compiler.compile_lexical_binding(name, init)?;
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

// Pass 1: the script's LexicallyDeclaredNames. ECMAScript makes a repeated
// name an early error (Script static semantics), and V8 words it exactly as
// below. Each declaration's shape is validated here too, so pass 2 can rely
// on `lexical_binding` succeeding for every name collected.
fn collect_lexical_names(ast: &Program) -> Result<HashSet<String>, String> {
    let mut names = HashSet::new();
    for item in &ast.body {
        let declaration = match item {
            ProgramItem::Statement(Statement::Declaration(Declaration::VariableDeclaration(
                declaration,
            )))
            | ProgramItem::Declaration(Declaration::VariableDeclaration(declaration)) => {
                declaration
            }
            _ => continue,
        };
        let (name, _) = lexical_binding(declaration)?;
        if !names.insert(name.to_string()) {
            return Err(format!(
                "SyntaxError: Identifier '{name}' has already been declared"
            ));
        }
    }
    Ok(names)
}

// Accept exactly `let NAME = INIT;` or `const NAME = INIT;` and return its
// parts. Every other declaration shape is outside this slice:
//
//   var x = 1;        function-scoped hoisting with `undefined` — not modelled
//   let x;            would need the `undefined` value
//   let a = 1, b = 2; kept to one declarator to keep the slice minimal
//   let [a] = xs;     destructuring needs arrays/objects
fn lexical_binding(declaration: &VariableDeclaration) -> Result<(&str, &Expression), String> {
    if declaration.kind == VarKind::Var {
        return Err(
            "unsupported JavaScript `var` declaration in native VM pilot; use `let` or `const`"
                .into(),
        );
    }
    let [declarator] = declaration.declarations.as_slice() else {
        return Err(
            "native let/const pilot requires exactly one declarator per declaration".into(),
        );
    };
    // `BindingTarget` has only the identifier form today. When destructuring
    // patterns join it, this irrefutable `let` stops compiling, forcing an
    // explicit decision here instead of silently accepting a new shape.
    let BindingTarget::Identifier(id) = &declarator.id;
    check_binding_name(&id.name)?;
    // `const k;` is a real JavaScript early error, worded here as V8 words
    // it; `let x;` is valid JavaScript that this slice merely cannot run yet.
    let init = declarator.init.as_ref().ok_or(match declaration.kind {
        VarKind::Const => "SyntaxError: Missing initializer in const declaration",
        _ => "native let/const pilot requires an initializer",
    })?;
    Ok((&id.name, init))
}

fn check_binding_name(name: &str) -> Result<(), String> {
    check_identifier_spelling(name)?;
    if RUNNER_GLOBAL_NAMES.contains(&name) {
        return Err(format!(
            "native let/const pilot cannot bind the global name `{name}`"
        ));
    }
    Ok(())
}

// Source parsing only produces identifier tokens, but `compile_ast` also takes
// trees built by other code. Accept only a plain IdentifierName spelling, so
// a forged name cannot smuggle `.`, spaces or control characters into register
// names and diagnostics. Unicode escapes (`\u0061`) are rejected too: the
// token layer keeps their raw spelling, so `\u0061` and `a` would otherwise be
// two different names although JavaScript treats them as one.
fn check_identifier_spelling(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let starts = chars
        .next()
        .is_some_and(|c| c == '$' || c == '_' || c.is_alphabetic());
    let continues = chars
        .all(|c| c == '$' || c == '_' || c == '\u{200C}' || c == '\u{200D}' || c.is_alphanumeric());
    if starts && continues {
        Ok(())
    } else {
        Err("unsupported JavaScript identifier spelling in native VM pilot".into())
    }
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
                for argument in call.arguments.iter().rev() {
                    pending.push((argument, depth + 1));
                }
                pending.push((&call.callee, depth + 1));
            }
            Expression::MemberExpression(member) => {
                pending.push((&member.property, depth + 1));
                pending.push((&member.object, depth + 1));
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
        let rendered = match args {
            [] => String::new(),
            [Value::Float(number)] => {
                format_js_number(*number).map_err(vm_core::errors::VMError::Custom)?
            }
            [Value::Float(first), Value::Float(second)] => {
                let first = format_js_number(*first).map_err(vm_core::errors::VMError::Custom)?;
                let second = format_js_number(*second).map_err(vm_core::errors::VMError::Custom)?;
                format!("{first} {second}")
            }
            [Value::Float(first), Value::Float(second), Value::Float(third)] => {
                // Build a complete line before locking or changing the sink.
                // A display failure in any argument leaves earlier calls intact.
                let first = format_js_number(*first).map_err(vm_core::errors::VMError::Custom)?;
                let second = format_js_number(*second).map_err(vm_core::errors::VMError::Custom)?;
                let third = format_js_number(*third).map_err(vm_core::errors::VMError::Custom)?;
                format!("{first} {second} {third}")
            }
            _ => {
                return Err(vm_core::errors::VMError::Custom(
                    "js_console_log expects zero, one, two, or three JavaScript Numbers".into(),
                ))
            }
        };
        let mut sink = captured.lock().map_err(|_| {
            vm_core::errors::VMError::Custom("JavaScript console lock poisoned".into())
        })?;
        let appended_bytes = rendered.len() + 1;
        if sink
            .len()
            .checked_add(appended_bytes)
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
    // ReferenceErrors are decided by the compiler (see `compile_reference`);
    // these builtins only raise them, carrying the JavaScript name as a string
    // operand. Messages follow V8's wording. Output from earlier completed
    // `console.log` calls stays in the sink, as for every other VM error.
    vm.builtins_mut().register(NOT_DEFINED_BUILTIN, |args| {
        raise_reference_error(args, |name| {
            format!("ReferenceError: {name} is not defined")
        })
    });
    vm.builtins_mut()
        .register(BEFORE_INITIALIZATION_BUILTIN, |args| {
            raise_reference_error(args, |name| {
                format!("ReferenceError: Cannot access '{name}' before initialization")
            })
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

// Shared body of the two ReferenceError builtins: always an error, never a
// value. A malformed call (not exactly one string name) is still an error,
// just a less informative one.
fn raise_reference_error(
    args: &[Value],
    render: impl Fn(&str) -> String,
) -> Result<Value, vm_core::errors::VMError> {
    match args {
        [Value::Str(name)] => Err(vm_core::errors::VMError::Custom(render(name))),
        _ => Err(vm_core::errors::VMError::Custom(
            "JavaScript ReferenceError builtin expects one binding name".into(),
        )),
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
    /// Every top-level `let`/`const` name in the script (pass 1).
    lexical_names: HashSet<String>,
    /// The subset whose declaration has already been lowered. A name in
    /// `lexical_names` but not here is in its temporal dead zone.
    initialized: HashSet<String>,
}

/// IIR register holding the JavaScript binding `name`; see the module docs
/// for why the `js.` prefix cannot collide with anything else.
fn binding_register(name: &str) -> String {
    format!("js.{name}")
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
                if call.arguments.len() > 3 {
                    return Err("native console.log pilot requires at most three arguments".into());
                }
                let mut operands = vec![Operand::Var("js_console_log".into())];
                for argument in &call.arguments {
                    operands.push(self.compile_number(argument)?);
                }
                self.emit("call_builtin", None, operands, "void");
                return Ok(());
            }
        }
        self.compile_number(expr)?;
        Ok(())
    }

    // `let NAME = INIT;` evaluates INIT, then copies the result into the
    // binding's own register. Only after that is NAME readable.
    fn compile_lexical_binding(&mut self, name: &str, init: &Expression) -> Result<(), String> {
        let value = self.compile_number(init)?;
        self.emit("mov", Some(binding_register(name)), vec![value], "f64");
        self.initialized.insert(name.to_string());
        Ok(())
    }

    // Resolve an identifier read. With no control flow, the binding's state
    // here is the same on every execution, so the decision is static:
    //
    //   state at this point            lowering
    //   ----------------------------   -------------------------------------
    //   declared and initialized       read register `js.NAME`
    //   a runner global (NaN, Math…)   compile error: not implemented yet
    //   declared later in the script   call js_reference_error_before_initialization
    //   declared nowhere               call js_reference_error_not_defined
    //
    // The error builtins never return. Their call still names a destination
    // register so the enclosing expression lowers normally; the VM stops
    // before anything could read it.
    fn compile_reference(&mut self, name: &str) -> Result<Operand, String> {
        check_identifier_spelling(name)?;
        if self.initialized.contains(name) {
            return Ok(Operand::Var(binding_register(name)));
        }
        if RUNNER_GLOBAL_NAMES.contains(&name) {
            return Err(format!(
                "unsupported JavaScript global `{name}` in native VM pilot"
            ));
        }
        let builtin = if self.lexical_names.contains(name) {
            BEFORE_INITIALIZATION_BUILTIN
        } else {
            NOT_DEFINED_BUILTIN
        };
        let dest = self.register();
        self.emit(
            "call_builtin",
            Some(dest.clone()),
            vec![Operand::Var(builtin.into()), Operand::Str(name.into())],
            "f64",
        );
        Ok(Operand::Var(dest))
    }

    fn compile_number(&mut self, expr: &Expression) -> Result<Operand, String> {
        match expr {
            Expression::Identifier(id) => self.compile_reference(&id.name),
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
    fn native_source_repeated_division_reaches_the_vm() {
        for (source, expected) in [
            ("console.log(-0, 1 / 0, 0 / 0)", "-0 Infinity NaN\n"),
            ("console.log(8/2/2, 9/3, 12/4);", "2 3 3\n"),
            ("console.log(8 /* / ignored / */ / 2 / 2);", "2\n"),
            ("console.log(8 /\n2 / 2)\nconsole.log(6 / 3 / 2)", "2\n1\n"),
        ] {
            assert_eq!(run_source(source).unwrap(), expected, "{source}");
        }
        let module = compile_source("console.log(8 / 2 / 2, 9 / 3, 12 / 4)", "div-order").unwrap();
        assert_eq!(
            module.functions[0]
                .instructions
                .iter()
                .filter(|i| i.op == "div")
                .count(),
            4
        );
    }

    #[test]
    fn native_division_goal_rejects_regex_and_malformed_operands() {
        for source in [
            "console.log(/1/);",
            "console.log(/1/ / 2);",
            "console.log(1, /2/g, 3);",
            "/1/;",
            "console.log(8 /= 2);",
            "console.log(8 /);",
            "console.log(8 / / 2);",
            "console.log(1); console.log(8 / 2, /3/, 4);",
        ] {
            let error = run_source(source).unwrap_err();
            assert!(error.output.is_empty(), "{source}");
        }
        for source in [
            "console.log(7); console.log(1e21 / 1, 8 / 2, 9 / 3);",
            "console.log(7); console.log(8 / 2, 1e21 / 1, 9 / 3);",
            "console.log(7); console.log(8 / 2, 9 / 3, 1e21 / 1);",
        ] {
            let error = run_source(source).unwrap_err();
            assert_eq!(error.output, "7\n", "{source}");
            assert!(error.message.contains("number display outside"));
        }
    }

    #[test]
    fn native_division_spine_guard_preserves_semantic_depth_boundary() {
        let chain = |length| vec!["1"; length].join("/");
        assert_eq!(
            run_source(&format!("console.log({});", chain(63))).unwrap(),
            "1\n"
        );
        let error = run_source(&format!("console.log({});", chain(64))).unwrap_err();
        assert!(error.output.is_empty());
        assert!(error.message.contains("depth limit"));
        assert_eq!(run_source(&format!("{};", chain(64))).unwrap(), "");
        for length in [5_000, 20_000] {
            let error = run_source(&format!("console.log({});", chain(length))).unwrap_err();
            assert!(error.output.is_empty());
            assert!(error.message.contains("spine limit"));
        }
        for length in [200, 5_000] {
            let source = format!("console.log({});", vec!["1"; length].join("**"));
            let error = run_source(&source).unwrap_err();
            assert!(error.output.is_empty());
            assert!(error.message.contains("exponentiation is unsupported"));
        }
    }

    #[test]
    fn executes_javascript_numbers_on_our_vm() {
        assert_eq!(
            run_source("console.log(1 + 2); console.log(1 / 2);").unwrap(),
            "3\n0.5\n"
        );
        assert_eq!(run_source("console.log(-(1 + 2) * 3);").unwrap(), "-9\n");
    }

    #[test]
    fn empty_console_log_lowers_directly_and_preserves_effect_order() {
        let ast = parse_javascript_program("console.log();", EsVersion::Es2020).unwrap();
        let module = compile_ast(&ast, "empty-log").unwrap();
        let calls: Vec<_> = module.functions[0]
            .instructions
            .iter()
            .filter(|instruction| instruction.op == "call_builtin")
            .collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].srcs.len(),
            1,
            "only the builtin name is an operand"
        );
        assert_eq!(
            run_source("console.log(); console.log(7);").unwrap(),
            "\n7\n"
        );
    }

    #[test]
    fn two_numeric_arguments_lower_in_order_and_print_with_one_space() {
        let ast =
            parse_javascript_program("console.log(1 + 2, 3 / 2);", EsVersion::Es2020).unwrap();
        let module = compile_ast(&ast, "two-log").unwrap();
        let call = module.functions[0]
            .instructions
            .iter()
            .find(|instruction| instruction.op == "call_builtin")
            .unwrap();
        assert_eq!(call.srcs.len(), 3);
        assert_eq!(
            run_source("console.log(); console.log(1 + 2, 3 / 2); console.log(4);").unwrap(),
            "\n3 1.5\n4\n"
        );
    }

    #[test]
    fn three_numeric_arguments_lower_in_source_order_and_execute_on_our_vm() {
        let ast = parse_javascript_program("console.log(11, 22, 33);", EsVersion::Es2020).unwrap();
        let module = compile_ast(&ast, "three-log").unwrap();
        let instructions = &module.functions[0].instructions;
        let call = instructions
            .iter()
            .find(|instruction| instruction.op == "call_builtin")
            .unwrap();
        assert_eq!(call.srcs.len(), 4);
        assert_eq!(call.srcs[0], Operand::Var("js_console_log".into()));
        for (index, number) in [11.0, 22.0, 33.0].into_iter().enumerate() {
            // Each source argument's constant must precede the call and feed
            // the corresponding operand, not merely produce the right arity.
            assert_eq!(instructions[index].op, "const");
            assert_eq!(instructions[index].srcs, vec![Operand::Float(number)]);
            assert_eq!(
                call.srcs[index + 1],
                Operand::Var(instructions[index].dest.clone().unwrap())
            );
        }
        assert_eq!(
            run_source("console.log(); console.log(7); console.log(1, 2); console.log(1 + 2, 3 / 2, -(4 * 2));").unwrap(),
            "\n7\n1 2\n3 1.5 -8\n"
        );
        assert_eq!(
            run_source("console.log(-0, 1 / 0, 0);").unwrap(),
            "-0 Infinity 0\n"
        );
        assert_eq!(
            run_source("console.log(0 / 0, -0, 0);").unwrap(),
            "NaN -0 0\n"
        );
        assert_eq!(
            run_source("console.log(0.000001, 100000000000000000000, -1 / 0);").unwrap(),
            "0.000001 100000000000000000000 -Infinity\n"
        );
    }

    #[test]
    fn three_argument_display_failure_appends_none_of_the_failed_call() {
        for source in [
            "console.log(); console.log(7); console.log(1e21, 2, 3);",
            "console.log(); console.log(7); console.log(1, 1e21, 3);",
            "console.log(); console.log(7); console.log(1, 2, 1e21);",
            "console.log(); console.log(7); console.log(1, 2, 0.0000001);",
        ] {
            let error = run_source(source).unwrap_err();
            assert_eq!(error.output, "\n7\n", "{source}");
            assert!(error.message.contains("number display outside"), "{error}");
        }
    }

    #[test]
    fn unsupported_third_arguments_and_four_arguments_emit_no_output() {
        for source in [
            "console.log(7); console.log(1, 2, 'three');",
            "console.log(7); console.log(1, 2, console.log(3));",
            "console.log(7); console.log(1, 2, 3 % 2);",
            "console.log(7); console.log(1, 2, 3, 4);",
        ] {
            let error = run_source(source).unwrap_err();
            assert!(error.output.is_empty(), "{source}");
            assert!(
                error.message.contains("unsupported") || error.message.contains("at most three"),
                "{error}"
            );
        }
    }

    #[test]
    fn direct_ast_node_budget_checks_the_third_console_argument() {
        use coding_adventures_javascript_ast::expression::BinaryExpression;
        let mut ast = parse_javascript_program("console.log(1, 2, 3);", EsVersion::Es2020).unwrap();
        let ProgramItem::Statement(Statement::Tagged(TaggedStatement::ExpressionStatement(stmt))) =
            &mut ast.body[0]
        else {
            panic!("expected expression statement");
        };
        let Expression::CallExpression(call) = &mut stmt.expression else {
            panic!("expected call expression");
        };
        let mut wide = call.arguments[2].clone();
        // A shallow balanced tree reaches 16,383 nodes. Adding the other
        // arguments, callee, and statement exceeds the whole-AST budget.
        for _ in 0..13 {
            wide = Expression::BinaryExpression(BinaryExpression {
                cv: None,
                operator: BinaryOperator::Add,
                left: Box::new(wide.clone()),
                right: Box::new(wide),
            });
        }
        call.arguments[2] = wide;
        let error = compile_ast(&ast, "forged-wide-third-argument").unwrap_err();
        assert!(error.contains("node limit"), "{error}");
    }

    #[test]
    fn two_argument_display_failure_keeps_only_prior_completed_output() {
        let error = run_source("console.log(7); console.log(1e21, 2);").unwrap_err();
        assert_eq!(error.output, "7\n");
        assert!(error.message.contains("number display outside"));
        let error = run_source("console.log(7); console.log(1, 1e21);").unwrap_err();
        assert_eq!(error.output, "7\n");
        assert!(error.message.contains("number display outside"));
        let error = run_source("console.log(7); console.log(1, 2, 3, 4);").unwrap_err();
        assert!(error.output.is_empty());
        assert!(error.message.contains("requires"));
    }

    #[test]
    fn direct_ast_budget_checks_every_console_argument() {
        use coding_adventures_javascript_ast::expression::UnaryExpression;

        for argument_index in 0..3 {
            let mut ast =
                parse_javascript_program("console.log(1, 2, 3);", EsVersion::Es2020).unwrap();
            let ProgramItem::Statement(Statement::Tagged(TaggedStatement::ExpressionStatement(
                stmt,
            ))) = &mut ast.body[0]
            else {
                panic!("expected expression statement");
            };
            let Expression::CallExpression(call) = &mut stmt.expression else {
                panic!("expected call expression");
            };
            let mut deep = call.arguments[argument_index].clone();
            for _ in 0..MAX_EXPRESSION_DEPTH {
                deep = Expression::UnaryExpression(UnaryExpression {
                    cv: None,
                    operator: UnaryOperator::Negate,
                    prefix: true,
                    argument: Box::new(deep),
                });
            }
            call.arguments[argument_index] = deep;
            let error = compile_ast(&ast, "forged-deep-argument").unwrap_err();
            assert!(error.contains("depth limit"), "{error}");
        }
    }

    #[test]
    fn empty_console_output_survives_later_vm_error_but_compile_errors_emit_nothing() {
        let error = run_source("console.log(); console.log(1e21);").unwrap_err();
        assert_eq!(error.output, "\n");
        assert!(error.message.contains("number display outside"));
        let error = run_source("console.log(); console.log(1, 2, 3, 4);").unwrap_err();
        assert!(error.output.is_empty());
        assert!(error.message.contains("requires"));
    }

    #[test]
    fn retains_completed_console_output_before_vm_error() {
        let error = run_source("console.log(7); console.log(1e21);").unwrap_err();
        assert_eq!(error.output, "7\n");
        assert!(error.message.contains("number display outside"));

        // `let x = 1;` joined the subset in 0.2.0; a statement form that is
        // still outside it keeps this compile-error probe meaningful.
        let error = run_source("if (1) 1;").unwrap_err();
        assert!(error.output.is_empty());
        assert!(error.message.contains("unsupported JavaScript statement"));
    }

    #[test]
    fn unsupported_semantics_are_rejected() {
        assert!(compile_source("console.log('a' + 'b');", "bad").is_err());
        assert!(compile_source("var x = 1;", "bad").is_err());
        assert!(run_source("console.log(1e21);").is_err());
        assert!(compile_source(&" ".repeat(MAX_SOURCE_BYTES + 1), "too-large").is_err());
        let ast =
            parse_javascript_program(&"1;".repeat(MAX_AST_NODES + 1), EsVersion::Es2020).unwrap();
        assert!(compile_ast(&ast, "too-many-nodes").is_err());
    }

    // ---- top-level numeric let/const (0.2.0) -------------------------------

    fn assert_runs(source: &str, expected: &str) {
        assert_eq!(run_source(source).unwrap(), expected, "{source}");
    }

    fn assert_compile_error(source: &str, needle: &str) {
        // Prefixing completed output proves a compile error runs nothing.
        let source = format!("console.log(7); {source}");
        let error = run_source(&source).unwrap_err();
        assert!(error.output.is_empty(), "{source}");
        assert!(error.message.contains(needle), "{source}: {error}");
    }

    #[test]
    fn let_and_const_bindings_execute_on_our_vm() {
        assert_runs("let x = 1 / 2; console.log(x, x * 4)", "0.5 2\n");
        assert_runs("const y = -0; console.log(y)", "-0\n");
        assert_runs(
            "let a = 2; const b = a * 3; console.log(a, b, -b / a); console.log(b - a);",
            "2 6 -3\n4\n",
        );
        assert_runs(
            "let n = 0 / 0; const i = 1 / n; console.log(n, i);",
            "NaN NaN\n",
        );
        assert_runs(
            "const z = 1 / -0; let w = -z; console.log(z, w);",
            "-Infinity Infinity\n",
        );
        // A bare reference is an expression statement with no output.
        assert_runs("let x = 1; x; -x + x;", "");
        // Declarations need no trailing semicolon thanks to ASI.
        assert_runs("let x = 3\nconsole.log(x)", "3\n");
    }

    #[test]
    fn user_names_cannot_meet_temporaries_or_builtins() {
        // `js0` is also the compiler's first temporary and `js_console_log`
        // the console builtin; user bindings live under `js.` instead.
        assert_runs(
            "let js0 = 5; const js_console_log = 6; let $ = 1; let _ = 2; \
             console.log(js0, js_console_log, js0 + $ + _);",
            "5 6 8\n",
        );
        let module = compile_source("let js0 = 5; console.log(js0);", "namespace").unwrap();
        let instructions = &module.functions[0].instructions;
        let mov = instructions.iter().find(|i| i.op == "mov").unwrap();
        assert_eq!(mov.dest.as_deref(), Some("js.js0"));
        assert_eq!(mov.srcs, vec![Operand::Var("js0".into())]);
        let call = instructions
            .iter()
            .find(|i| i.op == "call_builtin")
            .unwrap();
        assert_eq!(
            call.srcs,
            vec![
                Operand::Var("js_console_log".into()),
                Operand::Var("js.js0".into())
            ]
        );
    }

    #[test]
    fn undeclared_reads_raise_reference_error_after_prior_output() {
        let error = run_source("console.log(1); console.log(z)").unwrap_err();
        assert_eq!(error.output, "1\n");
        assert_eq!(error.message, "ReferenceError: z is not defined");
        // Arguments evaluate before the call, so the failing call adds nothing.
        let error = run_source("console.log(1); let a = 2; console.log(a, a + q, 3);").unwrap_err();
        assert_eq!(error.output, "1\n");
        assert_eq!(error.message, "ReferenceError: q is not defined");
        // A bare read throws too, and so does one inside an initializer.
        let error = run_source("console.log(); z; console.log(2);").unwrap_err();
        assert_eq!(error.output, "\n");
        assert_eq!(error.message, "ReferenceError: z is not defined");
        let error = run_source("console.log(4); let b = 1 + y;").unwrap_err();
        assert_eq!(error.output, "4\n");
        assert_eq!(error.message, "ReferenceError: y is not defined");
        // Node-only host globals are not part of this runner's host.
        let error = run_source("console.log(process);").unwrap_err();
        assert_eq!(error.message, "ReferenceError: process is not defined");
    }

    #[test]
    fn temporal_dead_zone_reads_raise_reference_error_after_prior_output() {
        let error = run_source("console.log(1); console.log(a); let a = 2;").unwrap_err();
        assert_eq!(error.output, "1\n");
        assert_eq!(
            error.message,
            "ReferenceError: Cannot access 'a' before initialization"
        );
        // A binding is not readable inside its own initializer.
        let error = run_source("console.log(1); const a = a + 1;").unwrap_err();
        assert_eq!(error.output, "1\n");
        assert_eq!(
            error.message,
            "ReferenceError: Cannot access 'a' before initialization"
        );
        // TDZ is per binding: an initialized neighbour still reads fine.
        let error =
            run_source("let a = 1; console.log(a); console.log(a, b); let b = 2;").unwrap_err();
        assert_eq!(error.output, "1\n");
        assert!(error.message.contains("Cannot access 'b'"), "{error}");
        // The IIR raises at the read site and never reaches the late `mov`.
        let module = compile_source("console.log(a); let a = 2;", "tdz").unwrap();
        let instructions = &module.functions[0].instructions;
        assert_eq!(instructions[0].op, "call_builtin");
        assert_eq!(
            instructions[0].srcs,
            vec![
                Operand::Var(BEFORE_INITIALIZATION_BUILTIN.into()),
                Operand::Str("a".into())
            ]
        );
    }

    #[test]
    fn duplicate_lexical_declarations_are_early_errors_with_no_output() {
        for source in [
            "let a = 1; let a = 2;",
            "console.log(1); let a = 1; let a = 2;",
            "const a = 1; let a = 2;",
            "let a = 1; console.log(a); const a = 2;",
            // Early errors win over runtime errors that would come first.
            "console.log(1); console.log(z); let a = 1; let a = 2;",
        ] {
            let error = run_source(source).unwrap_err();
            assert!(error.output.is_empty(), "{source}");
            assert_eq!(
                error.message, "SyntaxError: Identifier 'a' has already been declared",
                "{source}"
            );
            assert!(compile_source(source, "duplicate").is_err());
        }
    }

    #[test]
    fn unsupported_declaration_forms_are_rejected_at_compile_time() {
        assert_compile_error("var x = 1;", "`var` declaration");
        assert_compile_error("let x;", "requires an initializer");
        assert_compile_error(
            "const k;",
            "SyntaxError: Missing initializer in const declaration",
        );
        assert_compile_error("let a = 1, b = 2;", "exactly one declarator");
        assert_compile_error("let console = 1;", "global name `console`");
        assert_compile_error("const NaN = 1;", "global name `NaN`");
        assert_compile_error("let Math = 1;", "global name `Math`");
        assert_compile_error("let s = 'a';", "unsupported JavaScript expression");
        assert_compile_error("const t = true;", "unsupported JavaScript expression");
        assert_compile_error("let f = console.log;", "unsupported JavaScript expression");
        assert_compile_error("let m = 5 % 2;", "unsupported JavaScript numeric operator");
        // Assignment is not in the subset, so `const` reassignment cannot arise.
        assert_compile_error("x = 2;", "unsupported JavaScript expression");
        assert_compile_error("let x = 1; x = 2;", "unsupported JavaScript expression");
        assert_compile_error("const c = 1; c = 2;", "unsupported JavaScript expression");
        assert_compile_error(
            "let x = 1; console.log(x = 2);",
            "unsupported JavaScript expression",
        );
        // Known globals are not implemented as values yet; they must not read
        // as undeclared names.
        assert_compile_error("console.log(NaN);", "unsupported JavaScript global `NaN`");
        assert_compile_error("console.log(Infinity);", "global `Infinity`");
        assert_compile_error("console;", "global `console`");
        assert_compile_error("let x = globalThis;", "global `globalThis`");
        // Shapes the parser or AST bridge declines still emit nothing.
        for source in [
            "console.log(7); const [a] = [1];",
            "console.log(7); let {a} = {a: 1};",
            "console.log(7); let undefined = 1;",
            "console.log(7); console.log(undefined);",
            "console.log(7); let \\u0061 = 1;",
        ] {
            let error = run_source(source).unwrap_err();
            assert!(error.output.is_empty(), "{source}: {error}");
        }
    }

    fn single_declarator(
        ast: &mut Program,
    ) -> &mut coding_adventures_javascript_ast::VariableDeclarator {
        let ProgramItem::Statement(Statement::Declaration(Declaration::VariableDeclaration(
            declaration,
        ))) = &mut ast.body[0]
        else {
            panic!("expected a variable declaration");
        };
        &mut declaration.declarations[0]
    }

    #[test]
    fn typed_ast_declarations_lower_directly_in_both_item_positions() {
        let ast =
            parse_javascript_program("let x = 2; console.log(x, -x);", EsVersion::Es2020).unwrap();
        let module = compile_ast(&ast, "typed-let").unwrap();
        let ops: Vec<_> = module.functions[0]
            .instructions
            .iter()
            .map(|i| i.op.as_str())
            .collect();
        assert_eq!(ops, ["const", "mov", "neg", "call_builtin", "ret_void"]);

        // A caller may also place the declaration directly at program level.
        let mut moved = ast.clone();
        let ProgramItem::Statement(Statement::Declaration(declaration)) = moved.body.remove(0)
        else {
            panic!("expected a declaration statement");
        };
        moved.body.insert(0, ProgramItem::Declaration(declaration));
        let shape = |module: &IIRModule| -> Vec<_> {
            module.functions[0]
                .instructions
                .iter()
                .map(|i| (i.op.clone(), i.dest.clone(), i.srcs.clone()))
                .collect()
        };
        assert_eq!(
            shape(&compile_ast(&moved, "program-level-let").unwrap()),
            shape(&module)
        );

        // Caller-built names must be plain IdentifierName spellings.
        for forged in ["", "a b", "a.b", "1a", "a\n", "\\u0061"] {
            let mut bad = ast.clone();
            let BindingTarget::Identifier(id) = &mut single_declarator(&mut bad).id;
            id.name = forged.into();
            let error = compile_ast(&bad, "forged-binding").unwrap_err();
            assert!(error.contains("identifier spelling"), "{forged:?}: {error}");
        }
        let mut bad = ast.clone();
        single_declarator(&mut bad).init = Some(Expression::Identifier(
            coding_adventures_javascript_ast::Identifier {
                cv: None,
                name: "a.b".into(),
            },
        ));
        let error = compile_ast(&bad, "forged-reference").unwrap_err();
        assert!(error.contains("identifier spelling"), "{error}");
        let mut bad = ast.clone();
        single_declarator(&mut bad).init = None;
        assert!(compile_ast(&bad, "forged-no-init")
            .unwrap_err()
            .contains("requires an initializer"));
    }

    #[test]
    fn direct_ast_budget_checks_declarator_initializers() {
        use coding_adventures_javascript_ast::expression::{BinaryExpression, UnaryExpression};
        let base = parse_javascript_program("let x = 1;", EsVersion::Es2020).unwrap();

        let mut deep = base.clone();
        let init = single_declarator(&mut deep).init.as_mut().unwrap();
        for _ in 0..MAX_EXPRESSION_DEPTH {
            *init = Expression::UnaryExpression(UnaryExpression {
                cv: None,
                operator: UnaryOperator::Negate,
                prefix: true,
                argument: Box::new(init.clone()),
            });
        }
        let error = compile_ast(&deep, "forged-deep-initializer").unwrap_err();
        assert!(error.contains("depth limit"), "{error}");

        // 2^14 - 1 = 16,383 initializer nodes plus the item, declarator and
        // binding charges exceed the 16,384-node budget.
        let mut wide = base.clone();
        let init = single_declarator(&mut wide).init.as_mut().unwrap();
        for _ in 0..13 {
            *init = Expression::BinaryExpression(BinaryExpression {
                cv: None,
                operator: BinaryOperator::Add,
                left: Box::new(init.clone()),
                right: Box::new(init.clone()),
            });
        }
        let error = compile_ast(&wide, "forged-wide-initializer").unwrap_err();
        assert!(error.contains("node limit"), "{error}");

        // Declarations alone pay three nodes each (item, declarator, binding
        // name) plus their one-node initializer: 4,096 fit, one more does not.
        let fits: String = (0..MAX_AST_NODES / 4)
            .map(|i| format!("let v{i} = 1;"))
            .collect();
        let ast = parse_javascript_program(&fits, EsVersion::Es2020).unwrap();
        assert!(compile_ast(&ast, "declaration-budget").is_ok());
        let over = format!("{fits}let last = 1;");
        let ast = parse_javascript_program(&over, EsVersion::Es2020).unwrap();
        assert!(compile_ast(&ast, "declaration-budget")
            .unwrap_err()
            .contains("node limit"));
    }

    #[test]
    fn reference_error_builtins_reject_malformed_calls() {
        let render = |name: &str| format!("ReferenceError: {name} is not defined");
        for args in [
            vec![],
            vec![Value::Float(1.0)],
            vec![Value::Str("a".into()); 2],
        ] {
            let error = raise_reference_error(&args, render).unwrap_err();
            assert!(error.to_string().contains("expects one binding name"));
        }
        let error = raise_reference_error(&[Value::Str("a".into())], render).unwrap_err();
        assert_eq!(error.to_string(), "ReferenceError: a is not defined");
    }
}
