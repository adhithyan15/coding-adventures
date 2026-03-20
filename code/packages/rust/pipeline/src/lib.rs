//! # Pipeline — orchestrating the Rust computing stack end to end.
//!
//! This crate mirrors the educational pipeline packages in the other
//! languages. It wires the Rust `lexer` and `parser` crates into a complete
//! execution flow, then fills the current portability gap by carrying a local
//! bytecode compiler and a small stack-based virtual machine.
//!
//! The currently supported execution path is:
//!
//! ```text
//! Source -> Lexer -> Parser -> Compiler -> VM
//! ```
//!
//! In addition to running code, the crate records the output of each stage and
//! can export that trace in the JSON contract consumed by the `html-renderer`
//! package.

use lexer::token::{Token, TokenType};
use lexer::tokenizer::{Lexer, LexerConfig};
use parser::{ASTNode, ParseError, Parser as LangParser};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const PIPELINE_VERSION: &str = "0.1.0";

#[derive(Debug)]
pub enum PipelineError {
    Lex(String),
    Parse(ParseError),
    Compile(String),
    Runtime(String),
    Json(serde_json::Error),
    Io(std::io::Error),
}

impl Display for PipelineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lex(message) => write!(f, "lexer error: {message}"),
            Self::Parse(error) => write!(f, "parser error: {error}"),
            Self::Compile(message) => write!(f, "compiler error: {message}"),
            Self::Runtime(message) => write!(f, "runtime error: {message}"),
            Self::Json(error) => write!(f, "json error: {error}"),
            Self::Io(error) => write!(f, "i/o error: {error}"),
        }
    }
}

impl std::error::Error for PipelineError {}

impl From<ParseError> for PipelineError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<serde_json::Error> for PipelineError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<std::io::Error> for PipelineError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LexerStage {
    pub tokens: Vec<Token>,
    pub token_count: usize,
    pub source: String,
    pub duration_ms: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParserStage {
    pub ast: ASTNode,
    pub ast_dict: Value,
    pub duration_ms: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompilerStage {
    pub code: CodeObject,
    pub instructions_text: Vec<String>,
    pub constants: Vec<RuntimeValue>,
    pub names: Vec<String>,
    pub duration_ms: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VMStage {
    pub traces: Vec<VMTrace>,
    pub final_variables: BTreeMap<String, RuntimeValue>,
    pub output: Vec<String>,
    pub duration_ms: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PipelineResult {
    pub source: String,
    pub lexer_stage: LexerStage,
    pub parser_stage: ParserStage,
    pub compiler_stage: CompilerStage,
    pub vm_stage: VMStage,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum RuntimeValue {
    Number(f64),
    String(String),
}

impl RuntimeValue {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            Self::String(_) => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::Number(_) => None,
            Self::String(value) => Some(value.as_str()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum OpCode {
    LoadConst = 0x01,
    Pop = 0x02,
    Dup = 0x03,
    StoreName = 0x10,
    LoadName = 0x11,
    Add = 0x20,
    Sub = 0x21,
    Mul = 0x22,
    Div = 0x23,
    Print = 0x60,
    Halt = 0xff,
}

impl OpCode {
    pub fn name(self) -> &'static str {
        match self {
            Self::LoadConst => "LOAD_CONST",
            Self::Pop => "POP",
            Self::Dup => "DUP",
            Self::StoreName => "STORE_NAME",
            Self::LoadName => "LOAD_NAME",
            Self::Add => "ADD",
            Self::Sub => "SUB",
            Self::Mul => "MUL",
            Self::Div => "DIV",
            Self::Print => "PRINT",
            Self::Halt => "HALT",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Instruction {
    pub opcode: OpCode,
    pub operand: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CodeObject {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<RuntimeValue>,
    pub names: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VMTrace {
    pub pc: usize,
    pub instruction: Instruction,
    pub stack_before: Vec<RuntimeValue>,
    pub stack_after: Vec<RuntimeValue>,
    pub variables: BTreeMap<String, RuntimeValue>,
    pub output: Option<String>,
    pub description: String,
}

#[derive(Clone, Debug, Default)]
pub struct Pipeline;

impl Pipeline {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self, source: &str) -> Result<PipelineResult, PipelineError> {
        self.run_with_keywords(source, &[])
    }

    pub fn run_with_keywords(
        &self,
        source: &str,
        keywords: &[&str],
    ) -> Result<PipelineResult, PipelineError> {
        let lexer_start = Instant::now();
        let lexer_config = if keywords.is_empty() {
            None
        } else {
            Some(LexerConfig {
                keywords: keywords.iter().map(|keyword| (*keyword).to_string()).collect(),
            })
        };
        let mut lexer = Lexer::new(source, lexer_config);
        let tokens = lexer
            .tokenize()
            .map_err(|error| PipelineError::Lex(error.to_string()))?;
        let lexer_stage = LexerStage {
            token_count: tokens.len(),
            tokens: tokens.clone(),
            source: source.to_string(),
            duration_ms: elapsed_ms(lexer_start),
        };

        let parser_start = Instant::now();
        let mut parser = LangParser::new(tokens);
        let ast = parser.parse()?;
        let parser_stage = ParserStage {
            ast_dict: ast_to_dict(&ast),
            ast: ast.clone(),
            duration_ms: elapsed_ms(parser_start),
        };

        let compiler_start = Instant::now();
        let mut compiler = BytecodeCompiler::default();
        let code = compiler.compile(&ast)?;
        let instructions_text = code
            .instructions
            .iter()
            .map(|instruction| instruction_to_text(instruction, &code))
            .collect::<Vec<_>>();
        let compiler_stage = CompilerStage {
            constants: code.constants.clone(),
            names: code.names.clone(),
            code: code.clone(),
            instructions_text,
            duration_ms: elapsed_ms(compiler_start),
        };

        let vm_start = Instant::now();
        let mut vm = VirtualMachine::default();
        let traces = vm.execute(&code)?;
        let vm_stage = VMStage {
            final_variables: vm.variables.clone(),
            output: vm.output.clone(),
            traces,
            duration_ms: elapsed_ms(vm_start),
        };

        Ok(PipelineResult {
            source: source.to_string(),
            lexer_stage,
            parser_stage,
            compiler_stage,
            vm_stage,
        })
    }

    pub fn available_targets(&self) -> Vec<&'static str> {
        vec!["vm"]
    }

    pub fn run_to_json(&self, source: &str) -> Result<String, PipelineError> {
        let result = self.run(source)?;
        serde_json::to_string_pretty(&result.to_report()).map_err(PipelineError::from)
    }

    pub fn run_to_json_file<P: AsRef<Path>>(
        &self,
        source: &str,
        output_path: P,
    ) -> Result<(), PipelineError> {
        let json = self.run_to_json(source)?;
        fs::write(output_path, json)?;
        Ok(())
    }
}

impl PipelineResult {
    pub fn to_report(&self) -> Value {
        json!({
            "source": self.source,
            "language": "rust",
            "target": "vm",
            "metadata": {
                "generated_at": generated_at_string(),
                "generator_version": PIPELINE_VERSION,
                "packages": {
                    "lexer": "0.1.0",
                    "parser": "0.1.0",
                    "pipeline": PIPELINE_VERSION
                }
            },
            "stages": [
                {
                    "name": "lexer",
                    "display_name": "Tokenization",
                    "input_repr": self.source,
                    "output_repr": format!("{} tokens", self.lexer_stage.token_count),
                    "duration_ms": self.lexer_stage.duration_ms,
                    "data": {
                        "tokens": self.lexer_stage.tokens.iter().map(token_to_json).collect::<Vec<_>>()
                    }
                },
                {
                    "name": "parser",
                    "display_name": "Parsing",
                    "input_repr": format!("{} tokens", self.lexer_stage.token_count),
                    "output_repr": ast_summary(&self.parser_stage.ast),
                    "duration_ms": self.parser_stage.duration_ms,
                    "data": {
                        "ast": self.parser_stage.ast_dict
                    }
                },
                {
                    "name": "compiler",
                    "display_name": "Bytecode Compilation",
                    "input_repr": ast_summary(&self.parser_stage.ast),
                    "output_repr": format!(
                        "{} instructions",
                        self.compiler_stage.code.instructions.len()
                    ),
                    "duration_ms": self.compiler_stage.duration_ms,
                    "data": {
                        "instructions": self.compiler_stage.code.instructions.iter().enumerate().map(|(index, instruction)| {
                            json!({
                                "index": index,
                                "opcode": instruction.opcode.name(),
                                "arg": instruction.operand,
                                "stack_effect": stack_effect(instruction.opcode),
                            })
                        }).collect::<Vec<_>>()
                    }
                },
                {
                    "name": "vm",
                    "display_name": "VM Execution",
                    "input_repr": format!(
                        "{} instructions",
                        self.compiler_stage.code.instructions.len()
                    ),
                    "output_repr": format!("{} steps", self.vm_stage.traces.len()),
                    "duration_ms": self.vm_stage.duration_ms,
                    "data": {
                        "steps": self.vm_stage.traces.iter().map(|trace| {
                            json!({
                                "index": trace.pc,
                                "instruction": instruction_to_text(&trace.instruction, &self.compiler_stage.code),
                                "stack_before": trace.stack_before,
                                "stack_after": trace.stack_after,
                                "variables": trace.variables,
                            })
                        }).collect::<Vec<_>>(),
                        "output": self.vm_stage.output
                    }
                }
            ]
        })
    }
}

pub fn ast_to_dict(node: &ASTNode) -> Value {
    match node {
        ASTNode::Program(statements) => json!({
            "type": "Program",
            "statements": statements.iter().map(ast_to_dict).collect::<Vec<_>>()
        }),
        ASTNode::Assignment { target, value } => json!({
            "type": "Assignment",
            "target": {
                "type": "Name",
                "name": target
            },
            "value": ast_to_dict(value)
        }),
        ASTNode::BinaryOp { left, op, right } => json!({
            "type": "BinaryOp",
            "op": op,
            "left": ast_to_dict(left),
            "right": ast_to_dict(right)
        }),
        ASTNode::Number(value) => json!({
            "type": "NumberLiteral",
            "value": value
        }),
        ASTNode::String(value) => json!({
            "type": "StringLiteral",
            "value": value
        }),
        ASTNode::Name(name) => json!({
            "type": "Name",
            "name": name
        }),
        ASTNode::ExpressionStmt(expr) => json!({
            "type": "ExpressionStmt",
            "expression": ast_to_dict(expr)
        }),
    }
}

pub fn instruction_to_text(instruction: &Instruction, code: &CodeObject) -> String {
    match (instruction.opcode, instruction.operand) {
        (OpCode::LoadConst, Some(index)) => {
            if let Some(value) = code.constants.get(index) {
                format!(
                    "{} {} ({})",
                    instruction.opcode.name(),
                    index,
                    runtime_value_display(value)
                )
            } else {
                format!("{} {}", instruction.opcode.name(), index)
            }
        }
        (OpCode::StoreName | OpCode::LoadName, Some(index)) => {
            if let Some(name) = code.names.get(index) {
                format!("{} {} ('{}')", instruction.opcode.name(), index, name)
            } else {
                format!("{} {}", instruction.opcode.name(), index)
            }
        }
        (_, Some(index)) => format!("{} {}", instruction.opcode.name(), index),
        (_, None) => instruction.opcode.name().to_string(),
    }
}

#[derive(Default)]
struct BytecodeCompiler {
    instructions: Vec<Instruction>,
    constants: Vec<RuntimeValue>,
    names: Vec<String>,
}

impl BytecodeCompiler {
    fn compile(&mut self, program: &ASTNode) -> Result<CodeObject, PipelineError> {
        match program {
            ASTNode::Program(statements) => {
                for statement in statements {
                    self.compile_statement(statement)?;
                }
                self.instructions.push(Instruction {
                    opcode: OpCode::Halt,
                    operand: None,
                });
                Ok(CodeObject {
                    instructions: self.instructions.clone(),
                    constants: self.constants.clone(),
                    names: self.names.clone(),
                })
            }
            _ => Err(PipelineError::Compile(
                "pipeline compiler expected a Program root".to_string(),
            )),
        }
    }

    fn compile_statement(&mut self, statement: &ASTNode) -> Result<(), PipelineError> {
        match statement {
            ASTNode::Assignment { target, value } => {
                self.compile_expression(value)?;
                let name_index = self.add_name(target);
                self.instructions.push(Instruction {
                    opcode: OpCode::StoreName,
                    operand: Some(name_index),
                });
                Ok(())
            }
            ASTNode::ExpressionStmt(expr) => {
                self.compile_expression(expr)?;
                self.instructions.push(Instruction {
                    opcode: OpCode::Pop,
                    operand: None,
                });
                Ok(())
            }
            _ => Err(PipelineError::Compile(format!(
                "unsupported top-level statement: {statement}"
            ))),
        }
    }

    fn compile_expression(&mut self, node: &ASTNode) -> Result<(), PipelineError> {
        match node {
            ASTNode::Number(value) => {
                let index = self.add_constant(RuntimeValue::Number(*value));
                self.instructions.push(Instruction {
                    opcode: OpCode::LoadConst,
                    operand: Some(index),
                });
                Ok(())
            }
            ASTNode::String(value) => {
                let index = self.add_constant(RuntimeValue::String(value.clone()));
                self.instructions.push(Instruction {
                    opcode: OpCode::LoadConst,
                    operand: Some(index),
                });
                Ok(())
            }
            ASTNode::Name(name) => {
                let index = self.add_name(name);
                self.instructions.push(Instruction {
                    opcode: OpCode::LoadName,
                    operand: Some(index),
                });
                Ok(())
            }
            ASTNode::BinaryOp { left, op, right } => {
                self.compile_expression(left)?;
                self.compile_expression(right)?;
                let opcode = match op.as_str() {
                    "+" => OpCode::Add,
                    "-" => OpCode::Sub,
                    "*" => OpCode::Mul,
                    "/" => OpCode::Div,
                    _ => {
                        return Err(PipelineError::Compile(format!(
                            "unsupported binary operator: {op}"
                        )))
                    }
                };
                self.instructions.push(Instruction {
                    opcode,
                    operand: None,
                });
                Ok(())
            }
            ASTNode::ExpressionStmt(expr) => self.compile_expression(expr),
            ASTNode::Assignment { .. } | ASTNode::Program(_) => Err(PipelineError::Compile(
                "assignment/program nodes cannot appear inside expressions".to_string(),
            )),
        }
    }

    fn add_constant(&mut self, value: RuntimeValue) -> usize {
        if let Some(index) = self.constants.iter().position(|existing| existing == &value) {
            return index;
        }
        self.constants.push(value);
        self.constants.len() - 1
    }

    fn add_name(&mut self, name: &str) -> usize {
        if let Some(index) = self.names.iter().position(|existing| existing == name) {
            return index;
        }
        self.names.push(name.to_string());
        self.names.len() - 1
    }
}

#[derive(Debug, Default)]
struct VirtualMachine {
    stack: Vec<RuntimeValue>,
    variables: BTreeMap<String, RuntimeValue>,
    output: Vec<String>,
    pc: usize,
}

impl VirtualMachine {
    fn execute(&mut self, code: &CodeObject) -> Result<Vec<VMTrace>, PipelineError> {
        let mut traces = Vec::new();
        self.stack.clear();
        self.variables.clear();
        self.output.clear();
        self.pc = 0;

        while self.pc < code.instructions.len() {
            let instruction = code.instructions[self.pc].clone();
            let stack_before = self.stack.clone();
            let output_before = self.output.len();
            let current_pc = self.pc;
            let description = self.execute_instruction(&instruction, code)?;
            let output = if self.output.len() > output_before {
                self.output.last().cloned()
            } else {
                None
            };
            let trace = VMTrace {
                pc: current_pc,
                instruction: instruction.clone(),
                stack_before,
                stack_after: self.stack.clone(),
                variables: self.variables.clone(),
                output,
                description,
            };
            traces.push(trace);
            if instruction.opcode == OpCode::Halt {
                break;
            }
        }

        Ok(traces)
    }

    fn execute_instruction(
        &mut self,
        instruction: &Instruction,
        code: &CodeObject,
    ) -> Result<String, PipelineError> {
        match instruction.opcode {
            OpCode::LoadConst => {
                let index = required_operand(instruction)?;
                let value = code
                    .constants
                    .get(index)
                    .cloned()
                    .ok_or_else(|| {
                        PipelineError::Runtime(format!("constant index out of bounds: {index}"))
                    })?;
                self.stack.push(value.clone());
                self.pc += 1;
                Ok(format!("Push {} onto the stack", runtime_value_display(&value)))
            }
            OpCode::Pop => {
                let value = self.pop_stack()?;
                self.pc += 1;
                Ok(format!("Pop {} from the stack", runtime_value_display(&value)))
            }
            OpCode::Dup => {
                let value = self
                    .stack
                    .last()
                    .cloned()
                    .ok_or_else(|| PipelineError::Runtime("cannot DUP an empty stack".to_string()))?;
                self.stack.push(value.clone());
                self.pc += 1;
                Ok(format!("Duplicate {}", runtime_value_display(&value)))
            }
            OpCode::StoreName => {
                let index = required_operand(instruction)?;
                let name = code.names.get(index).cloned().ok_or_else(|| {
                    PipelineError::Runtime(format!("name index out of bounds: {index}"))
                })?;
                let value = self.pop_stack()?;
                self.variables.insert(name.clone(), value.clone());
                self.pc += 1;
                Ok(format!(
                    "Store {} into variable '{}'",
                    runtime_value_display(&value),
                    name
                ))
            }
            OpCode::LoadName => {
                let index = required_operand(instruction)?;
                let name = code.names.get(index).cloned().ok_or_else(|| {
                    PipelineError::Runtime(format!("name index out of bounds: {index}"))
                })?;
                let value = self.variables.get(&name).cloned().ok_or_else(|| {
                    PipelineError::Runtime(format!("undefined variable: {name}"))
                })?;
                self.stack.push(value.clone());
                self.pc += 1;
                Ok(format!(
                    "Load variable '{}' with value {}",
                    name,
                    runtime_value_display(&value)
                ))
            }
            OpCode::Add => {
                let (left, right) = self.pop_two_numbers()?;
                let result = left + right;
                self.stack.push(RuntimeValue::Number(result));
                self.pc += 1;
                Ok(format!("Add {left} and {right} -> {result}"))
            }
            OpCode::Sub => {
                let (left, right) = self.pop_two_numbers()?;
                let result = left - right;
                self.stack.push(RuntimeValue::Number(result));
                self.pc += 1;
                Ok(format!("Subtract {right} from {left} -> {result}"))
            }
            OpCode::Mul => {
                let (left, right) = self.pop_two_numbers()?;
                let result = left * right;
                self.stack.push(RuntimeValue::Number(result));
                self.pc += 1;
                Ok(format!("Multiply {left} by {right} -> {result}"))
            }
            OpCode::Div => {
                let (left, right) = self.pop_two_numbers()?;
                if right == 0.0 {
                    return Err(PipelineError::Runtime("division by zero".to_string()));
                }
                let result = left / right;
                self.stack.push(RuntimeValue::Number(result));
                self.pc += 1;
                Ok(format!("Divide {left} by {right} -> {result}"))
            }
            OpCode::Print => {
                let value = self.pop_stack()?;
                let rendered = runtime_value_display(&value);
                self.output.push(rendered.clone());
                self.pc += 1;
                Ok(format!("Print {rendered}"))
            }
            OpCode::Halt => Ok("Halt execution".to_string()),
        }
    }

    fn pop_stack(&mut self) -> Result<RuntimeValue, PipelineError> {
        self.stack
            .pop()
            .ok_or_else(|| PipelineError::Runtime("stack underflow".to_string()))
    }

    fn pop_two_numbers(&mut self) -> Result<(f64, f64), PipelineError> {
        let right = self.pop_stack()?;
        let left = self.pop_stack()?;
        let right_number = right.as_number().ok_or_else(|| {
            PipelineError::Runtime(format!(
                "expected numeric operand on stack, got {}",
                runtime_value_display(&right)
            ))
        })?;
        let left_number = left.as_number().ok_or_else(|| {
            PipelineError::Runtime(format!(
                "expected numeric operand on stack, got {}",
                runtime_value_display(&left)
            ))
        })?;
        Ok((left_number, right_number))
    }
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn generated_at_string() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}", duration.as_secs()),
        Err(_) => "0".to_string(),
    }
}

fn required_operand(instruction: &Instruction) -> Result<usize, PipelineError> {
    instruction.operand.ok_or_else(|| {
        PipelineError::Runtime(format!(
            "{} requires an operand",
            instruction.opcode.name()
        ))
    })
}

fn token_to_json(token: &Token) -> Value {
    json!({
        "type": token_type_name(token.type_),
        "value": token.value,
        "line": token.line,
        "column": token.column,
    })
}

fn token_type_name(token_type: TokenType) -> &'static str {
    match token_type {
        TokenType::Name => "NAME",
        TokenType::Number => "NUMBER",
        TokenType::String => "STRING",
        TokenType::Keyword => "KEYWORD",
        TokenType::Plus => "PLUS",
        TokenType::Minus => "MINUS",
        TokenType::Star => "STAR",
        TokenType::Slash => "SLASH",
        TokenType::Equals => "EQUALS",
        TokenType::EqualsEquals => "EQUALS_EQUALS",
        TokenType::LParen => "LPAREN",
        TokenType::RParen => "RPAREN",
        TokenType::Comma => "COMMA",
        TokenType::Colon => "COLON",
        TokenType::Semicolon => "SEMICOLON",
        TokenType::LBrace => "LBRACE",
        TokenType::RBrace => "RBRACE",
        TokenType::LBracket => "LBRACKET",
        TokenType::RBracket => "RBRACKET",
        TokenType::Dot => "DOT",
        TokenType::Bang => "BANG",
        TokenType::Newline => "NEWLINE",
        TokenType::Eof => "EOF",
    }
}

fn runtime_value_display(value: &RuntimeValue) -> String {
    match value {
        RuntimeValue::Number(number) => {
            if number.fract() == 0.0 {
                format!("{}", *number as i64)
            } else {
                number.to_string()
            }
        }
        RuntimeValue::String(string) => format!("{string:?}"),
    }
}

fn stack_effect(opcode: OpCode) -> &'static str {
    match opcode {
        OpCode::LoadConst | OpCode::LoadName | OpCode::Dup => "push",
        OpCode::StoreName | OpCode::Pop | OpCode::Print => "pop",
        OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div => "pop2→push1",
        OpCode::Halt => "stop",
    }
}

fn ast_summary(node: &ASTNode) -> String {
    match node {
        ASTNode::Program(statements) => format!("AST with {} statements", statements.len()),
        _ => "AST".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn simple_assignment_returns_pipeline_result() {
        let result = Pipeline::new().run("x = 1 + 2").expect("pipeline should run");
        assert_eq!(result.source, "x = 1 + 2");
        assert!(result.lexer_stage.token_count >= 6);
        assert_eq!(result.parser_stage.ast_dict["type"], "Program");
        assert_eq!(result.compiler_stage.constants.len(), 2);
        assert_eq!(
            result.vm_stage.final_variables.get("x"),
            Some(&RuntimeValue::Number(3.0))
        );
    }

    #[test]
    fn multiple_assignments_and_variable_reuse_work() {
        let result = Pipeline::new()
            .run("a = 10\nb = 20\nc = a + b")
            .expect("pipeline should run");

        assert_eq!(result.vm_stage.final_variables.get("a"), Some(&RuntimeValue::Number(10.0)));
        assert_eq!(result.vm_stage.final_variables.get("b"), Some(&RuntimeValue::Number(20.0)));
        assert_eq!(result.vm_stage.final_variables.get("c"), Some(&RuntimeValue::Number(30.0)));
    }

    #[test]
    fn parentheses_override_precedence() {
        let result = Pipeline::new()
            .run("x = (1 + 2) * 3")
            .expect("pipeline should run");
        assert_eq!(
            result.vm_stage.final_variables.get("x"),
            Some(&RuntimeValue::Number(9.0))
        );
    }

    #[test]
    fn strings_can_be_assigned() {
        let result = Pipeline::new()
            .run("x = \"hello\"")
            .expect("pipeline should run");
        assert_eq!(
            result.vm_stage.final_variables.get("x"),
            Some(&RuntimeValue::String("hello".to_string()))
        );
    }

    #[test]
    fn ast_to_dict_converts_assignment_shape() {
        let node = ASTNode::Assignment {
            target: "x".to_string(),
            value: Box::new(ASTNode::Number(42.0)),
        };
        assert_eq!(
            ast_to_dict(&node),
            json!({
                "type": "Assignment",
                "target": { "type": "Name", "name": "x" },
                "value": { "type": "NumberLiteral", "value": 42.0 }
            })
        );
    }

    #[test]
    fn instruction_to_text_resolves_constants_and_names() {
        let code = CodeObject {
            instructions: vec![],
            constants: vec![RuntimeValue::Number(42.0)],
            names: vec!["x".to_string()],
        };
        assert_eq!(
            instruction_to_text(
                &Instruction {
                    opcode: OpCode::LoadConst,
                    operand: Some(0),
                },
                &code,
            ),
            "LOAD_CONST 0 (42)"
        );
        assert_eq!(
            instruction_to_text(
                &Instruction {
                    opcode: OpCode::StoreName,
                    operand: Some(0),
                },
                &code,
            ),
            "STORE_NAME 0 ('x')"
        );
    }

    #[test]
    fn run_to_json_uses_html_renderer_contract_shape() {
        let json = Pipeline::new()
            .run_to_json("x = 1 + 2")
            .expect("json export should succeed");
        let value: Value = serde_json::from_str(&json).expect("json should parse");

        assert_eq!(value["language"], "rust");
        assert_eq!(value["target"], "vm");
        assert_eq!(value["stages"][0]["name"], "lexer");
        assert_eq!(value["stages"][1]["name"], "parser");
        assert_eq!(value["stages"][2]["name"], "compiler");
        assert_eq!(value["stages"][3]["name"], "vm");
    }

    #[test]
    fn run_to_json_file_writes_output() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let base = std::env::temp_dir().join(format!("pipeline-{unique}"));
        fs::create_dir_all(&base).expect("temp dir should be creatable");
        let output_path = base.join("report.json");

        Pipeline::new()
            .run_to_json_file("x = 1 + 2", &output_path)
            .expect("json file should be written");

        let written = fs::read_to_string(output_path).expect("json file should exist");
        assert!(written.contains("\"language\": \"rust\""));
        assert!(written.contains("\"name\": \"vm\""));
    }

    #[test]
    fn available_targets_is_vm_only_for_now() {
        assert_eq!(Pipeline::new().available_targets(), vec!["vm"]);
    }
}
