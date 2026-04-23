use std::collections::HashMap;

use compiler_ir::{IdGenerator, IrInstruction, IrOp, IrOperand, IrProgram};
use nib_type_checker::TypedAst;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

const REG_RETURN: usize = 1;
const REG_VAR_BASE: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildConfig {
    pub optimize: bool,
}

#[derive(Debug, Clone)]
pub struct CompileResult {
    pub program: IrProgram,
}

pub fn release_config() -> BuildConfig {
    BuildConfig { optimize: true }
}

pub fn compile_nib(ast: TypedAst, _config: BuildConfig) -> CompileResult {
    let mut compiler = Compiler {
        program: IrProgram::new("_start"),
        ids: IdGenerator::new(),
    };
    compiler.compile_program(&ast.root);
    CompileResult {
        program: compiler.program,
    }
}

struct Compiler {
    program: IrProgram,
    ids: IdGenerator,
}

impl Compiler {
    fn compile_program(&mut self, root: &GrammarASTNode) {
        self.emit_label("_start");
        if function_nodes(root)
            .iter()
            .any(|node| first_name(node).as_deref() == Some("main"))
        {
            self.emit(IrOp::Call, vec![IrOperand::Label("_fn_main".to_string())]);
        }
        self.emit(IrOp::Halt, vec![]);

        for function in function_nodes(root) {
            self.compile_function(function);
        }
    }

    fn compile_function(&mut self, node: &GrammarASTNode) {
        let Some(function_name) = first_name(node) else {
            return;
        };
        let Some(block) = child_nodes(node)
            .into_iter()
            .find(|child| child.rule_name == "block")
        else {
            return;
        };

        self.emit_label(&format!("_fn_{function_name}"));
        let mut locals = HashMap::new();
        let mut next_register = REG_VAR_BASE;
        for name in extract_params(node) {
            locals.insert(name, next_register);
            next_register += 1;
        }
        self.compile_block(block, &mut locals, next_register);
        self.emit(IrOp::Ret, vec![]);
    }

    fn compile_block(
        &mut self,
        block: &GrammarASTNode,
        locals: &mut HashMap<String, usize>,
        mut next_register: usize,
    ) -> usize {
        for stmt in child_nodes(block) {
            next_register = self.compile_stmt(stmt, locals, next_register);
        }
        next_register
    }

    fn compile_stmt(
        &mut self,
        stmt: &GrammarASTNode,
        locals: &mut HashMap<String, usize>,
        next_register: usize,
    ) -> usize {
        if stmt.rule_name == "stmt" {
            if let Some(inner) = child_nodes(stmt).first() {
                return self.compile_stmt(inner, locals, next_register);
            }
            return next_register;
        }

        match stmt.rule_name.as_str() {
            "let_stmt" => self.compile_let(stmt, locals, next_register),
            "assign_stmt" => {
                if let Some(name) = first_name(stmt) {
                    if let Some(&register) = locals.get(&name) {
                        if let Some(expr) = expression_children(stmt).first() {
                            self.emit_expr_into(expr, register, locals);
                        }
                    }
                }
                next_register
            }
            "return_stmt" => {
                if let Some(expr) = expression_children(stmt).first() {
                    self.emit_expr_into(expr, REG_RETURN, locals);
                }
                next_register
            }
            "expr_stmt" => {
                if let Some(expr) = expression_children(stmt).first() {
                    self.emit_expr_into(expr, REG_RETURN, locals);
                }
                next_register
            }
            _ => next_register,
        }
    }

    fn compile_let(
        &mut self,
        stmt: &GrammarASTNode,
        locals: &mut HashMap<String, usize>,
        next_register: usize,
    ) -> usize {
        let name = first_name(stmt).unwrap_or_else(|| format!("tmp{next_register}"));
        let register = *locals.entry(name).or_insert(next_register);
        if let Some(expr) = expression_children(stmt).first() {
            self.emit_expr_into(expr, register, locals);
        }
        next_register.max(register + 1)
    }

    fn emit_expr_into(
        &mut self,
        node: &GrammarASTNode,
        dest: usize,
        locals: &HashMap<String, usize>,
    ) {
        let children = child_nodes(node);
        if children.len() == 1 && node.rule_name != "add_expr" && node.rule_name != "call_expr" {
            self.emit_expr_into(children[0], dest, locals);
            return;
        }

        if node.rule_name == "call_expr" {
            self.emit_call_expr(node, dest, locals);
            return;
        }

        if node.rule_name == "add_expr" && children.len() >= 2 {
            self.emit_expr_into(children[0], dest, locals);
            let rhs_register = if let Some(value) = parse_literal(children[1]) {
                self.emit(
                    IrOp::AddImm,
                    vec![
                        IrOperand::Register(dest),
                        IrOperand::Register(dest),
                        IrOperand::Immediate(value),
                    ],
                );
                None
            } else {
                let scratch = REG_VAR_BASE + 32;
                self.emit_expr_into(children[1], scratch, locals);
                Some(scratch)
            };
            if let Some(rhs) = rhs_register {
                self.emit(
                    IrOp::Add,
                    vec![
                        IrOperand::Register(dest),
                        IrOperand::Register(dest),
                        IrOperand::Register(rhs),
                    ],
                );
            }
            self.emit(
                IrOp::AndImm,
                vec![
                    IrOperand::Register(dest),
                    IrOperand::Register(dest),
                    IrOperand::Immediate(15),
                ],
            );
            return;
        }

        if let Some(value) = parse_literal(node) {
            self.emit(
                IrOp::LoadImm,
                vec![IrOperand::Register(dest), IrOperand::Immediate(value)],
            );
            return;
        }

        if let Some(name) = lookup_name(node) {
            if let Some(&source) = locals.get(&name) {
                self.copy_register(dest, source);
                return;
            }
        }

        if let Some(child) = children.first() {
            self.emit_expr_into(child, dest, locals);
        }
    }

    fn emit_call_expr(
        &mut self,
        node: &GrammarASTNode,
        dest: usize,
        locals: &HashMap<String, usize>,
    ) {
        let Some(function_name) = first_direct_name(node) else {
            return;
        };
        for (index, arg) in call_arguments(node).iter().enumerate() {
            self.emit_expr_into(arg, REG_VAR_BASE + index, locals);
        }
        self.emit(
            IrOp::Call,
            vec![IrOperand::Label(format!("_fn_{function_name}"))],
        );
        if dest != REG_RETURN {
            self.copy_register(dest, REG_RETURN);
        }
    }

    fn copy_register(&mut self, dest: usize, source: usize) {
        if dest == source {
            return;
        }
        self.emit(
            IrOp::AddImm,
            vec![
                IrOperand::Register(dest),
                IrOperand::Register(source),
                IrOperand::Immediate(0),
            ],
        );
    }

    fn emit_label(&mut self, name: &str) {
        self.program.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label(name.to_string())],
            -1,
        ));
    }

    fn emit(&mut self, opcode: IrOp, operands: Vec<IrOperand>) {
        self.program
            .add_instruction(IrInstruction::new(opcode, operands, self.ids.next()));
    }
}

fn function_nodes(root: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    child_nodes(root)
        .into_iter()
        .filter_map(|node| {
            if node.rule_name == "fn_decl" {
                Some(node)
            } else if node.rule_name == "top_decl" {
                child_nodes(node)
                    .into_iter()
                    .find(|inner| inner.rule_name == "fn_decl")
            } else {
                None
            }
        })
        .collect()
}

fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|child| match child {
            ASTNodeOrToken::Node(inner) => Some(inner),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

fn expression_children(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    child_nodes(node)
        .into_iter()
        .filter(|child| {
            matches!(
                child.rule_name.as_str(),
                "expr"
                    | "or_expr"
                    | "and_expr"
                    | "eq_expr"
                    | "cmp_expr"
                    | "add_expr"
                    | "bitwise_expr"
                    | "unary_expr"
                    | "primary"
                    | "call_expr"
            )
        })
        .collect()
}

fn first_name(node: &GrammarASTNode) -> Option<String> {
    node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Token(token) if token.effective_type_name() == "NAME" => {
            Some(token.value.clone())
        }
        ASTNodeOrToken::Node(inner) => first_name(inner),
        _ => None,
    })
}

fn first_direct_name(node: &GrammarASTNode) -> Option<String> {
    node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Token(token) if token.effective_type_name() == "NAME" => {
            Some(token.value.clone())
        }
        _ => None,
    })
}

fn lookup_name(node: &GrammarASTNode) -> Option<String> {
    node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Token(token) if token.effective_type_name() == "NAME" => {
            Some(token.value.clone())
        }
        ASTNodeOrToken::Node(inner) => lookup_name(inner),
        _ => None,
    })
}

fn extract_params(fn_decl: &GrammarASTNode) -> Vec<String> {
    child_nodes(fn_decl)
        .into_iter()
        .find(|node| node.rule_name == "param_list")
        .map(|param_list| {
            child_nodes(param_list)
                .into_iter()
                .filter(|node| node.rule_name == "param")
                .filter_map(first_name)
                .collect()
        })
        .unwrap_or_default()
}

fn call_arguments(call_expr: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    child_nodes(call_expr)
        .into_iter()
        .find(|node| node.rule_name == "arg_list")
        .map(|arg_list| expression_children(arg_list))
        .unwrap_or_default()
}

fn parse_literal(node: &GrammarASTNode) -> Option<i64> {
    if node.rule_name == "call_expr" {
        return None;
    }
    node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Token(token) if token.effective_type_name() == "INT_LIT" => {
            token.value.parse().ok()
        }
        ASTNodeOrToken::Token(token) if token.effective_type_name() == "HEX_LIT" => {
            i64::from_str_radix(token.value.trim_start_matches("0x"), 16).ok()
        }
        ASTNodeOrToken::Node(inner) => parse_literal(inner),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nib_type_checker::check_source;

    #[test]
    fn compile_source_produces_ir() {
        let typed = check_source("fn main() { let x: u4 = 5; }");
        assert!(typed.ok, "type check failed: {:?}", typed.errors);
        let result = compile_nib(typed.typed_ast, release_config());
        assert!(!result.program.instructions.is_empty());
    }

    #[test]
    fn compiles_functions_and_returns() {
        let typed = check_source("fn answer() -> u4 { return 7; }");
        assert!(typed.ok, "type check failed: {:?}", typed.errors);
        let result = compile_nib(typed.typed_ast, release_config());
        assert!(result.program.instructions.iter().any(|instruction| {
            instruction.opcode == IrOp::Label
                && instruction.operands == vec![IrOperand::Label("_fn_answer".to_string())]
        }));
        assert!(result
            .program
            .instructions
            .iter()
            .any(|instruction| instruction.opcode == IrOp::Ret));
    }
}
