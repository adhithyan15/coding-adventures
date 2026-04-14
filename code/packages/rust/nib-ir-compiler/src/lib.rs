use std::collections::HashMap;

use compiler_ir::{IdGenerator, IrInstruction, IrOp, IrOperand, IrProgram};
use nib_type_checker::TypedAst;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

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
        registers: HashMap::new(),
        next_register: 2,
    };
    compiler.compile_main(&ast.root);
    CompileResult {
        program: compiler.program,
    }
}

struct Compiler {
    program: IrProgram,
    ids: IdGenerator,
    registers: HashMap<String, usize>,
    next_register: usize,
}

impl Compiler {
    fn compile_main(&mut self, root: &GrammarASTNode) {
        self.program.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("_start".to_string())],
            -1,
        ));

        if let Some(main_fn) = child_nodes(root)
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
            .find(|node| first_name(node).as_deref() == Some("main"))
        {
            if let Some(block) = child_nodes(main_fn)
                .into_iter()
                .find(|node| node.rule_name == "block")
            {
                self.compile_block(block);
            }
        }

        self.program
            .add_instruction(IrInstruction::new(IrOp::Halt, vec![], self.ids.next()));
    }

    fn compile_block(&mut self, block: &GrammarASTNode) {
        for stmt in child_nodes(block) {
            self.compile_stmt(stmt);
        }
    }

    fn compile_stmt(&mut self, stmt: &GrammarASTNode) {
        if stmt.rule_name == "stmt" {
            if let Some(inner) = child_nodes(stmt).first() {
                self.compile_stmt(inner);
            }
            return;
        }
        match stmt.rule_name.as_str() {
            "let_stmt" => {
                let name = first_name(stmt).unwrap_or_else(|| format!("tmp{}", self.next_register));
                let register = self.allocate_register(name.clone());
                if let Some(expr) = child_nodes(stmt)
                    .into_iter()
                    .find(|node| node.rule_name == "expr")
                {
                    self.emit_expr_into(expr, register);
                }
                self.registers.insert(name, register);
            }
            "assign_stmt" => {
                if let Some(name) = first_name(stmt) {
                    if let Some(&register) = self.registers.get(&name) {
                        if let Some(expr) = child_nodes(stmt)
                            .into_iter()
                            .find(|node| node.rule_name == "expr")
                        {
                            self.emit_expr_into(expr, register);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn emit_expr_into(&mut self, node: &GrammarASTNode, dest: usize) {
        if let Some(value) = parse_literal(node) {
            self.program.add_instruction(IrInstruction::new(
                IrOp::LoadImm,
                vec![IrOperand::Register(dest), IrOperand::Immediate(value)],
                self.ids.next(),
            ));
            return;
        }

        if let Some(name) = lookup_name(node) {
            if let Some(&source) = self.registers.get(&name) {
                if source != dest {
                    self.program.add_instruction(IrInstruction::new(
                        IrOp::AddImm,
                        vec![
                            IrOperand::Register(dest),
                            IrOperand::Register(source),
                            IrOperand::Immediate(0),
                        ],
                        self.ids.next(),
                    ));
                }
                return;
            }
        }

        if node.rule_name == "add_expr" {
            let operands = child_nodes(node);
            if operands.len() >= 2 {
                self.emit_expr_into(operands[0], dest);
                if let Some(rhs_imm) = parse_literal(operands[1]) {
                    self.program.add_instruction(IrInstruction::new(
                        IrOp::AddImm,
                        vec![
                            IrOperand::Register(dest),
                            IrOperand::Register(dest),
                            IrOperand::Immediate(rhs_imm),
                        ],
                        self.ids.next(),
                    ));
                } else {
                    let scratch = 15;
                    self.emit_expr_into(operands[1], scratch);
                    self.program.add_instruction(IrInstruction::new(
                        IrOp::Add,
                        vec![
                            IrOperand::Register(dest),
                            IrOperand::Register(dest),
                            IrOperand::Register(scratch),
                        ],
                        self.ids.next(),
                    ));
                }
                return;
            }
        }

        if let Some(child) = child_nodes(node).first() {
            self.emit_expr_into(child, dest);
        }
    }

    fn allocate_register(&mut self, name: String) -> usize {
        if let Some(&register) = self.registers.get(&name) {
            return register;
        }
        let register = self.next_register;
        self.next_register += 1;
        self.registers.insert(name, register);
        register
    }
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

fn first_name(node: &GrammarASTNode) -> Option<String> {
    node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Token(token) if token.effective_type_name() == "NAME" => {
            Some(token.value.clone())
        }
        ASTNodeOrToken::Node(inner) => first_name(inner),
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

fn parse_literal(node: &GrammarASTNode) -> Option<i64> {
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
}
