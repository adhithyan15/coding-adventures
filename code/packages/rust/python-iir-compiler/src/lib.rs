//! Native Python grammar-tree to InterpreterIR lowering for LANG79's float pilot.
//!
//! The source is parsed by the Rust Python parser and executed by `vm-core`.
//! Semantic IR and a host Python runtime are not part of the execution path.
//!
//! # What a program may contain
//!
//! A program is a sequence of module-level statements, each one of:
//!
//! ```text
//! <float expression>                   # evaluated, result discarded
//! print(<zero to three float exprs>)   # written to captured stdout
//! NAME = <float expression>            # module-level binding (rebinding OK)
//! ```
//!
//! A float expression is built from float literals, bound NAMEs, parentheses,
//! unary `+`/`-` and binary `+ - * /`. Everything else is rejected at compile
//! time with a message naming the unsupported form.
//!
//! # Where values live in the generated IIR
//!
//! The VM addresses values by register *name*. Two disjoint namespaces exist:
//!
//! | Register        | Spelling          | Written by                       |
//! |-----------------|-------------------|----------------------------------|
//! | temporary       | `py0`, `py1`, ... | every `const`, `neg`, `add`, ... |
//! | Python binding  | `py:x`, `py:total`| the `mov` that ends `x = ...`    |
//!
//! A Python identifier can never contain `:`, and a temporary is `py` followed
//! only by decimal digits, so `x = ...` cannot overwrite a temporary however
//! the user names it. Builtins (`py_print_float`, `py_name_error`, ...) are not
//! registers at all: `call_builtin` reads its first operand as a literal name.

use coding_adventures_python_parser::parse_python;
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::collections::HashSet;
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
    let captured_empty = Arc::clone(&output);
    let captured_two = Arc::clone(&output);
    let captured_three = Arc::clone(&output);
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
    // A read of a name with no earlier binding lowers to this builtin. It never
    // returns a value: it raises CPython's exact NameError text so that the
    // run stops at that point with every earlier print still in `output`.
    vm.builtins_mut().register("py_name_error", |args| {
        let [Value::Str(name)] = args else {
            return Err(VMError::Custom("py_name_error expects one name".into()));
        };
        Err(VMError::Custom(format!(
            "NameError: name '{name}' is not defined"
        )))
    });
    vm.builtins_mut().register("py_print_empty", move |args| {
        if !args.is_empty() {
            return Err(VMError::Custom(
                "py_print_empty expects no arguments".into(),
            ));
        }
        let mut sink = captured_empty
            .lock()
            .map_err(|_| VMError::Custom("Python output lock poisoned".into()))?;
        if sink.len() >= 1_000_000 {
            return Err(VMError::Custom("Python output limit exceeded".into()));
        }
        sink.push('\n');
        Ok(Value::Null)
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
    vm.builtins_mut()
        .register("py_print_two_floats", move |args| {
            let [Value::Float(left), Value::Float(right)] = args else {
                return Err(VMError::Custom(
                    "py_print_two_floats expects two floats".into(),
                ));
            };
            let first = format_python_float(*left).map_err(VMError::Custom)?;
            let second = format_python_float(*right).map_err(VMError::Custom)?;
            let line = format!("{first} {second}\n");
            let mut sink = captured_two
                .lock()
                .map_err(|_| VMError::Custom("Python output lock poisoned".into()))?;
            if sink.len() + line.len() > 1_000_000 {
                return Err(VMError::Custom("Python output limit exceeded".into()));
            }
            sink.push_str(&line);
            Ok(Value::Null)
        });
    vm.builtins_mut()
        .register("py_print_three_floats", move |args| {
            let [Value::Float(first), Value::Float(second), Value::Float(third)] = args else {
                return Err(VMError::Custom(
                    "py_print_three_floats expects three floats".into(),
                ));
            };
            let first = format_python_float(*first).map_err(VMError::Custom)?;
            let second = format_python_float(*second).map_err(VMError::Custom)?;
            let third = format_python_float(*third).map_err(VMError::Custom)?;
            let line = format!("{first} {second} {third}\n");
            let mut sink = captured_three
                .lock()
                .map_err(|_| VMError::Custom("Python output lock poisoned".into()))?;
            if sink.len() + line.len() > 1_000_000 {
                return Err(VMError::Custom("Python output limit exceeded".into()));
            }
            sink.push_str(&line);
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
    /// Python names bound by an earlier statement of this module.
    ///
    /// The subset has no branches, loops, functions or `del`, so execution is
    /// one straight line: once a statement binds `x`, every later program
    /// point sees `x` bound, and every earlier one sees it unbound. That makes
    /// "is this read a NameError?" a compile-time question. Statement order is
    /// lowering order, so this set is exactly the bound names at the statement
    /// being compiled.
    bound: HashSet<String>,
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
        // Every Python 3.12 simple statement we accept parses as `assign_stmt`;
        // a bare expression is the degenerate assignment with no `=` tail:
        //
        //   assign_stmt = expression_list
        //                 [ COLON expression [ EQUALS expression_list ]   (annotated)
        //                 | EQUALS expression_list { EQUALS expression_list } (plain)
        //                 | augmented_assign_op expression_list ] ;        (x += ...)
        //
        // The child count and the second child's kind tell the forms apart.
        let assign = only_node(small, "assign_stmt")?;
        match assign.children.as_slice() {
            [ASTNodeOrToken::Node(expression_list)]
                if expression_list.rule_name == "expression_list" =>
            {
                self.compile_expression_statement(only_node(expression_list, "expression")?)
            }
            [ASTNodeOrToken::Node(target), ASTNodeOrToken::Token(operator), ASTNodeOrToken::Node(value)] =>
            {
                if operator.type_ == TokenType::Colon {
                    return Err(ANNOTATED_ASSIGNMENT.into());
                }
                // Direct-AST callers can hand us any token here. Only the
                // lexer's own EQUALS token (enum kind, no override, exact "=")
                // makes this a binding; `==`, `:=` or a forged kind do not.
                if !syntax_token(operator, TokenType::Equals, "=") {
                    return Err("Python assignment requires an actual = token".into());
                }
                self.compile_binding(target, value)
            }
            [ASTNodeOrToken::Node(_), ASTNodeOrToken::Node(operator), ..]
                if operator.rule_name == "augmented_assign_op" =>
            {
                Err(
                    "augmented assignment such as x += 1.0 is outside the native float pilot"
                        .into(),
                )
            }
            [ASTNodeOrToken::Node(_), ASTNodeOrToken::Token(colon), ..]
                if colon.type_ == TokenType::Colon =>
            {
                Err(ANNOTATED_ASSIGNMENT.into())
            }
            [ASTNodeOrToken::Node(_), ASTNodeOrToken::Token(_), ASTNodeOrToken::Node(_), ASTNodeOrToken::Token(_), ..] => {
                Err(
                    "chained assignment such as a = b = 1.0 is outside the native float pilot"
                        .into(),
                )
            }
            _ => Err("unsupported Python assignment statement shape".into()),
        }
    }

    /// Lower `NAME = <float expression>`.
    ///
    /// ```text
    ///   x = x + 1.0        py0 = const 1.0
    ///                      py1 = add py:x, py0     <- reads the *old* x
    ///                      py:x = mov py1          <- then binds
    /// ```
    ///
    /// Python evaluates the whole right-hand side before it binds the target,
    /// so the value is lowered first, the `mov` comes last, and only then is
    /// the name added to `bound`. If the value raises (division by zero, or a
    /// NameError for an unbound read) the binding never happens, exactly as in
    /// CPython, and the run stops there anyway.
    fn compile_binding(
        &mut self,
        target: &GrammarASTNode,
        value: &GrammarASTNode,
    ) -> Result<(), String> {
        let name = binding_target(target)?;
        let [ASTNodeOrToken::Node(expression)] = value.children.as_slice() else {
            return Err("Python assignment value must be one float expression, not a tuple".into());
        };
        if value.rule_name != "expression_list" || expression.rule_name != "expression" {
            return Err("unsupported Python assignment value".into());
        }
        // `x = print(...)` is valid Python, but it binds None, which no float
        // operation in this pilot can consume. Say so rather than reporting a
        // generic call-shape error.
        if print_argument(expression)?.is_some() {
            return Err(
                "binding the None result of print() is outside the native float pilot".into(),
            );
        }
        let operand = self.compile_number(expression)?;
        self.emit("mov", Some(binding_register(name)), vec![operand], "f64");
        self.bound.insert(name.to_owned());
        Ok(())
    }

    /// Lower an expression statement: either a `print(...)` call or a float
    /// expression whose value is discarded (but whose errors still raise).
    fn compile_expression_statement(&mut self, expression: &GrammarASTNode) -> Result<(), String> {
        match print_argument(expression)? {
            Some(PrintCall::Empty) => {
                self.emit(
                    "call_builtin",
                    None,
                    vec![Operand::Var("py_print_empty".into())],
                    "void",
                );
            }
            Some(PrintCall::One(argument)) => {
                let value = self.compile_number(argument)?;
                self.emit(
                    "call_builtin",
                    None,
                    vec![Operand::Var("py_print_float".into()), value],
                    "void",
                );
            }
            Some(PrintCall::Two(first, second)) => {
                let left = self.compile_number(first)?;
                let right = self.compile_number(second)?;
                self.emit(
                    "call_builtin",
                    None,
                    vec![Operand::Var("py_print_two_floats".into()), left, right],
                    "void",
                );
            }
            Some(PrintCall::Three(first, second, third)) => {
                let first = self.compile_number(first)?;
                let second = self.compile_number(second)?;
                let third = self.compile_number(third)?;
                self.emit(
                    "call_builtin",
                    None,
                    vec![
                        Operand::Var("py_print_three_floats".into()),
                        first,
                        second,
                        third,
                    ],
                    "void",
                );
            }
            None => {
                self.compile_number(expression)?;
            }
        }
        Ok(())
    }

    /// Lower a NAME read inside a float expression.
    ///
    /// | Name state at this program point      | Lowering                      |
    /// |---------------------------------------|-------------------------------|
    /// | bound by an earlier statement          | the binding register `py:x`   |
    /// | `print`                                | compile error (call-only)     |
    /// | unbound builtin or dunder name         | compile error (unsupported)   |
    /// | unbound ordinary name                  | `call_builtin py_name_error`  |
    ///
    /// The bound case reads `py:x` directly instead of copying it into a
    /// temporary: no expression in this subset can rebind a name, so the
    /// register cannot change between this read and its use.
    ///
    /// The builtin rows exist because CPython would *not* raise NameError
    /// there. `print(float)` prints `<class 'float'>` and `print(__name__)`
    /// prints `__main__`; neither is a float this pilot can display, so the
    /// honest answer is "unsupported", not an invented NameError.
    fn compile_name(&mut self, token: &Token) -> Result<Operand, String> {
        let name = token.value.as_str();
        if !python_name_spelling(name) {
            return Err("invalid Python NAME token spelling".into());
        }
        if name == "print" {
            return Err(
                "print is only supported as a direct call in the native float pilot".into(),
            );
        }
        if self.bound.contains(name) {
            return Ok(Operand::Var(binding_register(name)));
        }
        if is_dunder(name) || PYTHON_312_BUILTINS.binary_search(&name).is_ok() {
            return Err(format!(
                "Python builtin or module attribute {name} is not available as a value in the native float pilot"
            ));
        }
        // Statically unbound: emit the raising builtin at exactly this point.
        // It is given a destination so the instructions that would consume
        // the value still have a well-formed operand; that code is never
        // reached because the builtin always returns an error.
        let dest = self.register();
        self.emit(
            "call_builtin",
            Some(dest.clone()),
            vec![
                Operand::Var("py_name_error".into()),
                Operand::Str(name.into()),
            ],
            "f64",
        );
        Ok(Operand::Var(dest))
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
                if !arithmetic_token(sign) || inner.rule_name != "factor" {
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
                let [ASTNodeOrToken::Token(open), ASTNodeOrToken::Node(body), ASTNodeOrToken::Token(close)] =
                    node.children.as_slice()
                else {
                    return Err("unsupported Python parenthesized expression".into());
                };
                if !syntax_token(open, TokenType::LParen, "(")
                    || !syntax_token(close, TokenType::RParen, ")")
                    || body.rule_name != "paren_body"
                {
                    return Err("unsupported Python parenthesized expression".into());
                }
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
        let (pairs, remainder) = node.children[1..].as_chunks::<2>();
        for pair in pairs {
            let (ASTNodeOrToken::Token(operator), ASTNodeOrToken::Node(right_node)) =
                (&pair[0], &pair[1])
            else {
                return Err("unsupported Python arithmetic expression".into());
            };
            if right_node.rule_name != operand_rule {
                return Err("unsupported Python arithmetic expression".into());
            }
            let Some((_, op)) = operators.iter().find(|(symbol, _)| {
                operator.value == symbol.to_string() && arithmetic_token(operator)
            }) else {
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
        if !remainder.is_empty() {
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
                if token.type_ != TokenType::Name || !float_spelling(&token.value) {
                    return Err("invalid Python FLOAT token kind or raw spelling".into());
                }
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
            // The versioned lexer emits identifiers as plain `Name` tokens with
            // no named override (FLOAT and INT literals are `Name` *with* one).
            // Keywords such as True arrive as `Keyword` and fall through below.
            ASTNodeOrToken::Token(token)
                if token.type_ == TokenType::Name && token.type_name.is_none() =>
            {
                self.compile_name(token)
            }
            ASTNodeOrToken::Node(paren) if paren.rule_name == "paren_expr" => {
                self.compile_number(paren)
            }
            _ => Err("unsupported Python atom in native float pilot".into()),
        }
    }
}

// The versioned lexer aliases FLOAT_EXP and FLOAT_DOT to FLOAT. Direct AST
// callers bypass that lexer, so recognize its three complete ASCII forms before
// erasing underscores. A digit group is DIGIT ("_"? DIGIT)*; the optional
// underscore must have a digit on both sides. Each byte is inspected at most
// twice, and the existing AST text budget bounds both scanning and conversion.
fn float_spelling(spelling: &str) -> bool {
    let bytes = spelling.as_bytes();
    let mut position = 0;
    let integral = float_digit_group(bytes, &mut position);
    let dotted = bytes.get(position) == Some(&b'.');
    if dotted {
        position += 1;
        // Unlike full Python, the current lexer requires fractional digits
        // after the dot. Preserve that subset rather than silently widening it.
        if !float_digit_group(bytes, &mut position) {
            return false;
        }
    } else if !integral {
        return false;
    }
    let exponent = matches!(bytes.get(position), Some(b'e' | b'E'));
    if exponent {
        position += 1;
        if matches!(bytes.get(position), Some(b'+' | b'-')) {
            position += 1;
        }
        if !float_digit_group(bytes, &mut position) {
            return false;
        }
    }
    (dotted || exponent) && position == bytes.len()
}

fn float_digit_group(bytes: &[u8], position: &mut usize) -> bool {
    if !bytes.get(*position).is_some_and(u8::is_ascii_digit) {
        return false;
    }
    *position += 1;
    loop {
        if bytes.get(*position).is_some_and(u8::is_ascii_digit) {
            *position += 1;
        } else if bytes.get(*position) == Some(&b'_')
            && bytes.get(*position + 1).is_some_and(u8::is_ascii_digit)
        {
            *position += 2;
        } else {
            return true;
        }
    }
}

// A caller can build tokens without the lexer. Check the structural kind,
// effective-type override, and spelling together before treating one as syntax.
// Canonical lexer punctuation carries no named override, just its enum kind.
fn syntax_token(token: &Token, kind: TokenType, spelling: &str) -> bool {
    token.type_ == kind && token.type_name.is_none() && token.value == spelling
}

fn arithmetic_token(token: &Token) -> bool {
    match token.value.as_str() {
        "+" => syntax_token(token, TokenType::Plus, "+"),
        "-" => syntax_token(token, TokenType::Minus, "-"),
        "*" => syntax_token(token, TokenType::Star, "*"),
        "/" => syntax_token(token, TokenType::Slash, "/"),
        _ => false,
    }
}

const ANNOTATED_ASSIGNMENT: &str =
    "annotated assignment such as x: float = 1.0 is outside the native float pilot";

/// The VM register that holds Python module global `name` (see the namespace
/// table at the top of this file).
fn binding_register(name: &str) -> String {
    format!("py:{name}")
}

/// Find the single NAME an assignment target binds, or explain why not.
///
/// The parser wraps even a bare `x` in the full precedence ladder:
///
/// ```text
/// expression_list > expression > or_expr > ... > primary > atom > NAME "x"
/// ```
///
/// so the walk follows single-child nodes down to where the tree branches or
/// ends in a token. What it finds there classifies the target:
///
/// | Source            | Walk stops at                   | Result               |
/// |-------------------|---------------------------------|----------------------|
/// | `x`               | `atom` holding one NAME token   | bind `x`             |
/// | `a, b` / `x,`     | `expression_list`, 2+ children  | tuple target error   |
/// | `a.b` / `a[0]`    | `primary` with a `suffix`       | attribute/subscript  |
/// | `(x)` / `[x]`     | `atom` holding a node           | single-name error    |
/// | `-x`              | `factor` with a sign token      | single-name error    |
/// | `True` / `1.0`    | `atom` holding a non-NAME token | not-a-name error     |
///
/// The AST depth budget has already bounded this loop.
fn binding_target(target: &GrammarASTNode) -> Result<&str, String> {
    if target.rule_name != "expression_list" {
        return Err("unsupported Python assignment target".into());
    }
    let [ASTNodeOrToken::Node(first)] = target.children.as_slice() else {
        return Err(
            "native Python assignment requires one name target; tuple, list and starred targets are unsupported"
                .into(),
        );
    };
    let mut node = first;
    while let [ASTNodeOrToken::Node(inner)] = node.children.as_slice() {
        node = inner;
    }
    if node.rule_name == "primary" && node.children.len() > 1 {
        return Err(
            "attribute, subscript and call assignment targets are outside the native float pilot"
                .into(),
        );
    }
    let [ASTNodeOrToken::Token(token)] = node.children.as_slice() else {
        return Err(
            "native Python assignment requires one name target; tuple, list and starred targets are unsupported"
                .into(),
        );
    };
    if node.rule_name != "atom" || token.type_ != TokenType::Name || token.type_name.is_some() {
        return Err("Python assignment target must be a name".into());
    }
    let name = token.value.as_str();
    if !python_name_spelling(name) {
        return Err("invalid Python NAME token spelling".into());
    }
    match name {
        "print" => {
            Err("binding print is outside the native float pilot; print stays the builtin".into())
        }
        // CPython: "SyntaxError: cannot assign to __debug__".
        "__debug__" => Err("cannot assign to __debug__".into()),
        _ => Ok(name),
    }
}

/// Python 3.12's hard keywords (`keyword.kwlist`). The lexer tags these as
/// `Keyword`, but a caller-built `Name` token could spell one; such a token is
/// not an identifier. Soft keywords (`match`, `case`, `type`, `_`) are ordinary
/// identifiers outside their statement contexts and remain valid names.
#[rustfmt::skip]
const PYTHON_KEYWORDS: [&str; 35] = [
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

/// Check a NAME spelling against the versioned lexer pattern
/// `/[a-zA-Z_][a-zA-Z0-9_]*/` (ASCII only) and the hard-keyword list. One
/// linear byte scan; the AST text budget bounds its length.
fn python_name_spelling(spelling: &str) -> bool {
    let bytes = spelling.as_bytes();
    let Some((first, rest)) = bytes.split_first() else {
        return false;
    };
    (first.is_ascii_alphabetic() || *first == b'_')
        && rest
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        && !PYTHON_KEYWORDS.contains(&spelling)
}

/// `__name__`-shaped identifiers. In a CPython script several of these
/// (`__name__`, `__doc__`, `__spec__`, ...) exist before any assignment, and
/// `__debug__` is a builtin, so an unbound read is rejected as unsupported
/// rather than lowered to a NameError that CPython would not raise.
fn is_dunder(name: &str) -> bool {
    name.len() > 4 && name.starts_with("__") && name.ends_with("__")
}

/// Non-dunder names in Python 3.12's `builtins` module as a script sees it
/// (including the `site`-added `exit`, `quit`, `help`, `copyright`, `credits`
/// and `license`). Reading one before the module binds it resolves to the
/// builtin object in CPython, not to a NameError, so the compiler must know
/// them. Kept in byte order for `binary_search`.
#[rustfmt::skip]
const PYTHON_312_BUILTINS: [&str; 149] = [
    "ArithmeticError", "AssertionError", "AttributeError", "BaseException", "BaseExceptionGroup",
    "BlockingIOError", "BrokenPipeError", "BufferError", "BytesWarning", "ChildProcessError",
    "ConnectionAbortedError", "ConnectionError", "ConnectionRefusedError", "ConnectionResetError",
    "DeprecationWarning", "EOFError", "Ellipsis", "EncodingWarning", "EnvironmentError",
    "Exception", "ExceptionGroup", "False", "FileExistsError", "FileNotFoundError",
    "FloatingPointError", "FutureWarning", "GeneratorExit", "IOError", "ImportError",
    "ImportWarning", "IndentationError", "IndexError", "InterruptedError", "IsADirectoryError",
    "KeyError", "KeyboardInterrupt", "LookupError", "MemoryError", "ModuleNotFoundError",
    "NameError", "None", "NotADirectoryError", "NotImplemented", "NotImplementedError", "OSError",
    "OverflowError", "PendingDeprecationWarning", "PermissionError", "ProcessLookupError",
    "RecursionError", "ReferenceError", "ResourceWarning", "RuntimeError", "RuntimeWarning",
    "StopAsyncIteration", "StopIteration", "SyntaxError", "SyntaxWarning", "SystemError",
    "SystemExit", "TabError", "TimeoutError", "True", "TypeError", "UnboundLocalError",
    "UnicodeDecodeError", "UnicodeEncodeError", "UnicodeError", "UnicodeTranslateError",
    "UnicodeWarning", "UserWarning", "ValueError", "Warning", "ZeroDivisionError", "abs", "aiter",
    "all", "anext", "any", "ascii", "bin", "bool", "breakpoint", "bytearray", "bytes", "callable",
    "chr", "classmethod", "compile", "complex", "copyright", "credits", "delattr", "dict", "dir",
    "divmod", "enumerate", "eval", "exec", "exit", "filter", "float", "format", "frozenset",
    "getattr", "globals", "hasattr", "hash", "help", "hex", "id", "input", "int", "isinstance",
    "issubclass", "iter", "len", "license", "list", "locals", "map", "max", "memoryview", "min",
    "next", "object", "oct", "open", "ord", "pow", "print", "property", "quit", "range", "repr",
    "reversed", "round", "set", "setattr", "slice", "sorted", "staticmethod", "str", "sum",
    "super", "tuple", "type", "vars", "zip",
];

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

enum PrintCall<'a> {
    Empty,
    One(&'a GrammarASTNode),
    Two(&'a GrammarASTNode, &'a GrammarASTNode),
    Three(&'a GrammarASTNode, &'a GrammarASTNode, &'a GrammarASTNode),
}

fn positional_argument(node: &GrammarASTNode) -> Result<&GrammarASTNode, String> {
    if node.rule_name != "argument" {
        return Err("native Python print requires positional expression arguments".into());
    }
    only_node(node, "expression")
}

fn print_argument(expression: &GrammarASTNode) -> Result<Option<PrintCall<'_>>, String> {
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
    if callee.value != "print" {
        return Ok(None);
    }
    if atom.rule_name != "atom"
        || callee.type_ != TokenType::Name
        || callee.type_name.is_some()
        || suffix.rule_name != "suffix"
    {
        return Err("native Python print requires a name-call shape".into());
    }
    match suffix.children.as_slice() {
        [ASTNodeOrToken::Token(open), ASTNodeOrToken::Token(close)]
            if open.value == "("
                && open.type_ == TokenType::LParen
                && open.type_name.is_none()
                && close.value == ")"
                && close.type_ == TokenType::RParen
                && close.type_name.is_none() =>
        {
            Ok(Some(PrintCall::Empty))
        }
        [ASTNodeOrToken::Token(open), ASTNodeOrToken::Node(args), ASTNodeOrToken::Token(close)]
            if open.value == "("
                && open.type_ == TokenType::LParen
                && open.type_name.is_none()
                && close.value == ")"
                && close.type_ == TokenType::RParen
                && close.type_name.is_none() =>
        {
            if args.rule_name != "arguments" {
                return Err("native Python print requires positional expression arguments".into());
            }
            match args.children.as_slice() {
                [ASTNodeOrToken::Node(argument)] => {
                    Ok(Some(PrintCall::One(positional_argument(argument)?)))
                }
                [ASTNodeOrToken::Node(first), ASTNodeOrToken::Token(comma), ASTNodeOrToken::Node(second)]
                    if comma.value == ","
                        && comma.type_ == TokenType::Comma
                        && comma.type_name.is_none() =>
                {
                    Ok(Some(PrintCall::Two(
                        positional_argument(first)?,
                        positional_argument(second)?,
                    )))
                }
                [ASTNodeOrToken::Node(first), ASTNodeOrToken::Token(comma_one), ASTNodeOrToken::Node(second), ASTNodeOrToken::Token(comma_two), ASTNodeOrToken::Node(third)]
                    if [comma_one, comma_two].iter().all(|comma| {
                        comma.value == ","
                            && comma.type_ == TokenType::Comma
                            && comma.type_name.is_none()
                    }) =>
                {
                    Ok(Some(PrintCall::Three(
                        positional_argument(first)?,
                        positional_argument(second)?,
                        positional_argument(third)?,
                    )))
                }
                _ => Err(
                    "native Python print requires at most three positional float expressions"
                        .into(),
                ),
            }
        }
        _ => Err("native Python print requires zero to three expression arguments".into()),
    }
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
    fn empty_print_writes_one_newline_and_preserves_earlier_output() {
        assert_eq!(run_source("print()\nprint(1.0)\n").unwrap(), "\n1.0\n");
        let error = run_source("print()\nprint(1.0 / 0.0)\n").unwrap_err();
        assert_eq!(error.output, "\n");
        assert!(error.message.contains("ZeroDivisionError"));
    }

    #[test]
    fn two_float_print_uses_one_space_and_keeps_prior_output_on_error() {
        assert_eq!(
            run_source("print(1.0 + 2.0, -0.0)\nprint(1.0, 2.0 / 4.0)\n").unwrap(),
            "3.0 -0.0\n1.0 0.5\n"
        );
        let error = run_source("print(3.0, 4.0)\nprint(2.0, 1.0 / 0.0)\n").unwrap_err();
        assert_eq!(error.output, "3.0 4.0\n");
        assert!(error.message.contains("ZeroDivisionError"));
        let display_error = run_source("print(1.0)\nprint(2.0, 1e20)\n").unwrap_err();
        assert_eq!(display_error.output, "1.0\n");
        assert!(display_error.message.contains("display outside"));
    }

    #[test]
    fn three_float_print_uses_two_spaces_and_keeps_prior_output_on_error() {
        assert_eq!(
            run_source("print(1.0 + 2.0, -0.0, 2.5)\n").unwrap(),
            "3.0 -0.0 2.5\n"
        );
        let error = run_source("print(1.0, 2.0, 3.0)\nprint(2.0, 3.0, 1.0 / 0.0)\n").unwrap_err();
        assert_eq!(error.output, "1.0 2.0 3.0\n");
        assert!(error.message.contains("ZeroDivisionError"));
        let second_error = run_source("print(1.0)\nprint(2.0, 1.0 / 0.0, 3.0)\n").unwrap_err();
        assert_eq!(second_error.output, "1.0\n");
        let display_error = run_source("print(1.0)\nprint(2.0, 1e20, 3.0)\n").unwrap_err();
        assert_eq!(display_error.output, "1.0\n");
        assert!(display_error.message.contains("display outside"));
        let third_display_error = run_source("print(1.0)\nprint(2.0, 3.0, 1e20)\n").unwrap_err();
        assert_eq!(third_display_error.output, "1.0\n");
        assert!(third_display_error.message.contains("display outside"));
    }

    #[test]
    fn direct_ast_multi_argument_print_requires_actual_comma_tokens() {
        fn forge_comma(node: &mut GrammarASTNode, remaining: &mut usize, named: bool) -> bool {
            for child in &mut node.children {
                match child {
                    ASTNodeOrToken::Token(token) if token.value == "," => {
                        if *remaining != 0 {
                            *remaining -= 1;
                            continue;
                        }
                        if named {
                            token.type_name = Some("STRING".into());
                        } else {
                            token.type_ = TokenType::String;
                        }
                        return true;
                    }
                    ASTNodeOrToken::Node(inner) => {
                        if forge_comma(inner, remaining, named) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            false
        }
        for (source, count) in [("print(1.0, 2.0)\n", 1), ("print(1.0, 2.0, 3.0)\n", 2)] {
            for index in 0..count {
                for named in [false, true] {
                    let mut ast = parse_python(source, "3.12").unwrap();
                    let mut remaining = index;
                    assert!(forge_comma(&mut ast, &mut remaining, named));
                    assert!(compile_ast(&ast, "forged").is_err());
                }
            }
        }
    }

    #[test]
    fn direct_ast_print_call_rejects_forged_token_kinds() {
        fn forge_type(node: &mut GrammarASTNode, value: &str, variant: bool) -> bool {
            for child in &mut node.children {
                match child {
                    ASTNodeOrToken::Token(token) if token.value == value => {
                        if variant {
                            token.type_ = TokenType::String;
                        } else {
                            token.type_name = Some("STRING".into());
                        }
                        return true;
                    }
                    ASTNodeOrToken::Node(inner) => {
                        if forge_type(inner, value, variant) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            false
        }

        for source in ["print()\n", "print(1.0)\n", "print(1.0, 2.0, 3.0)\n"] {
            for value in ["print", "(", ")"] {
                for variant in [false, true] {
                    let mut ast = parse_python(source, "3.12").unwrap();
                    assert!(forge_type(&mut ast, value, variant));
                    assert!(
                        compile_ast(&ast, "forged").is_err(),
                        "{source:?} {value:?} {variant:?}"
                    );
                }
            }
        }

        fn forge_atom_rule(node: &mut GrammarASTNode) -> bool {
            if node.rule_name == "atom"
                && matches!(node.children.as_slice(), [ASTNodeOrToken::Token(token)] if token.value == "print")
            {
                node.rule_name = "forged_atom".into();
                return true;
            }
            for child in &mut node.children {
                if let ASTNodeOrToken::Node(inner) = child {
                    if forge_atom_rule(inner) {
                        return true;
                    }
                }
            }
            false
        }
        let mut ast = parse_python("print()\n", "3.12").unwrap();
        assert!(forge_atom_rule(&mut ast));
        assert!(compile_ast(&ast, "forged").is_err());
    }

    #[test]
    fn unsupported_python_semantics_are_rejected() {
        for source in [
            "print(1 + 2)\n",
            "print(1.0 // 2.0)\n",
            "x = 1\n",
            "print(1.0, 2.0, 3.0, 4.0)\n",
            "print(1.0, 2.0, end=3.0)\n",
            "print(1.0, end=2.0)\n",
            "other()\n",
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
    // These mutations retain the original grammar tree and alter one trust
    // boundary field at a time. Parsing source first makes each accepted token
    // and wrapper shape independent of the lowering implementation.
    fn forge_arithmetic_token(node: &mut GrammarASTNode, spelling: &str, mode: usize) -> bool {
        let index = if node.rule_name == "paren_expr" {
            match spelling {
                "(" => Some(0),
                ")" => Some(2),
                _ => None,
            }
        } else if matches!(node.rule_name.as_str(), "arith" | "term" | "factor") {
            node.children.iter().position(
                |child| matches!(child, ASTNodeOrToken::Token(token) if token.value == spelling),
            )
        } else {
            None
        };
        if let Some(index) = index {
            if mode == 4 {
                node.children[index] = ASTNodeOrToken::Node(empty_node("forged_token"));
            } else if let ASTNodeOrToken::Token(token) = &mut node.children[index] {
                match mode {
                    0 => token.type_ = TokenType::String,
                    1 => token.type_name = Some("STRING".into()),
                    2 => token.type_name = Some(token.effective_type_name().to_owned()),
                    3 => token.value = "unsupported".into(),
                    _ => unreachable!(),
                }
            } else {
                return false;
            }
            return true;
        }
        for child in &mut node.children {
            if let ASTNodeOrToken::Node(inner) = child {
                if forge_arithmetic_token(inner, spelling, mode) {
                    return true;
                }
            }
        }
        false
    }

    #[test]
    fn direct_ast_arithmetic_rejects_forged_operator_and_group_tokens() {
        for (expression, spelling) in [
            ("+1.0", "+"),
            ("-1.0", "-"),
            ("1.0 + 2.0", "+"),
            ("1.0 - 2.0", "-"),
            ("1.0 * 2.0", "*"),
            ("1.0 / 2.0", "/"),
            ("(1.0 + 2.0)", "("),
            ("(1.0 + 2.0)", ")"),
        ] {
            // The same numeric lowering serves standalone expressions and
            // each print argument; exercise all four entry contexts.
            for source in [
                format!("{expression}\n"),
                format!("print({expression}, 3.0, 4.0)\n"),
                format!("print(3.0, {expression}, 4.0)\n"),
                format!("print(3.0, 4.0, {expression})\n"),
            ] {
                let ast = parse_python(&source, "3.12").unwrap();
                assert!(compile_ast(&ast, "valid").is_ok(), "{source}");
                for mode in 0..5 {
                    let mut forged = ast.clone();
                    assert!(forge_arithmetic_token(&mut forged, spelling, mode));
                    assert!(
                        compile_ast(&forged, "forged").is_err(),
                        "{source:?} {spelling:?} mode={mode}"
                    );
                }
            }
        }
    }

    #[test]
    fn direct_ast_grouping_rejects_forged_wrapper_and_extra_expression() {
        fn alter_body(node: &mut GrammarASTNode, extra: bool) -> bool {
            if node.rule_name == "paren_body" {
                if extra {
                    node.children
                        .push(ASTNodeOrToken::Node(empty_node("expression")));
                } else {
                    node.rule_name = "forged_body".into();
                }
                return true;
            }
            for child in &mut node.children {
                if let ASTNodeOrToken::Node(inner) = child {
                    if alter_body(inner, extra) {
                        return true;
                    }
                }
            }
            false
        }
        for extra in [false, true] {
            let mut ast = parse_python("print((1.0 + 2.0))\n", "3.12").unwrap();
            assert!(alter_body(&mut ast, extra));
            assert!(compile_ast(&ast, "forged").is_err());
        }
    }

    #[test]
    fn arithmetic_token_guards_preserve_source_runtime_behavior() {
        assert_eq!(
            run_source(
                "print((1.0 + 2.0) * 3.0, +0.0, -0.0)\nprint(7.0 - 3.0, 7.0 / 2.0)\nprint()\n"
            )
            .unwrap(),
            "9.0 0.0 -0.0\n4.0 3.5\n\n"
        );
        let failure = run_source("print(1.0)\nprint(2.0, (1.0 / -0.0), 3.0)\n").unwrap_err();
        assert_eq!(failure.output, "1.0\n");
        assert!(failure.message.contains("ZeroDivisionError"));
    }

    // Source trees establish the authentic FLOAT representation. Each mutation
    // changes one literal at the public AST boundary, bypassing source lexing.
    fn float_token_mut(node: &mut GrammarASTNode) -> Option<&mut Token> {
        for child in &mut node.children {
            match child {
                ASTNodeOrToken::Token(token) if token.value == "1.0" => return Some(token),
                ASTNodeOrToken::Node(inner) => {
                    if let Some(token) = float_token_mut(inner) {
                        return Some(token);
                    }
                }
                _ => {}
            }
        }
        None
    }

    #[test]
    fn direct_ast_float_tokens_require_canonical_kind_and_complete_spelling() {
        let invalid = [
            "1", "NaN", "inf", "Infinity", "-1.0", "+1.0", " 1.0", "1.0 ", "1.0\n", "_1.0", "1_.0",
            "1._0", "1.0_", "1__0.0", "1.0__1", "1e_2", "1e2_", "1e+_2", "1e", "1e+", "1.e2", "1.",
            ".", ".e2", "1.0f", "1.0j", "0x1.0", "1.0.0", "1.0e2x", "١.٠", "１.０", "1.0\0",
            "1.0\t", "1.0/2.0", "1.0e2e3",
        ];
        let sources = [
            "1.0\n",
            "print(1.0)\n",
            "print(1.0, 2.0)\n",
            "print(2.0, 1.0)\n",
            "print(1.0, 2.0, 3.0)\n",
            "print(2.0, 1.0, 3.0)\n",
            "print(2.0, 3.0, 1.0)\n",
        ];
        for source in sources {
            let mut original = parse_python(source, "3.12").unwrap();
            let token = float_token_mut(&mut original).unwrap();
            assert_eq!(token.type_, TokenType::Name);
            assert_eq!(token.type_name.as_deref(), Some("FLOAT"));
            assert!(compile_ast(&original, "valid").is_ok(), "{source}");
            for kind in [
                TokenType::String,
                TokenType::Number,
                TokenType::Keyword,
                TokenType::Plus,
            ] {
                let mut forged = original.clone();
                float_token_mut(&mut forged).unwrap().type_ = kind;
                assert!(compile_ast(&forged, "forged").is_err(), "{source} {kind:?}");
            }
            for named in [
                None,
                Some("INT"),
                Some("FLOAT_EXP"),
                Some("FLOAT_DOT"),
                Some("float"),
            ] {
                let mut forged = original.clone();
                float_token_mut(&mut forged).unwrap().type_name = named.map(str::to_owned);
                assert!(
                    compile_ast(&forged, "forged").is_err(),
                    "{source} {named:?}"
                );
            }
            for spelling in invalid {
                let mut forged = original.clone();
                float_token_mut(&mut forged).unwrap().value = spelling.into();
                assert!(
                    compile_ast(&forged, "forged").is_err(),
                    "{source} {spelling:?}"
                );
            }
        }
    }

    #[test]
    fn float_token_validation_preserves_versioned_source_forms_and_metadata() {
        for spelling in [
            "1.0", ".5", "00.5", "0_0.5", "1_0.0_5", "1e2", "1E+2", "1e-2", "1_0e0_2", ".5e+2",
            "00.5E-1", "1.0e9999", "1e-9999",
        ] {
            let source = format!("{spelling}\n");
            let tree = parse_python(&source, "3.12").unwrap();
            assert!(compile_ast(&tree, "source").is_ok(), "{spelling}");
        }
        let mut tree = parse_python("print(1.0)\n", "3.12").unwrap();
        float_token_mut(&mut tree).unwrap().cv = Some("999.0".into());
        let module = compile_ast(&tree, "raw").unwrap();
        let debug = format!("{module:?}");
        assert!(debug.contains("Float(1.0)"));
        assert!(!debug.contains("Float(999.0)"));
        assert_eq!(
            run_source("print(1_0.0_5, .5e1, -0e0)\n").unwrap(),
            "10.05 5.0 -0.0\n"
        );
        assert!(compile_source("1e9999\n", "overflow").is_ok());
        let error = run_source("print(1.0)\nprint(1e9999)\n").unwrap_err();
        assert_eq!(error.output, "1.0\n");
        assert!(error.message.contains("display outside"));
    }

    #[test]
    fn float_literal_scan_retains_preexisting_text_limit() {
        let mut tree = parse_python("1.0\n", "3.12").unwrap();
        float_token_mut(&mut tree).unwrap().value =
            format!("{}.0", "0".repeat(MAX_AST_FIELD_BYTES - 2));
        assert!(compile_ast(&tree, "boundary").is_ok());
        // The first token no longer spells 1.0, so reparse for the over-limit case.
        let mut too_long = parse_python("1.0\n", "3.12").unwrap();
        float_token_mut(&mut too_long).unwrap().value =
            format!("{}.0", "0".repeat(MAX_AST_FIELD_BYTES - 1));
        assert!(compile_ast(&too_long, "boundary")
            .unwrap_err()
            .contains("text limit"));
    }

    // ---------------------------------------------------------------
    // Module-level float bindings (LANG79 bounded follow-up). Expected
    // outputs and error texts were checked against host CPython, which
    // is only an oracle; nothing here runs it.
    // ---------------------------------------------------------------

    #[test]
    fn bindings_execute_rebind_and_read_in_every_position() {
        for (source, expected) in [
            ("x = 1.5\nprint(x * 2.0)\n", "3.0\n"),
            ("x = 1.0\nx = x + 1.0\nprint(x)\n", "2.0\n"),
            (
                "a = 2.0\nb = -a / 4.0\nprint(a, b, (a + b) * a)\n",
                "2.0 -0.5 3.0\n",
            ),
            ("x = 1.0\ny = x\nx = 5.0\nprint(x, y)\n", "5.0 1.0\n"),
            ("x = 3.0\nx\nprint(x)\n", "3.0\n"),
            ("x = 2.0\nprint()\nprint(x)\n", "\n2.0\n"),
            ("x = 2.0\nprint(+x, -x, (x))\n", "2.0 -2.0 2.0\n"),
            // Soft keywords are ordinary identifiers here.
            (
                "match = 1.0\ntype = 2.0\n_ = 3.0\nprint(match, type, _)\n",
                "1.0 2.0 3.0\n",
            ),
            // Binding a builtin name shadows it as a module global.
            ("float = 2.0\nprint(float)\n", "2.0\n"),
            ("len = 1.0\nprint(len + 1.0)\n", "2.0\n"),
            // User names cannot collide with temporaries or builtin names.
            ("py0 = 2.0\nprint(py0 + 1.0)\n", "3.0\n"),
            (
                "py1 = 2.0\npy0 = 7.0\nprint(py1, py0, 1.0)\n",
                "2.0 7.0 1.0\n",
            ),
            ("py_print_float = 1.0\nprint(py_print_float)\n", "1.0\n"),
            ("py_name_error = 4.0\nprint(py_name_error)\n", "4.0\n"),
        ] {
            assert_eq!(run_source(source).unwrap(), expected, "{source:?}");
        }
    }

    #[test]
    fn unbound_reads_raise_name_error_after_prior_output() {
        for (source, output, message) in [
            (
                "print(1.0)\nprint(y)\n",
                "1.0\n",
                "NameError: name 'y' is not defined",
            ),
            (
                "print(1.0)\nprint(z)\nz = 2.0\nprint(z)\n",
                "1.0\n",
                "NameError: name 'z' is not defined",
            ),
            ("x = x + 1.0\n", "", "NameError: name 'x' is not defined"),
            (
                "print(1.0)\ny\nprint(2.0)\n",
                "1.0\n",
                "NameError: name 'y' is not defined",
            ),
            (
                "print(y)\nprint(1.0)\n",
                "",
                "NameError: name 'y' is not defined",
            ),
            (
                "print(1.0)\nprint(w, 2.0, 3.0)\n",
                "1.0\n",
                "NameError: name 'w' is not defined",
            ),
            (
                "print(1.0)\nprint(2.0, w, 3.0)\n",
                "1.0\n",
                "NameError: name 'w' is not defined",
            ),
            (
                "print(1.0)\nprint(2.0, 3.0, w)\n",
                "1.0\n",
                "NameError: name 'w' is not defined",
            ),
            (
                "a = 1.0\nprint(a)\nb = a + c\n",
                "1.0\n",
                "NameError: name 'c' is not defined",
            ),
            // Left-to-right evaluation decides which error wins.
            ("y + 1.0 / 0.0\n", "", "NameError: name 'y' is not defined"),
            (
                "print(1.0 / 0.0, y)\n",
                "",
                "ZeroDivisionError: float division by zero",
            ),
            (
                "print(1.0)\nx = 1.0 / 0.0\nprint(x)\n",
                "1.0\n",
                "ZeroDivisionError: float division by zero",
            ),
        ] {
            let error = run_source(source).unwrap_err();
            assert_eq!(error.output, output, "{source:?}");
            assert_eq!(error.message, message, "{source:?}");
        }
    }

    #[test]
    fn unsupported_binding_forms_are_rejected_with_specific_errors() {
        for (source, expected) in [
            ("print = 1.0\n", "binding print"),
            ("a = b = 1.0\n", "chained assignment"),
            ("a = b = c = 1.0\n", "chained assignment"),
            ("x, y = 1.0, 2.0\n", "one name target"),
            ("x, = 1.0,\n", "one name target"),
            ("[x] = 1.0\n", "one name target"),
            ("(x) = 1.0\n", "one name target"),
            ("x = 1.0,\n", "not a tuple"),
            ("x = 1.0, 2.0\n", "not a tuple"),
            ("x: float = 1.0\n", "annotated assignment"),
            ("x: float\n", "annotated assignment"),
            ("x += 1.0\n", "augmented assignment"),
            ("x -= 1.0\n", "augmented assignment"),
            ("x /= 1.0\n", "augmented assignment"),
            ("a.b = 1.0\n", "attribute, subscript"),
            ("a[0.0] = 1.0\n", "attribute, subscript"),
            ("True = 1.0\n", "must be a name"),
            ("__debug__ = 1.0\n", "__debug__"),
            ("x = 1\n", "arbitrary-precision"),
            ("x = 1.0 + 1\n", "arbitrary-precision"),
            ("x = print()\n", "None result of print()"),
            ("x = print(1.0)\n", "None result of print()"),
            ("x = print\n", "direct call"),
            ("print(print)\n", "direct call"),
            ("print\n", "direct call"),
            ("print(float)\n", "not available as a value"),
            ("x = len\n", "not available as a value"),
            ("print(__name__)\n", "not available as a value"),
            ("print(__debug__)\n", "not available as a value"),
            ("print(float)\nfloat = 1.0\n", "not available as a value"),
        ] {
            let error = compile_source(source, "negative").unwrap_err();
            assert!(error.contains(expected), "{source:?}: {error}");
        }
        for source in [
            "x = 1.0; print(x)\n",
            "x = other()\n",
            "x = 1.0\nx()\n",
            "x = 1.0\nprint(x.real)\n",
            "x = (y := 1.0)\n",
            "x = 1.0\nprint(x, end=x)\n",
            "x = 'a'\n",
            "x = None\n",
        ] {
            assert!(compile_source(source, "negative").is_err(), "{source:?}");
        }
    }

    #[test]
    fn bindings_lower_to_const_and_mov_into_mangled_registers() {
        let module = compile_source("x = 1.5\nprint(x)\nprint(y)\n", "shape").unwrap();
        let main = module.get_function("main").unwrap();
        let shape: Vec<(&str, Option<&str>, &[Operand])> = main
            .instructions
            .iter()
            .map(|instr| {
                (
                    instr.op.as_str(),
                    instr.dest.as_deref(),
                    instr.srcs.as_slice(),
                )
            })
            .collect();
        assert_eq!(
            shape,
            vec![
                ("const", Some("py0"), &[Operand::Float(1.5)][..]),
                ("mov", Some("py:x"), &[Operand::Var("py0".into())][..]),
                (
                    "call_builtin",
                    None,
                    &[
                        Operand::Var("py_print_float".into()),
                        Operand::Var("py:x".into())
                    ][..]
                ),
                (
                    "call_builtin",
                    Some("py1"),
                    &[
                        Operand::Var("py_name_error".into()),
                        Operand::Str("y".into())
                    ][..]
                ),
                (
                    "call_builtin",
                    None,
                    &[
                        Operand::Var("py_print_float".into()),
                        Operand::Var("py1".into())
                    ][..]
                ),
                ("ret_void", None, &[][..]),
            ]
        );
        // Rebinding reads the old register before the closing mov replaces it.
        let module = compile_source("x = 1.0\nx = x + 1.0\n", "shape").unwrap();
        let ops: Vec<(&str, Option<&str>)> = module
            .get_function("main")
            .unwrap()
            .instructions
            .iter()
            .map(|instr| (instr.op.as_str(), instr.dest.as_deref()))
            .collect();
        assert_eq!(
            ops,
            vec![
                ("const", Some("py0")),
                ("mov", Some("py:x")),
                ("const", Some("py1")),
                ("add", Some("py2")),
                ("mov", Some("py:x")),
                ("ret_void", None),
            ]
        );
    }

    // Find the `index`-th NAME token spelled `x` (FLOAT/INT literals also use
    // the Name enum kind, but carry a named override, and never spell `x`).
    fn name_token_mut<'a>(
        node: &'a mut GrammarASTNode,
        index: &mut usize,
    ) -> Option<&'a mut ASTNodeOrToken> {
        for child in &mut node.children {
            match child {
                ASTNodeOrToken::Token(token) if token.value == "x" => {
                    if *index == 0 {
                        return Some(child);
                    }
                    *index -= 1;
                }
                ASTNodeOrToken::Node(inner) => {
                    if let Some(found) = name_token_mut(inner, index) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }

    #[test]
    fn direct_ast_name_tokens_require_canonical_kind_and_spelling() {
        let sources = [
            ("x = 1.0\nx\n", 2),
            ("x = 1.0\ny = x\n", 2),
            ("x = 1.0\nprint(x)\n", 2),
            ("x = 1.0\nprint(x, 2.0, 3.0)\n", 2),
            ("x = 1.0\nprint(2.0, x, 3.0)\n", 2),
            ("x = 1.0\nprint(2.0, 3.0, x)\n", 2),
            ("x = 1.0\nprint(-x + (x))\n", 3),
        ];
        let invalid_spellings = [
            "", "1x", "x-y", "x y", "x\0", " x", "x\n", "é", "ｘ", "if", "None", "True", "lambda",
            "print",
        ];
        for (source, occurrences) in sources {
            let original = parse_python(source, "3.12").unwrap();
            assert!(compile_ast(&original, "valid").is_ok(), "{source:?}");
            for index in 0..occurrences {
                for mode in 0..5 {
                    let mut forged = original.clone();
                    let slot = name_token_mut(&mut forged, &mut { index }).unwrap();
                    match (mode, &mut *slot) {
                        (0, ASTNodeOrToken::Token(token)) => token.type_ = TokenType::String,
                        (1, ASTNodeOrToken::Token(token)) => token.type_ = TokenType::Keyword,
                        (2, ASTNodeOrToken::Token(token)) => token.type_name = Some("NAME".into()),
                        (3, ASTNodeOrToken::Token(token)) => {
                            token.type_name = Some("STRING".into())
                        }
                        (4, _) => *slot = ASTNodeOrToken::Node(empty_node("forged_name")),
                        _ => unreachable!(),
                    }
                    assert!(
                        compile_ast(&forged, "forged").is_err(),
                        "{source:?} index={index} mode={mode}"
                    );
                }
                for spelling in invalid_spellings {
                    let mut forged = original.clone();
                    let Some(ASTNodeOrToken::Token(token)) =
                        name_token_mut(&mut forged, &mut { index })
                    else {
                        unreachable!()
                    };
                    token.value = spelling.into();
                    assert!(
                        compile_ast(&forged, "forged").is_err(),
                        "{source:?} index={index} {spelling:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn direct_ast_assignment_requires_an_actual_equals_token() {
        fn equals_slot(node: &mut GrammarASTNode) -> Option<&mut ASTNodeOrToken> {
            if node.rule_name == "assign_stmt" && node.children.len() == 3 {
                return node.children.get_mut(1);
            }
            node.children.iter_mut().find_map(|child| match child {
                ASTNodeOrToken::Node(inner) => equals_slot(inner),
                ASTNodeOrToken::Token(_) => None,
            })
        }
        for source in ["x = 1.0\n", "x = 1.0\nprint(x)\n"] {
            let original = parse_python(source, "3.12").unwrap();
            assert!(compile_ast(&original, "valid").is_ok());
            for mode in 0..8 {
                let mut forged = original.clone();
                let slot = equals_slot(&mut forged).unwrap();
                let ASTNodeOrToken::Token(token) = &mut *slot else {
                    unreachable!()
                };
                match mode {
                    0 => token.type_ = TokenType::String,
                    1 => token.type_ = TokenType::EqualsEquals,
                    2 => token.type_ = TokenType::Colon,
                    3 => token.type_name = Some("EQUALS".into()),
                    4 => token.type_name = Some("STRING".into()),
                    5 => token.value = "==".into(),
                    6 => token.value = ":=".into(),
                    7 => *slot = ASTNodeOrToken::Node(empty_node("forged_equals")),
                    _ => unreachable!(),
                }
                assert!(
                    compile_ast(&forged, "forged").is_err(),
                    "{source:?} mode={mode}"
                );
            }
        }
    }

    #[test]
    fn binding_names_stay_inside_existing_text_budgets() {
        fn rename_all(node: &mut GrammarASTNode, name: &str) {
            for child in &mut node.children {
                match child {
                    ASTNodeOrToken::Token(token) if token.value == "x" => {
                        token.value = name.into();
                    }
                    ASTNodeOrToken::Node(inner) => rename_all(inner, name),
                    _ => {}
                }
            }
        }
        let source = "x = 1.0\nprint(x)\n";
        let mut boundary = parse_python(source, "3.12").unwrap();
        rename_all(&mut boundary, &"a".repeat(MAX_AST_FIELD_BYTES));
        assert!(compile_ast(&boundary, "boundary").is_ok());
        let mut too_long = parse_python(source, "3.12").unwrap();
        rename_all(&mut too_long, &"a".repeat(MAX_AST_FIELD_BYTES + 1));
        assert!(compile_ast(&too_long, "boundary")
            .unwrap_err()
            .contains("text limit"));
        // Many bindings still hit the source cap before parsing.
        let many = "x = 1.0\n".repeat(MAX_SOURCE_BYTES / 8 + 1);
        assert!(compile_source(&many, "many")
            .unwrap_err()
            .contains("source exceeds"));
    }

    #[test]
    fn name_tables_are_sorted_and_consistent() {
        assert!(PYTHON_312_BUILTINS.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(PYTHON_KEYWORDS
            .iter()
            .all(|keyword| !python_name_spelling(keyword)));
        for name in ["x", "_", "_x1", "match", "case", "type", "Z9"] {
            assert!(python_name_spelling(name), "{name}");
        }
        assert!(is_dunder("__name__") && !is_dunder("____") && !is_dunder("__x"));
    }
}
