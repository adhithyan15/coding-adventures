//! MACSYMA runtime session facade.
//!
//! This crate is the first Rust runtime layer above the grammar-driven
//! MACSYMA compiler. It keeps the public API pure and WASM-friendly: callers
//! pass source strings in and receive evaluated IR nodes plus display metadata.

use coding_adventures_macsyma_compiler::{
    compile_macsyma_with_options, CompileError, CompileOptions, DISPLAY, SUPPRESS,
};
use symbolic_ir::{sym, IRNode};
use symbolic_vm::{SymbolicBackend, VM};

/// One evaluated MACSYMA statement.
#[derive(Debug, Clone, PartialEq)]
pub struct EvalResult {
    /// The 1-based input index, matching MACSYMA's `%iN` convention.
    pub input_index: usize,
    /// The 1-based output index, matching MACSYMA's `%oN` convention.
    pub output_index: usize,
    /// The compiled input expression with display/suppress wrapper removed.
    pub input: IRNode,
    /// The evaluated output expression.
    pub output: IRNode,
    /// Whether this statement should be shown by a REPL. `;` displays, `$`
    /// suppresses.
    pub display: bool,
}

/// In-memory `%i`/`%o` history for a single runtime session.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct History {
    inputs: Vec<IRNode>,
    outputs: Vec<IRNode>,
}

impl History {
    pub fn record_input(&mut self, node: IRNode) -> usize {
        self.inputs.push(node);
        self.inputs.len()
    }

    pub fn record_output(&mut self, node: IRNode) -> usize {
        self.outputs.push(node);
        self.outputs.len()
    }

    pub fn get_input(&self, index: usize) -> Option<&IRNode> {
        index.checked_sub(1).and_then(|idx| self.inputs.get(idx))
    }

    pub fn get_output(&self, index: usize) -> Option<&IRNode> {
        index.checked_sub(1).and_then(|idx| self.outputs.get(idx))
    }

    pub fn last_output(&self) -> Option<&IRNode> {
        self.outputs.last()
    }

    pub fn next_input_index(&self) -> usize {
        self.inputs.len() + 1
    }

    pub fn inputs(&self) -> &[IRNode] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[IRNode] {
        &self.outputs
    }

    pub fn reset(&mut self) {
        self.inputs.clear();
        self.outputs.clear();
    }
}

/// Stateful MACSYMA evaluator over the Rust symbolic VM.
pub struct MacsymaSession {
    vm: VM,
    history: History,
}

impl MacsymaSession {
    pub fn new() -> Self {
        let mut backend = SymbolicBackend::new();
        backend.pre_bind("%pi", IRNode::Float(std::f64::consts::PI));
        backend.pre_bind("%e", IRNode::Float(std::f64::consts::E));
        backend.pre_bind("%i", sym("ImaginaryUnit"));
        Self {
            vm: VM::new(Box::new(backend)),
            history: History::default(),
        }
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn history_mut(&mut self) -> &mut History {
        &mut self.history
    }

    /// Compile and evaluate every statement in `source`.
    pub fn eval_source(&mut self, source: &str) -> Result<Vec<EvalResult>, CompileError> {
        let statements = compile_macsyma_with_options(
            source,
            CompileOptions {
                wrap_terminators: true,
            },
        )?;
        Ok(self.eval_statements(statements))
    }

    /// Evaluate already-compiled statements. Display wrappers are honored when
    /// present; unwrapped statements default to display=true.
    pub fn eval_statements(&mut self, statements: Vec<IRNode>) -> Vec<EvalResult> {
        statements
            .into_iter()
            .map(|statement| self.eval_statement(statement))
            .collect()
    }

    fn eval_statement(&mut self, statement: IRNode) -> EvalResult {
        let (input, display) = unwrap_display(statement);
        let input_index = self.history.record_input(input.clone());
        let output = self.vm.eval(input.clone());
        let output_index = self.history.record_output(output.clone());
        EvalResult {
            input_index,
            output_index,
            input,
            output,
            display,
        }
    }
}

impl Default for MacsymaSession {
    fn default() -> Self {
        Self::new()
    }
}

fn unwrap_display(statement: IRNode) -> (IRNode, bool) {
    if let IRNode::Apply(apply) = statement {
        if apply.args.len() == 1 {
            if let IRNode::Symbol(head) = &apply.head {
                if head == DISPLAY {
                    return (apply.args[0].clone(), true);
                }
                if head == SUPPRESS {
                    return (apply.args[0].clone(), false);
                }
            }
        }
        return (IRNode::Apply(apply), true);
    }
    (statement, true)
}
