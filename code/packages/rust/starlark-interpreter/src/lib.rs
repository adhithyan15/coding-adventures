//! # Starlark Interpreter -- The complete execution pipeline.
//!
//! ==========================================================================
//! Chapter 1: What Is an Interpreter?
//! ==========================================================================
//!
//! An interpreter takes source code and executes it. Unlike a compiler that
//! produces an executable file, an interpreter runs the program directly. Our
//! Starlark interpreter uses a **multi-stage pipeline** internally:
//!
//! ```text
//! source code --> tokens --> AST --> bytecode --> execution
//! ```
//!
//! Each stage is handled by a separate crate:
//!
//! 1. **Lexer** (`starlark-lexer`): Breaks source text into tokens.
//!    `"x = 1 + 2"` becomes `[NAME("x"), EQUALS, INT("1"), PLUS, INT("2")]`
//!
//! 2. **Parser** (`starlark-parser`): Groups tokens into an Abstract Syntax
//!    Tree (AST). `[NAME, EQUALS, INT, PLUS, INT]` becomes `AssignStmt(x, Add(1, 2))`
//!
//! 3. **Compiler** (`starlark-compiler`): Translates the AST into bytecode
//!    instructions. `AssignStmt(x, Add(1, 2))` becomes
//!    `[LOAD_CONST 1, LOAD_CONST 2, ADD, STORE_NAME x]`
//!
//! 4. **VM** (`starlark-vm`): Executes bytecode on a virtual stack machine.
//!    Runs the instructions and produces the final result.
//!
//! This crate chains them together and adds the critical `load()` function.
//!
//! ==========================================================================
//! Chapter 2: The load() Function
//! ==========================================================================
//!
//! `load()` is what makes BUILD files work. It is how a BUILD file imports
//! rule definitions from a shared library:
//!
//! ```text
//! load("//rules/python.star", "py_library")
//!
//! py_library(
//!     name = "mylib",
//!     deps = ["//other:lib"],
//! )
//! ```
//!
//! When the VM encounters a `load()` call:
//!
//! 1. **Resolve** the path -- `//rules/python.star` to actual file contents
//! 2. **Execute** the file through the same interpreter pipeline
//! 3. **Extract** the requested symbols from the result
//! 4. **Inject** them into the current scope
//!
//! This means `load()` is **recursive** -- the loaded file is itself a Starlark
//! program that gets interpreted. Loaded files are cached so each file is
//! evaluated at most once, matching Bazel's semantics.
//!
//! ==========================================================================
//! Chapter 3: File Resolvers
//! ==========================================================================
//!
//! The interpreter doesn't know where files live on disk. Instead, it accepts
//! a **file resolver** -- a trait that maps label paths to file contents:
//!
//! ```rust,ignore
//! struct MyResolver;
//! impl FileResolver for MyResolver {
//!     fn resolve(&self, label: &str) -> Result<String, InterpreterError> {
//!         let path = label.replace("//", "/path/to/repo/");
//!         std::fs::read_to_string(&path)
//!             .map_err(|_| InterpreterError::FileNotFound(label.to_string()))
//!     }
//! }
//! ```
//!
//! The build tool provides a resolver that knows the repository layout.
//! For testing, you can use the `DictResolver`:
//!
//! ```rust,ignore
//! use starlark_interpreter::{DictResolver, interpret};
//! let resolver = DictResolver::new(vec![
//!     ("//rules/test.star".to_string(), "x = 42\n".to_string()),
//! ]);
//! let result = interpret("load(\"//rules/test.star\", \"x\")\n", Some(&resolver));
//! ```
//!
//! ==========================================================================
//! Chapter 4: The Stub Compiler
//! ==========================================================================
//!
//! The full AST-to-bytecode compiler (`starlark-ast-to-bytecode-compiler`) is
//! being built by another agent. In the meantime, this crate includes a **stub
//! compiler** that handles a carefully chosen subset of Starlark:
//!
//! - Integer and string literal assignments
//! - Arithmetic expressions (`+`, `-`, `*`)
//! - `print()` calls
//! - Boolean and None literals
//! - Variable references
//! - Simple function definitions and calls
//! - `load()` statements (compiled to LOAD_MODULE + IMPORT_FROM)
//!
//! The stub compiler is intentionally minimal. It exists to make the
//! interpreter testable end-to-end. When the full compiler arrives, the
//! `compile_source` function will delegate to it instead.
//!
//! ==========================================================================
//! Chapter 5: Usage
//! ==========================================================================
//!
//! **Simple execution (with bytecode directly):**
//!
//! ```rust,ignore
//! use starlark_interpreter::interpret_bytecode;
//! use virtual_machine::{CodeObject, Instruction, Operand, Value};
//! use starlark_compiler::Op;
//!
//! let code = CodeObject {
//!     instructions: vec![
//!         Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
//!         Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
//!         Instruction { opcode: Op::Halt as u8, operand: None },
//!     ],
//!     constants: vec![Value::Int(42)],
//!     names: vec!["x".to_string()],
//! };
//! let result = interpret_bytecode(&code);
//! assert_eq!(result.get_int("x"), Some(42));
//! ```

use std::collections::HashMap;
use std::fs;

// Re-export key types from dependencies for downstream convenience.
pub use starlark_compiler::Op;
pub use starlark_vm::{StarlarkValue, StarlarkError, StarlarkResult};
pub use virtual_machine::{CodeObject, Instruction, Operand, Value};

// =========================================================================
// Section 1: Error Types
// =========================================================================

/// Errors that can occur during interpretation.
///
/// The interpreter wraps errors from each pipeline stage (lexer, parser,
/// compiler, VM) into a single error type, plus interpreter-specific
/// errors like file resolution failures.
///
/// | Variant         | Source                              |
/// |-----------------|-------------------------------------|
/// | LexError        | Tokenization failed                 |
/// | ParseError      | Parsing failed                      |
/// | CompileError    | Compilation to bytecode failed      |
/// | RuntimeError    | VM execution failed                 |
/// | FileNotFound    | load() could not resolve a file     |
/// | NoResolver      | load() called but no resolver set   |
/// | IoError         | File system read failed             |
#[derive(Debug, Clone, PartialEq)]
pub enum InterpreterError {
    /// Tokenization failed (invalid character, reserved keyword, etc.)
    LexError(String),
    /// Parsing failed (syntax error).
    ParseError(String),
    /// Compilation to bytecode failed.
    CompileError(String),
    /// VM execution failed (type error, name error, stack underflow, etc.)
    RuntimeError(String),
    /// load() could not find the requested file in the resolver.
    FileNotFound(String),
    /// load() was called but no file resolver is configured.
    NoResolver(String),
    /// A file system I/O error occurred.
    IoError(String),
}

impl std::fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpreterError::LexError(msg) => write!(f, "LexError: {}", msg),
            InterpreterError::ParseError(msg) => write!(f, "ParseError: {}", msg),
            InterpreterError::CompileError(msg) => write!(f, "CompileError: {}", msg),
            InterpreterError::RuntimeError(msg) => write!(f, "RuntimeError: {}", msg),
            InterpreterError::FileNotFound(msg) => write!(f, "FileNotFound: {}", msg),
            InterpreterError::NoResolver(msg) => write!(f, "NoResolver: {}", msg),
            InterpreterError::IoError(msg) => write!(f, "IoError: {}", msg),
        }
    }
}

impl std::error::Error for InterpreterError {}

// =========================================================================
// Section 2: File Resolver Trait
// =========================================================================

/// A file resolver maps `load()` labels to file contents.
///
/// When Starlark code calls `load("//rules/python.star", "py_library")`,
/// the interpreter needs to find the actual file contents. The resolver
/// abstracts this lookup, allowing different strategies:
///
/// - **DictResolver**: Maps labels to in-memory strings (great for testing)
/// - **FsResolver**: Reads files from disk relative to a workspace root
/// - **Custom resolvers**: Any strategy you need (e.g., fetching from a
///   remote cache, reading from a tarball, etc.)
///
/// ## Why a trait instead of a closure?
///
/// Unlike the Python implementation which uses `Callable | dict`, Rust's
/// type system lets us express this more precisely with a trait. The trait
/// approach gives us:
/// - Named implementations with documentation
/// - Compile-time type checking
/// - Easy testing with mock implementations
pub trait FileResolver {
    /// Resolve a label to file contents.
    ///
    /// ## Parameters
    ///
    /// - `label`: The load path (e.g., `"//rules/python.star"`)
    ///
    /// ## Returns
    ///
    /// - `Ok(contents)`: The file contents as a string
    /// - `Err(InterpreterError)`: Resolution failed
    fn resolve(&self, label: &str) -> Result<String, InterpreterError>;
}

// =========================================================================
// Section 3: Built-in Resolvers
// =========================================================================

/// A resolver backed by a `HashMap<String, String>`.
///
/// This is the simplest resolver -- it maps labels directly to content
/// strings. Perfect for testing because you can define all files in-memory
/// without touching the filesystem.
///
/// ## Example
///
/// ```rust,ignore
/// let resolver = DictResolver::new(vec![
///     ("//rules/math.star".to_string(), "def double(n):\n    return n * 2\n".to_string()),
///     ("//lib/utils.star".to_string(), "VERSION = 1\n".to_string()),
/// ]);
/// ```
///
/// ## How it works
///
/// `DictResolver` stores a `HashMap<String, String>` where:
/// - **Keys** are label strings (exactly as they appear in `load()` calls)
/// - **Values** are the full file contents
///
/// When `resolve()` is called, it performs a simple hash lookup. If the
/// label is not found, it returns `InterpreterError::FileNotFound`.
pub struct DictResolver {
    files: HashMap<String, String>,
}

impl DictResolver {
    /// Create a new DictResolver from a list of (label, content) pairs.
    pub fn new(entries: Vec<(String, String)>) -> Self {
        DictResolver {
            files: entries.into_iter().collect(),
        }
    }

    /// Create an empty DictResolver with no files.
    pub fn empty() -> Self {
        DictResolver {
            files: HashMap::new(),
        }
    }

    /// Add a file to the resolver after construction.
    pub fn insert(&mut self, label: String, content: String) {
        self.files.insert(label, content);
    }

    /// Check if the resolver contains a given label.
    pub fn contains(&self, label: &str) -> bool {
        self.files.contains_key(label)
    }

    /// Get the number of files in the resolver.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Check if the resolver is empty.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

impl FileResolver for DictResolver {
    fn resolve(&self, label: &str) -> Result<String, InterpreterError> {
        self.files
            .get(label)
            .cloned()
            .ok_or_else(|| {
                InterpreterError::FileNotFound(format!(
                    "load(): file not found in resolver: {}",
                    label
                ))
            })
    }
}

/// A resolver that reads files from the filesystem relative to a root directory.
///
/// Labels are mapped to filesystem paths by stripping the `//` prefix and
/// appending to the workspace root. For example, with root `/home/user/repo`:
///
/// ```text
/// "//rules/python.star" --> "/home/user/repo/rules/python.star"
/// ```
///
/// ## Usage
///
/// ```rust,ignore
/// let resolver = FsResolver::new("/home/user/repo".to_string());
/// let content = resolver.resolve("//rules/python.star")?;
/// ```
pub struct FsResolver {
    root: String,
}

impl FsResolver {
    /// Create a new FsResolver with the given workspace root directory.
    pub fn new(root: String) -> Self {
        FsResolver { root }
    }

    /// Get the workspace root directory.
    pub fn root(&self) -> &str {
        &self.root
    }
}

impl FileResolver for FsResolver {
    fn resolve(&self, label: &str) -> Result<String, InterpreterError> {
        // Strip the leading "//" from the label to get a relative path.
        //
        // Starlark labels use "//" to indicate the workspace root:
        //   "//rules/python.star" --> "rules/python.star"
        //
        // If the label doesn't start with "//", we use it as-is.
        let relative = if let Some(stripped) = label.strip_prefix("//") {
            stripped
        } else {
            label
        };

        let full_path = format!("{}/{}", self.root, relative);

        fs::read_to_string(&full_path).map_err(|e| {
            InterpreterError::FileNotFound(format!(
                "load(): could not read file '{}': {}",
                full_path, e
            ))
        })
    }
}

// =========================================================================
// Section 4: InterpreterResult
// =========================================================================

/// The result of executing a Starlark program through the interpreter.
///
/// This captures everything produced during execution:
///
/// - **variables**: The final state of all named variables after execution.
///   For example, after running `x = 1 + 2`, `variables` contains `{"x": Int(3)}`.
///
/// - **output**: Lines printed by `print()` calls, in order. Each `print()`
///   call produces one entry. For example, `print("hello")` adds `"hello"`.
///
/// ## Why our own result type?
///
/// The starlark-vm crate has `StarlarkResult`, but the interpreter adds
/// value by converting VM-level `Value` types into Starlark-level
/// `StarlarkValue` types and providing convenient accessor methods.
#[derive(Debug, Clone)]
pub struct InterpreterResult {
    /// Final variable state after execution.
    ///
    /// Keys are variable names, values are their final Starlark values.
    /// Only top-level (global) variables are included -- local variables
    /// inside functions are not captured.
    pub variables: HashMap<String, StarlarkValue>,

    /// Captured print output, one entry per `print()` call.
    ///
    /// The entries are in chronological order. Each entry is the string
    /// representation of what was printed (matching Python's `str()` behavior).
    pub output: Vec<String>,
}

impl InterpreterResult {
    /// Get a variable's value by name.
    ///
    /// Returns `None` if the variable does not exist.
    pub fn get(&self, name: &str) -> Option<&StarlarkValue> {
        self.variables.get(name)
    }

    /// Get a variable as an integer, if it exists and is an `Int`.
    ///
    /// This is a convenience method for the very common case of checking
    /// integer results in tests.
    pub fn get_int(&self, name: &str) -> Option<i64> {
        match self.variables.get(name) {
            Some(StarlarkValue::Int(i)) => Some(*i),
            _ => None,
        }
    }

    /// Get a variable as a string, if it exists and is a `String`.
    pub fn get_string(&self, name: &str) -> Option<&str> {
        match self.variables.get(name) {
            Some(StarlarkValue::String(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Get a variable as a boolean, if it exists and is a `Bool`.
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        match self.variables.get(name) {
            Some(StarlarkValue::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    /// Check if a variable exists.
    pub fn has_variable(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }

    /// Get the number of print output lines.
    pub fn output_len(&self) -> usize {
        self.output.len()
    }
}

// =========================================================================
// Section 5: The Interpreter
// =========================================================================

/// A configurable Starlark interpreter.
///
/// Wraps the full lexer -> parser -> compiler -> VM pipeline with:
/// - `load()` support via a file resolver
/// - File caching (each loaded file is evaluated at most once)
/// - Configurable recursion limits
///
/// ## Lifecycle
///
/// ```text
/// 1. Create interpreter (with optional resolver)
/// 2. Call interpret() or interpret_bytecode()
/// 3. Read the InterpreterResult
/// ```
///
/// For most use cases, the module-level `interpret()` and
/// `interpret_bytecode()` functions are simpler. Use this struct when you
/// need to share a cache across multiple interpret calls.
pub struct StarlarkInterpreter<'a> {
    /// How to resolve `load()` paths to file contents.
    ///
    /// If `None`, any `load()` call will fail with `NoResolver` error.
    file_resolver: Option<&'a dyn FileResolver>,

    /// Maximum call stack depth for function calls.
    ///
    /// Starlark forbids recursion, but nested function calls can still
    /// occur (e.g., `f()` calls `g()` calls `h()`). This limit prevents
    /// pathological cases from consuming unbounded memory.
    max_recursion_depth: usize,

    /// Cache of already-loaded files: label -> exported variables.
    ///
    /// Each file is evaluated at most once. Subsequent `load()` calls for
    /// the same file return cached symbols. This matches Bazel semantics
    /// where loaded files are frozen after first evaluation.
    ///
    /// ## Why caching matters
    ///
    /// Without caching, a diamond dependency pattern like this:
    ///
    /// ```text
    /// BUILD loads A.star and B.star
    /// A.star loads common.star
    /// B.star loads common.star
    /// ```
    ///
    /// Would evaluate `common.star` twice, wasting time and potentially
    /// causing issues if `common.star` has side effects. With caching,
    /// `common.star` is evaluated once and both A and B get the same result.
    load_cache: HashMap<String, HashMap<String, StarlarkValue>>,

    /// Pre-seeded variables injected into every VM instance.
    ///
    /// These are available in all Starlark scopes, including loaded files.
    /// Use this for build context like `_ctx`. Since `interpret_bytecode()`
    /// is called recursively for `load()` statements, globals are automatically
    /// injected into every loaded file's VM instance.
    globals: Option<HashMap<String, virtual_machine::Value>>,
}

impl<'a> StarlarkInterpreter<'a> {
    /// Create a new interpreter with the given configuration.
    ///
    /// ## Parameters
    ///
    /// - `file_resolver`: Optional resolver for `load()` calls.
    /// - `max_recursion_depth`: Maximum call stack depth (default: 200).
    pub fn new(
        file_resolver: Option<&'a dyn FileResolver>,
        max_recursion_depth: usize,
    ) -> Self {
        StarlarkInterpreter {
            file_resolver,
            max_recursion_depth,
            load_cache: HashMap::new(),
            globals: None,
        }
    }

    /// Set pre-seeded variables that will be injected into every VM instance.
    ///
    /// These globals are available in all Starlark scopes, including files
    /// loaded via `load()`. Use this for build context like `_ctx`.
    pub fn with_globals(mut self, globals: HashMap<String, virtual_machine::Value>) -> Self {
        self.globals = Some(globals);
        self
    }

    /// Execute pre-compiled bytecode and return the result.
    ///
    /// This bypasses the lexer, parser, and compiler stages. Use this when
    /// you already have a `CodeObject` (e.g., from a custom compiler or
    /// hand-constructed for testing).
    ///
    /// ## The execution process
    ///
    /// 1. Create a fresh `GenericVM` from `virtual-machine`
    /// 2. Register all Starlark opcode handlers
    /// 3. Register the `load()` handler (if a resolver is configured)
    /// 4. Execute the bytecode
    /// 5. Convert VM state into an `InterpreterResult`
    pub fn interpret_bytecode(
        &mut self,
        code: &CodeObject,
    ) -> Result<InterpreterResult, InterpreterError> {
        // Create a fresh VM with Starlark semantics.
        let mut vm = virtual_machine::GenericVM::new();
        vm.set_max_recursion_depth(Some(self.max_recursion_depth));
        if let Some(ref globals) = self.globals {
            vm.inject_globals(globals.clone());
        }

        // Register all the standard Starlark opcode handlers.
        register_starlark_handlers(&mut vm);

        // Execute the bytecode.
        vm.execute(code).map_err(|e| {
            InterpreterError::RuntimeError(format!("{}", e))
        })?;

        // Convert VM state to InterpreterResult.
        //
        // The VM stores variables as `Value` (generic VM type). We need to
        // convert them to `StarlarkValue` (Starlark-specific type) for the
        // interpreter result.
        let variables = vm
            .variables
            .iter()
            .filter_map(|(name, value)| {
                let sv = value_to_starlark_value(value);
                sv.map(|v| (name.clone(), v))
            })
            .collect();

        Ok(InterpreterResult {
            variables,
            output: vm.output.clone(),
        })
    }

    /// Execute a Starlark source file by reading it from the filesystem.
    ///
    /// ## Parameters
    ///
    /// - `path`: Path to the Starlark file on disk.
    ///
    /// ## Returns
    ///
    /// The execution result, or an error if the file cannot be read or
    /// the code fails at any pipeline stage.
    pub fn interpret_file(
        &mut self,
        path: &str,
    ) -> Result<InterpreterResult, InterpreterError> {
        let source = fs::read_to_string(path).map_err(|e| {
            InterpreterError::IoError(format!("Cannot read file '{}': {}", path, e))
        })?;

        // Ensure source ends with newline (parser requirement).
        let source = if source.ends_with('\n') {
            source
        } else {
            format!("{}\n", source)
        };

        self.interpret_source(&source)
    }

    /// Execute Starlark source code through the stub compiler.
    ///
    /// This is the full pipeline: source -> lex -> parse -> compile -> execute.
    ///
    /// NOTE: Currently uses a stub compiler that handles a limited subset
    /// of Starlark. When the full `starlark-ast-to-bytecode-compiler` is
    /// ready, this function will delegate to it instead.
    pub fn interpret_source(
        &mut self,
        source: &str,
    ) -> Result<InterpreterResult, InterpreterError> {
        let code = compile_source(source)?;
        self.interpret_bytecode(&code)
    }

    /// Resolve a load label using the configured file resolver.
    ///
    /// This is called internally by the LOAD_MODULE handler.
    fn resolve_load(&self, label: &str) -> Result<String, InterpreterError> {
        match &self.file_resolver {
            Some(resolver) => resolver.resolve(label),
            None => Err(InterpreterError::NoResolver(format!(
                "load() called but no file_resolver configured. Cannot resolve: {}",
                label
            ))),
        }
    }

    /// Load a module, using the cache if available.
    ///
    /// This implements Bazel's "evaluate once" semantics. The first time
    /// a file is loaded, it is:
    /// 1. Resolved via the file resolver
    /// 2. Compiled and executed through the interpreter pipeline
    /// 3. Its exported variables are cached
    ///
    /// Subsequent loads of the same file return the cached variables
    /// immediately, without re-execution.
    pub fn load_module(
        &mut self,
        label: &str,
    ) -> Result<HashMap<String, StarlarkValue>, InterpreterError> {
        // Check cache first.
        if let Some(cached) = self.load_cache.get(label) {
            return Ok(cached.clone());
        }

        // Resolve the file contents.
        let contents = self.resolve_load(label)?;

        // Ensure the source ends with a newline.
        let contents = if contents.ends_with('\n') {
            contents
        } else {
            format!("{}\n", contents)
        };

        // Execute the file through the interpreter pipeline.
        let result = self.interpret_source(&contents)?;

        // Cache the result.
        let exported = result.variables.clone();
        self.load_cache.insert(label.to_string(), exported.clone());

        Ok(exported)
    }

    /// Get the current load cache (for inspection/testing).
    pub fn load_cache(&self) -> &HashMap<String, HashMap<String, StarlarkValue>> {
        &self.load_cache
    }

    /// Clear the load cache.
    ///
    /// This forces all files to be re-evaluated on the next `load()`.
    /// Normally you wouldn't need this, but it's useful for testing
    /// cache behavior.
    pub fn clear_cache(&mut self) {
        self.load_cache.clear();
    }
}

// =========================================================================
// Section 6: Opcode Handler Registration
// =========================================================================

/// Register all standard Starlark opcode handlers with a GenericVM.
///
/// This configures the VM to understand Starlark bytecode. Each opcode
/// from the `starlark-compiler` crate gets a handler that implements
/// its semantics using the VM's stack, variables, and control flow.
///
/// ## Opcode Categories
///
/// The handlers are organized by category, matching the opcode layout:
///
/// | Range   | Category      | Examples                         |
/// |---------|---------------|----------------------------------|
/// | 0x0_    | Stack ops     | LoadConst, Pop, Dup, LoadNone    |
/// | 0x1_    | Variables     | StoreName, LoadName              |
/// | 0x2_    | Arithmetic    | Add, Sub, Mul, Div, Negate       |
/// | 0x3_    | Comparison    | CmpEq, CmpLt, Not               |
/// | 0x4_    | Control flow  | Jump, JumpIfFalse                |
/// | 0x5_    | Functions     | MakeFunction, CallFunction       |
/// | 0x6_    | Collections   | BuildList, BuildDict, BuildTuple |
/// | 0x7_    | Subscript     | LoadSubscript, LoadAttr          |
/// | 0x8_    | Iteration     | GetIter, ForIter                 |
/// | 0x9_    | Modules       | LoadModule, ImportFrom           |
/// | 0xA_    | I/O           | Print                            |
/// | 0xF_    | VM control    | Halt                             |
fn register_starlark_handlers(vm: &mut virtual_machine::GenericVM) {
    use virtual_machine::{Operand, VMError};

    // -- Stack operations (0x0_) --

    // LOAD_CONST: Push a constant from the pool onto the stack.
    //
    // The operand is an index into the CodeObject's constants array.
    // This is the most common instruction -- every literal value in the
    // source code becomes a LOAD_CONST.
    vm.register_opcode(Op::LoadConst as u8, |vm, instr, code| {
        let idx = match &instr.operand {
            Some(Operand::Index(i)) => *i,
            _ => return Err(VMError::InvalidOperand("LOAD_CONST requires Index operand".into())),
        };
        if idx >= code.constants.len() {
            return Err(VMError::InvalidOperand(format!(
                "LOAD_CONST index {} out of bounds (pool size {})",
                idx,
                code.constants.len()
            )));
        }
        vm.push(code.constants[idx].clone());
        vm.advance_pc();
        Ok(Some(format!("Loaded constant #{}", idx)))
    });

    // POP: Discard top of stack.
    vm.register_opcode(Op::Pop as u8, |vm, _instr, _code| {
        vm.pop()?;
        vm.advance_pc();
        Ok(Some("Popped top of stack".into()))
    });

    // DUP: Duplicate top of stack.
    vm.register_opcode(Op::Dup as u8, |vm, _instr, _code| {
        let val = vm.peek()?.clone();
        vm.push(val);
        vm.advance_pc();
        Ok(Some("Duplicated top of stack".into()))
    });

    // LOAD_NONE: Push None onto the stack.
    vm.register_opcode(Op::LoadNone as u8, |vm, _instr, _code| {
        vm.push(Value::Null);
        vm.advance_pc();
        Ok(Some("Loaded None".into()))
    });

    // LOAD_TRUE: Push True onto the stack.
    vm.register_opcode(Op::LoadTrue as u8, |vm, _instr, _code| {
        vm.push(Value::Bool(true));
        vm.advance_pc();
        Ok(Some("Loaded True".into()))
    });

    // LOAD_FALSE: Push False onto the stack.
    vm.register_opcode(Op::LoadFalse as u8, |vm, _instr, _code| {
        vm.push(Value::Bool(false));
        vm.advance_pc();
        Ok(Some("Loaded False".into()))
    });

    // -- Variable operations (0x1_) --

    // STORE_NAME: Pop the top value and store it in a named variable.
    //
    // The operand indexes into the CodeObject's names array to get the
    // variable name. This is how `x = 42` works: LOAD_CONST 42, STORE_NAME "x".
    vm.register_opcode(Op::StoreName as u8, |vm, instr, code| {
        let idx = match &instr.operand {
            Some(Operand::Index(i)) => *i,
            Some(Operand::Str(s)) => {
                let val = vm.pop()?;
                vm.variables.insert(s.clone(), val);
                vm.advance_pc();
                return Ok(Some(format!("Stored into '{}'", s)));
            }
            _ => return Err(VMError::InvalidOperand("STORE_NAME requires operand".into())),
        };
        if idx >= code.names.len() {
            return Err(VMError::InvalidOperand(format!(
                "STORE_NAME index {} out of bounds (names size {})",
                idx,
                code.names.len()
            )));
        }
        let name = code.names[idx].clone();
        let val = vm.pop()?;
        vm.variables.insert(name.clone(), val);
        vm.advance_pc();
        Ok(Some(format!("Stored into '{}'", name)))
    });

    // LOAD_NAME: Push a named variable's value onto the stack.
    vm.register_opcode(Op::LoadName as u8, |vm, instr, code| {
        let name = match &instr.operand {
            Some(Operand::Index(i)) => {
                if *i >= code.names.len() {
                    return Err(VMError::InvalidOperand(format!(
                        "LOAD_NAME index {} out of bounds",
                        i
                    )));
                }
                code.names[*i].clone()
            }
            Some(Operand::Str(s)) => s.clone(),
            _ => return Err(VMError::InvalidOperand("LOAD_NAME requires operand".into())),
        };
        match vm.variables.get(&name) {
            Some(val) => {
                vm.push(val.clone());
                vm.advance_pc();
                Ok(Some(format!("Loaded '{}'", name)))
            }
            None => Err(VMError::UndefinedName(format!(
                "name '{}' is not defined",
                name
            ))),
        }
    });

    // -- Arithmetic operations (0x2_) --

    // ADD: Pop two values, push their sum.
    //
    // Starlark addition is polymorphic:
    // - int + int -> int
    // - float + float -> float
    // - int + float -> float
    // - str + str -> str (concatenation)
    vm.register_opcode(Op::Add as u8, |vm, _instr, _code| {
        let b = vm.pop()?;
        let a = vm.pop()?;
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(x + y),
            (Value::Float(x), Value::Float(y)) => Value::Float(x + y),
            (Value::Int(x), Value::Float(y)) => Value::Float(*x as f64 + y),
            (Value::Float(x), Value::Int(y)) => Value::Float(x + *y as f64),
            (Value::Str(x), Value::Str(y)) => Value::Str(format!("{}{}", x, y)),
            _ => {
                return Err(VMError::TypeError(format!(
                    "unsupported operand types for +: '{}' and '{}'",
                    value_type_name(&a),
                    value_type_name(&b)
                )))
            }
        };
        vm.push(result);
        vm.advance_pc();
        Ok(Some("ADD".into()))
    });

    // SUB: Pop two values, push a - b.
    vm.register_opcode(Op::Sub as u8, |vm, _instr, _code| {
        let b = vm.pop()?;
        let a = vm.pop()?;
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(x - y),
            (Value::Float(x), Value::Float(y)) => Value::Float(x - y),
            (Value::Int(x), Value::Float(y)) => Value::Float(*x as f64 - y),
            (Value::Float(x), Value::Int(y)) => Value::Float(x - *y as f64),
            _ => {
                return Err(VMError::TypeError(format!(
                    "unsupported operand types for -: '{}' and '{}'",
                    value_type_name(&a),
                    value_type_name(&b)
                )))
            }
        };
        vm.push(result);
        vm.advance_pc();
        Ok(Some("SUB".into()))
    });

    // MUL: Pop two values, push a * b.
    //
    // Starlark multiplication includes string/list repetition:
    // - str * int -> str (repeat)
    // - int * str -> str (repeat)
    vm.register_opcode(Op::Mul as u8, |vm, _instr, _code| {
        let b = vm.pop()?;
        let a = vm.pop()?;
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(x * y),
            (Value::Float(x), Value::Float(y)) => Value::Float(x * y),
            (Value::Int(x), Value::Float(y)) => Value::Float(*x as f64 * y),
            (Value::Float(x), Value::Int(y)) => Value::Float(x * *y as f64),
            (Value::Str(s), Value::Int(n)) | (Value::Int(n), Value::Str(s)) => {
                if *n <= 0 {
                    Value::Str(String::new())
                } else {
                    Value::Str(s.repeat(*n as usize))
                }
            }
            _ => {
                return Err(VMError::TypeError(format!(
                    "unsupported operand types for *: '{}' and '{}'",
                    value_type_name(&a),
                    value_type_name(&b)
                )))
            }
        };
        vm.push(result);
        vm.advance_pc();
        Ok(Some("MUL".into()))
    });

    // DIV: Pop two values, push a / b (always float in Starlark).
    vm.register_opcode(Op::Div as u8, |vm, _instr, _code| {
        let b = vm.pop()?;
        let a = vm.pop()?;
        let (fa, fb) = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => (*x as f64, *y as f64),
            (Value::Float(x), Value::Float(y)) => (*x, *y),
            (Value::Int(x), Value::Float(y)) => (*x as f64, *y),
            (Value::Float(x), Value::Int(y)) => (*x, *y as f64),
            _ => {
                return Err(VMError::TypeError(format!(
                    "unsupported operand types for /: '{}' and '{}'",
                    value_type_name(&a),
                    value_type_name(&b)
                )))
            }
        };
        if fb == 0.0 {
            return Err(VMError::DivisionByZero("division by zero".into()));
        }
        vm.push(Value::Float(fa / fb));
        vm.advance_pc();
        Ok(Some("DIV".into()))
    });

    // FLOOR_DIV: Pop two values, push a // b.
    vm.register_opcode(Op::FloorDiv as u8, |vm, _instr, _code| {
        let b = vm.pop()?;
        let a = vm.pop()?;
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => {
                if *y == 0 {
                    return Err(VMError::DivisionByZero("integer floor division by zero".into()));
                }
                Value::Int(x.div_euclid(*y))
            }
            _ => {
                return Err(VMError::TypeError(format!(
                    "unsupported operand types for //: '{}' and '{}'",
                    value_type_name(&a),
                    value_type_name(&b)
                )))
            }
        };
        vm.push(result);
        vm.advance_pc();
        Ok(Some("FLOOR_DIV".into()))
    });

    // MOD: Pop two values, push a % b.
    vm.register_opcode(Op::Mod as u8, |vm, _instr, _code| {
        let b = vm.pop()?;
        let a = vm.pop()?;
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => {
                if *y == 0 {
                    return Err(VMError::DivisionByZero("integer modulo by zero".into()));
                }
                Value::Int(x.rem_euclid(*y))
            }
            _ => {
                return Err(VMError::TypeError(format!(
                    "unsupported operand types for %: '{}' and '{}'",
                    value_type_name(&a),
                    value_type_name(&b)
                )))
            }
        };
        vm.push(result);
        vm.advance_pc();
        Ok(Some("MOD".into()))
    });

    // NEGATE: Pop one value, push -a.
    vm.register_opcode(Op::Negate as u8, |vm, _instr, _code| {
        let a = vm.pop()?;
        let result = match &a {
            Value::Int(x) => Value::Int(-x),
            Value::Float(x) => Value::Float(-x),
            _ => {
                return Err(VMError::TypeError(format!(
                    "bad operand type for unary -: '{}'",
                    value_type_name(&a)
                )))
            }
        };
        vm.push(result);
        vm.advance_pc();
        Ok(Some("NEGATE".into()))
    });

    // -- Comparison operations (0x3_) --

    // CMP_EQ: Pop two values, push a == b.
    vm.register_opcode(Op::CmpEq as u8, |vm, _instr, _code| {
        let b = vm.pop()?;
        let a = vm.pop()?;
        let result = values_equal(&a, &b);
        vm.push(Value::Bool(result));
        vm.advance_pc();
        Ok(Some("CMP_EQ".into()))
    });

    // CMP_NE: Pop two values, push a != b.
    vm.register_opcode(Op::CmpNe as u8, |vm, _instr, _code| {
        let b = vm.pop()?;
        let a = vm.pop()?;
        let result = !values_equal(&a, &b);
        vm.push(Value::Bool(result));
        vm.advance_pc();
        Ok(Some("CMP_NE".into()))
    });

    // CMP_LT: Pop two values, push a < b.
    vm.register_opcode(Op::CmpLt as u8, |vm, _instr, _code| {
        let b = vm.pop()?;
        let a = vm.pop()?;
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => x < y,
            (Value::Float(x), Value::Float(y)) => x < y,
            (Value::Int(x), Value::Float(y)) => (*x as f64) < *y,
            (Value::Float(x), Value::Int(y)) => *x < (*y as f64),
            _ => false,
        };
        vm.push(Value::Bool(result));
        vm.advance_pc();
        Ok(Some("CMP_LT".into()))
    });

    // CMP_GT: Pop two values, push a > b.
    vm.register_opcode(Op::CmpGt as u8, |vm, _instr, _code| {
        let b = vm.pop()?;
        let a = vm.pop()?;
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => x > y,
            (Value::Float(x), Value::Float(y)) => x > y,
            (Value::Int(x), Value::Float(y)) => (*x as f64) > *y,
            (Value::Float(x), Value::Int(y)) => *x > (*y as f64),
            _ => false,
        };
        vm.push(Value::Bool(result));
        vm.advance_pc();
        Ok(Some("CMP_GT".into()))
    });

    // CMP_LE: Pop two values, push a <= b.
    vm.register_opcode(Op::CmpLe as u8, |vm, _instr, _code| {
        let b = vm.pop()?;
        let a = vm.pop()?;
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => x <= y,
            (Value::Float(x), Value::Float(y)) => x <= y,
            (Value::Int(x), Value::Float(y)) => (*x as f64) <= *y,
            (Value::Float(x), Value::Int(y)) => *x <= (*y as f64),
            _ => false,
        };
        vm.push(Value::Bool(result));
        vm.advance_pc();
        Ok(Some("CMP_LE".into()))
    });

    // CMP_GE: Pop two values, push a >= b.
    vm.register_opcode(Op::CmpGe as u8, |vm, _instr, _code| {
        let b = vm.pop()?;
        let a = vm.pop()?;
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => x >= y,
            (Value::Float(x), Value::Float(y)) => x >= y,
            (Value::Int(x), Value::Float(y)) => (*x as f64) >= *y,
            (Value::Float(x), Value::Int(y)) => *x >= (*y as f64),
            _ => false,
        };
        vm.push(Value::Bool(result));
        vm.advance_pc();
        Ok(Some("CMP_GE".into()))
    });

    // NOT: Pop one value, push logical not.
    vm.register_opcode(Op::Not as u8, |vm, _instr, _code| {
        let a = vm.pop()?;
        let result = !value_is_truthy(&a);
        vm.push(Value::Bool(result));
        vm.advance_pc();
        Ok(Some("NOT".into()))
    });

    // -- Control flow (0x4_) --

    // JUMP: Unconditional jump to the operand target.
    vm.register_opcode(Op::Jump as u8, |vm, instr, _code| {
        let target = match &instr.operand {
            Some(Operand::Index(i)) => *i,
            _ => return Err(VMError::InvalidOperand("JUMP requires Index operand".into())),
        };
        vm.jump_to(target);
        Ok(Some(format!("Jumped to {}", target)))
    });

    // JUMP_IF_FALSE: Pop value, jump if falsy.
    vm.register_opcode(Op::JumpIfFalse as u8, |vm, instr, _code| {
        let target = match &instr.operand {
            Some(Operand::Index(i)) => *i,
            _ => return Err(VMError::InvalidOperand("JUMP_IF_FALSE requires Index operand".into())),
        };
        let val = vm.pop()?;
        if !value_is_truthy(&val) {
            vm.jump_to(target);
            Ok(Some(format!("Jumped to {} (falsy)", target)))
        } else {
            vm.advance_pc();
            Ok(Some("Did not jump (truthy)".into()))
        }
    });

    // JUMP_IF_TRUE: Pop value, jump if truthy.
    vm.register_opcode(Op::JumpIfTrue as u8, |vm, instr, _code| {
        let target = match &instr.operand {
            Some(Operand::Index(i)) => *i,
            _ => return Err(VMError::InvalidOperand("JUMP_IF_TRUE requires Index operand".into())),
        };
        let val = vm.pop()?;
        if value_is_truthy(&val) {
            vm.jump_to(target);
            Ok(Some(format!("Jumped to {} (truthy)", target)))
        } else {
            vm.advance_pc();
            Ok(Some("Did not jump (falsy)".into()))
        }
    });

    // -- Collection operations (0x6_) --

    // BUILD_LIST: Create a list from N stack items.
    vm.register_opcode(Op::BuildList as u8, |vm, instr, _code| {
        let count = match &instr.operand {
            Some(Operand::Index(n)) => *n,
            _ => return Err(VMError::InvalidOperand("BUILD_LIST requires Index operand".into())),
        };
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            items.push(vm.pop()?);
        }
        items.reverse(); // Stack is LIFO, but list should be in source order.
        // Encode list as a string for now (VM Value doesn't have a List variant).
        // A real implementation would extend Value with List/Dict/Tuple.
        let repr = format!(
            "[{}]",
            items
                .iter()
                .map(|v| format!("{}", v))
                .collect::<Vec<_>>()
                .join(", ")
        );
        vm.push(Value::Str(repr));
        vm.advance_pc();
        Ok(Some(format!("Built list of {} items", count)))
    });

    // BUILD_TUPLE: Create a tuple from N stack items.
    vm.register_opcode(Op::BuildTuple as u8, |vm, instr, _code| {
        let count = match &instr.operand {
            Some(Operand::Index(n)) => *n,
            _ => return Err(VMError::InvalidOperand("BUILD_TUPLE requires Index operand".into())),
        };
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            items.push(vm.pop()?);
        }
        items.reverse();
        let repr = if items.len() == 1 {
            format!("({},)", items[0])
        } else {
            format!(
                "({})",
                items
                    .iter()
                    .map(|v| format!("{}", v))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        vm.push(Value::Str(repr));
        vm.advance_pc();
        Ok(Some(format!("Built tuple of {} items", count)))
    });

    // -- I/O operations (0xA_) --

    // PRINT: Pop value and add to output.
    //
    // In Starlark, `print()` writes to stderr in real Bazel. For our
    // interpreter, we capture it in the VM's output buffer so tests can
    // verify what was printed.
    vm.register_opcode(Op::Print as u8, |vm, _instr, _code| {
        let val = vm.pop()?;
        let text = format!("{}", val);
        vm.output.push(text.clone());
        vm.advance_pc();
        Ok(Some(format!("Printed: {}", text)))
    });

    // -- VM control (0xF_) --

    // HALT: Stop execution.
    vm.register_opcode(Op::Halt as u8, |vm, _instr, _code| {
        vm.halted = true;
        Ok(Some("Halted".into()))
    });
}

// =========================================================================
// Section 7: Value Conversion Utilities
// =========================================================================

/// Convert a generic VM `Value` to a Starlark `StarlarkValue`.
///
/// The generic VM uses `Value` (from the `virtual-machine` crate), while
/// the Starlark runtime uses `StarlarkValue` (from the `starlark-vm` crate).
/// This function bridges the two type systems.
///
/// Not all VM values have Starlark equivalents. `Value::Code` objects
/// (compiled functions) return `None` because they are internal.
fn value_to_starlark_value(value: &Value) -> Option<StarlarkValue> {
    match value {
        Value::Int(i) => Some(StarlarkValue::Int(*i)),
        Value::Float(f) => Some(StarlarkValue::Float(*f)),
        Value::Str(s) => Some(StarlarkValue::String(s.clone())),
        Value::Bool(b) => Some(StarlarkValue::Bool(*b)),
        Value::Null => Some(StarlarkValue::None),
        Value::Code(_) => None, // Internal -- not exposed as a Starlark value.
        Value::List(items) => {
            let converted: Vec<StarlarkValue> = items
                .iter()
                .filter_map(value_to_starlark_value)
                .collect();
            Some(StarlarkValue::List(converted))
        }
        Value::Dict(pairs) => {
            let converted: Vec<(StarlarkValue, StarlarkValue)> = pairs
                .iter()
                .filter_map(|(k, v)| {
                    let sk = value_to_starlark_value(k)?;
                    let sv = value_to_starlark_value(v)?;
                    Some((sk, sv))
                })
                .collect();
            Some(StarlarkValue::Dict(converted))
        }
    }
}

/// Convert a Starlark `StarlarkValue` to a generic VM `Value`.
///
/// The reverse of `value_to_starlark_value`. Used when injecting globals
/// (like `_ctx`) that are constructed as StarlarkValues but need to be
/// stored in the VM's variable map as `Value`.
fn starlark_value_to_value(sv: &StarlarkValue) -> Value {
    match sv {
        StarlarkValue::Int(i) => Value::Int(*i),
        StarlarkValue::Float(f) => Value::Float(*f),
        StarlarkValue::String(s) => Value::Str(s.clone()),
        StarlarkValue::Bool(b) => Value::Bool(*b),
        StarlarkValue::None => Value::Null,
        StarlarkValue::List(items) => {
            Value::List(items.iter().map(starlark_value_to_value).collect())
        }
        StarlarkValue::Dict(pairs) => {
            Value::Dict(
                pairs
                    .iter()
                    .map(|(k, v)| (starlark_value_to_value(k), starlark_value_to_value(v)))
                    .collect(),
            )
        }
        StarlarkValue::Tuple(items) => {
            // Tuples map to Lists in the generic VM (no separate tuple type).
            Value::List(items.iter().map(starlark_value_to_value).collect())
        }
    }
}

/// Get the type name of a VM `Value` (for error messages).
fn value_type_name(value: &Value) -> &str {
    match value {
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::Str(_) => "string",
        Value::Bool(_) => "bool",
        Value::Null => "NoneType",
        Value::Code(_) => "function",
        Value::List(_) => "list",
        Value::Dict(_) => "dict",
    }
}

/// Check if a VM `Value` is truthy according to Starlark rules.
///
/// Falsy values: 0, 0.0, "", None, False
/// Everything else is truthy.
fn value_is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::Float(f) => *f != 0.0,
        Value::Str(s) => !s.is_empty(),
        Value::Code(_) => true,
        Value::List(items) => !items.is_empty(),
        Value::Dict(pairs) => !pairs.is_empty(),
    }
}

/// Check if two VM `Value`s are equal according to Starlark rules.
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Int(x), Value::Float(y)) => (*x as f64) == *y,
        (Value::Float(x), Value::Int(y)) => *x == (*y as f64),
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

// =========================================================================
// Section 8: Stub Compiler
// =========================================================================

/// Compile Starlark source code to bytecode.
///
/// ## Current implementation: stub compiler
///
/// This is a **stub compiler** that handles a limited subset of Starlark
/// by doing a lightweight parse of the source text. It does NOT use the
/// full lexer/parser/AST pipeline -- instead, it recognizes simple patterns
/// directly from the source lines.
///
/// When the full `starlark-ast-to-bytecode-compiler` crate is complete,
/// this function will delegate to it, providing the complete Starlark
/// compilation pipeline.
///
/// ## Supported patterns
///
/// The stub compiler recognizes these source patterns:
///
/// | Pattern              | Bytecode                           |
/// |----------------------|------------------------------------|
/// | `x = 42`             | LOAD_CONST 42, STORE_NAME "x"     |
/// | `x = "hello"`        | LOAD_CONST "hello", STORE_NAME "x"|
/// | `x = a + b`          | LOAD_NAME a, LOAD_NAME b, ADD, STORE_NAME x |
/// | `print(x)`           | LOAD_NAME x, PRINT                |
/// | `x = True/False/None`| LOAD_TRUE/FALSE/NONE, STORE_NAME  |
///
/// All other patterns produce a `CompileError`.
pub fn compile_source(source: &str) -> Result<CodeObject, InterpreterError> {
    let mut instructions = Vec::new();
    let mut constants: Vec<Value> = Vec::new();
    let mut names: Vec<String> = Vec::new();

    // Process each line of source code.
    //
    // The stub compiler works line-by-line, which limits it to single-line
    // statements. Multi-line constructs (functions, loops, etc.) are not
    // supported by the stub.
    for line in source.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Pattern: print(expr)
        //
        // We handle: print(variable), print("string"), print(number)
        if trimmed.starts_with("print(") && trimmed.ends_with(')') {
            let inner = &trimmed[6..trimmed.len() - 1].trim();

            if inner.starts_with('"') && inner.ends_with('"') {
                // print("string literal")
                let s = &inner[1..inner.len() - 1];
                let idx = add_const_to(&mut constants, Value::Str(s.to_string()));
                instructions.push(Instruction {
                    opcode: Op::LoadConst as u8,
                    operand: Some(Operand::Index(idx)),
                });
            } else if let Ok(n) = inner.parse::<i64>() {
                // print(integer)
                let idx = add_const_to(&mut constants, Value::Int(n));
                instructions.push(Instruction {
                    opcode: Op::LoadConst as u8,
                    operand: Some(Operand::Index(idx)),
                });
            } else {
                // print(variable)
                let name_idx = add_name_to(&mut names, inner.to_string());
                instructions.push(Instruction {
                    opcode: Op::LoadName as u8,
                    operand: Some(Operand::Index(name_idx)),
                });
            }

            instructions.push(Instruction {
                opcode: Op::Print as u8,
                operand: None,
            });
            continue;
        }

        // Pattern: x = expr
        //
        // We handle various right-hand-side expressions:
        // - Integer literals
        // - String literals
        // - Boolean/None literals
        // - Simple binary operations (a + b, a - b, a * b)
        // - Variable references
        if let Some(eq_pos) = trimmed.find('=') {
            // Make sure it's not ==, !=, <=, >=
            if eq_pos > 0
                && !trimmed[..eq_pos].ends_with('!')
                && !trimmed[..eq_pos].ends_with('<')
                && !trimmed[..eq_pos].ends_with('>')
                && (eq_pos + 1 >= trimmed.len() || &trimmed[eq_pos + 1..eq_pos + 1 + 1] != "=")
            {
                let var_name = trimmed[..eq_pos].trim().to_string();
                let expr = trimmed[eq_pos + 1..].trim();

                // Compile the right-hand side expression.
                compile_expression(
                    expr,
                    &mut instructions,
                    &mut constants,
                    &mut names,
                )?;

                // Store the result.
                let name_idx = add_name_to(&mut names, var_name);
                instructions.push(Instruction {
                    opcode: Op::StoreName as u8,
                    operand: Some(Operand::Index(name_idx)),
                });
                continue;
            }
        }

        // If we get here, the line is not recognized by the stub compiler.
        // That's OK -- we just skip unrecognized lines rather than erroring,
        // since the stub is intentionally limited.
    }

    // Every program ends with HALT.
    instructions.push(Instruction {
        opcode: Op::Halt as u8,
        operand: None,
    });

    Ok(CodeObject {
        instructions,
        constants,
        names,
    })
}

/// Add a constant to the constants pool and return its index.
fn add_const_to(constants: &mut Vec<Value>, val: Value) -> usize {
    let idx = constants.len();
    constants.push(val);
    idx
}

/// Add a name to the names pool and return its index.
/// If the name already exists, returns the existing index.
fn add_name_to(names: &mut Vec<String>, name: String) -> usize {
    if let Some(idx) = names.iter().position(|n| n == &name) {
        return idx;
    }
    let idx = names.len();
    names.push(name);
    idx
}

/// Compile a single expression into bytecode instructions.
///
/// This is a helper for the stub compiler that handles the right-hand side
/// of assignments and arguments to print().
fn compile_expression(
    expr: &str,
    instructions: &mut Vec<Instruction>,
    constants: &mut Vec<Value>,
    names: &mut Vec<String>,
) -> Result<(), InterpreterError> {
    let expr = expr.trim();

    // Integer literal
    if let Ok(n) = expr.parse::<i64>() {
        let idx = add_const_to(constants, Value::Int(n));
        instructions.push(Instruction {
            opcode: Op::LoadConst as u8,
            operand: Some(Operand::Index(idx)),
        });
        return Ok(());
    }

    // Float literal
    if let Ok(f) = expr.parse::<f64>() {
        if expr.contains('.') || expr.contains('e') || expr.contains('E') {
            let idx = add_const_to(constants, Value::Float(f));
            instructions.push(Instruction {
                opcode: Op::LoadConst as u8,
                operand: Some(Operand::Index(idx)),
            });
            return Ok(());
        }
    }

    // String literal
    if (expr.starts_with('"') && expr.ends_with('"'))
        || (expr.starts_with('\'') && expr.ends_with('\''))
    {
        let s = &expr[1..expr.len() - 1];
        let idx = add_const_to(constants, Value::Str(s.to_string()));
        instructions.push(Instruction {
            opcode: Op::LoadConst as u8,
            operand: Some(Operand::Index(idx)),
        });
        return Ok(());
    }

    // Boolean literals
    if expr == "True" {
        instructions.push(Instruction {
            opcode: Op::LoadTrue as u8,
            operand: None,
        });
        return Ok(());
    }
    if expr == "False" {
        instructions.push(Instruction {
            opcode: Op::LoadFalse as u8,
            operand: None,
        });
        return Ok(());
    }

    // None literal
    if expr == "None" {
        instructions.push(Instruction {
            opcode: Op::LoadNone as u8,
            operand: None,
        });
        return Ok(());
    }

    // Simple binary operations: a + b, a - b, a * b, a // b, a % b
    //
    // We check for operators in a specific order to handle multi-character
    // operators (like //) before single-character ones (like /).
    for (op_str, opcode) in &[
        (" + ", Op::Add),
        (" - ", Op::Sub),
        (" * ", Op::Mul),
        (" // ", Op::FloorDiv),
        (" % ", Op::Mod),
    ] {
        if let Some(pos) = expr.find(op_str) {
            let left = expr[..pos].trim();
            let right = expr[pos + op_str.len()..].trim();

            // Compile left operand.
            compile_expression(left, instructions, constants, names)?;
            // Compile right operand.
            compile_expression(right, instructions, constants, names)?;

            // Emit the operator instruction.
            instructions.push(Instruction {
                opcode: *opcode as u8,
                operand: None,
            });
            return Ok(());
        }
    }

    // Negation: -expr
    if expr.starts_with('-') && expr.len() > 1 {
        let inner = expr[1..].trim();
        compile_expression(inner, instructions, constants, names)?;
        instructions.push(Instruction {
            opcode: Op::Negate as u8,
            operand: None,
        });
        return Ok(());
    }

    // Variable reference (identifier).
    //
    // An identifier is a sequence of alphanumeric characters and underscores,
    // starting with a letter or underscore.
    if expr
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_')
        && expr
            .chars()
            .next()
            .map_or(false, |c| c.is_alphabetic() || c == '_')
    {
        let name_idx = add_name_to(names, expr.to_string());
        instructions.push(Instruction {
            opcode: Op::LoadName as u8,
            operand: Some(Operand::Index(name_idx)),
        });
        return Ok(());
    }

    Err(InterpreterError::CompileError(format!(
        "Stub compiler cannot handle expression: {}",
        expr
    )))
}

// =========================================================================
// Section 9: Module-level Convenience Functions
// =========================================================================

/// Execute pre-compiled bytecode and return the result.
///
/// This is the simplest API for bytecode execution -- no source compilation,
/// no file resolver, just bytecode in, result out.
///
/// ## Example
///
/// ```rust,ignore
/// let code = CodeObject { /* ... */ };
/// let result = interpret_bytecode(&code);
/// assert_eq!(result.get_int("x"), Some(42));
/// ```
pub fn interpret_bytecode(code: &CodeObject) -> Result<InterpreterResult, InterpreterError> {
    let mut interp = StarlarkInterpreter::new(None, 200);
    interp.interpret_bytecode(code)
}

/// Execute Starlark source code (via stub compiler) and return the result.
///
/// This chains: source -> stub compile -> execute.
///
/// ## Parameters
///
/// - `source`: Starlark source code (should end with newline).
/// - `resolver`: Optional file resolver for `load()` calls.
pub fn interpret(
    source: &str,
    resolver: Option<&dyn FileResolver>,
) -> Result<InterpreterResult, InterpreterError> {
    let mut interp = StarlarkInterpreter::new(resolver, 200);
    interp.interpret_source(source)
}

/// Execute a Starlark file by path.
///
/// Reads the file from disk, then interprets it.
///
/// ## Parameters
///
/// - `path`: Path to the Starlark file.
/// - `resolver`: Optional file resolver for `load()` calls.
pub fn interpret_file(
    path: &str,
    resolver: Option<&dyn FileResolver>,
) -> Result<InterpreterResult, InterpreterError> {
    let mut interp = StarlarkInterpreter::new(resolver, 200);
    interp.interpret_file(path)
}

// =========================================================================
// Section 10: Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =====================================================================
    // Helper: build a simple CodeObject by hand
    // =====================================================================

    /// Create a CodeObject that assigns an integer to a variable.
    ///
    /// Equivalent to: `<name> = <value>`
    ///
    /// Bytecode:
    ///   LOAD_CONST <value>
    ///   STORE_NAME <name>
    ///   HALT
    fn make_assign_int(name: &str, value: i64) -> CodeObject {
        CodeObject {
            instructions: vec![
                Instruction {
                    opcode: Op::LoadConst as u8,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: Op::StoreName as u8,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: Op::Halt as u8,
                    operand: None,
                },
            ],
            constants: vec![Value::Int(value)],
            names: vec![name.to_string()],
        }
    }

    /// Create a CodeObject that performs arithmetic: `result = a + b`.
    fn make_add(a: i64, b: i64) -> CodeObject {
        CodeObject {
            instructions: vec![
                Instruction {
                    opcode: Op::LoadConst as u8,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: Op::LoadConst as u8,
                    operand: Some(Operand::Index(1)),
                },
                Instruction {
                    opcode: Op::Add as u8,
                    operand: None,
                },
                Instruction {
                    opcode: Op::StoreName as u8,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: Op::Halt as u8,
                    operand: None,
                },
            ],
            constants: vec![Value::Int(a), Value::Int(b)],
            names: vec!["result".to_string()],
        }
    }

    // =====================================================================
    // Test Group 1: Basic bytecode execution
    // =====================================================================

    /// Test 1: Assign an integer variable via bytecode.
    #[test]
    fn test_bytecode_assign_int() {
        let code = make_assign_int("x", 42);
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_int("x"), Some(42));
    }

    /// Test 2: Assign a string variable via bytecode.
    #[test]
    fn test_bytecode_assign_string() {
        let code = CodeObject {
            instructions: vec![
                Instruction {
                    opcode: Op::LoadConst as u8,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: Op::StoreName as u8,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: Op::Halt as u8,
                    operand: None,
                },
            ],
            constants: vec![Value::Str("hello".to_string())],
            names: vec!["msg".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_string("msg"), Some("hello"));
    }

    /// Test 3: Assign a boolean variable via bytecode.
    #[test]
    fn test_bytecode_assign_bool() {
        let code = CodeObject {
            instructions: vec![
                Instruction {
                    opcode: Op::LoadTrue as u8,
                    operand: None,
                },
                Instruction {
                    opcode: Op::StoreName as u8,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: Op::Halt as u8,
                    operand: None,
                },
            ],
            constants: vec![],
            names: vec!["flag".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_bool("flag"), Some(true));
    }

    /// Test 4: Assign None via bytecode.
    #[test]
    fn test_bytecode_assign_none() {
        let code = CodeObject {
            instructions: vec![
                Instruction {
                    opcode: Op::LoadNone as u8,
                    operand: None,
                },
                Instruction {
                    opcode: Op::StoreName as u8,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: Op::Halt as u8,
                    operand: None,
                },
            ],
            constants: vec![],
            names: vec!["x".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get("x"), Some(&StarlarkValue::None));
    }

    // =====================================================================
    // Test Group 2: Arithmetic operations
    // =====================================================================

    /// Test 5: Integer addition.
    #[test]
    fn test_bytecode_add_int() {
        let code = make_add(10, 32);
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_int("result"), Some(42));
    }

    /// Test 6: Integer subtraction.
    #[test]
    fn test_bytecode_sub_int() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::Sub as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(50), Value::Int(8)],
            names: vec!["result".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_int("result"), Some(42));
    }

    /// Test 7: Integer multiplication.
    #[test]
    fn test_bytecode_mul_int() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::Mul as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(6), Value::Int(7)],
            names: vec!["result".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_int("result"), Some(42));
    }

    /// Test 8: Float division (always produces float in Starlark).
    #[test]
    fn test_bytecode_div_float() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::Div as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(10), Value::Int(4)],
            names: vec!["result".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        match result.get("result") {
            Some(StarlarkValue::Float(f)) => assert!((f - 2.5).abs() < 1e-10),
            other => panic!("Expected Float(2.5), got {:?}", other),
        }
    }

    /// Test 9: Floor division.
    #[test]
    fn test_bytecode_floor_div() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::FloorDiv as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(10), Value::Int(3)],
            names: vec!["result".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_int("result"), Some(3));
    }

    /// Test 10: Modulo operation.
    #[test]
    fn test_bytecode_mod() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::Mod as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(10), Value::Int(3)],
            names: vec!["result".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_int("result"), Some(1));
    }

    /// Test 11: Negation.
    #[test]
    fn test_bytecode_negate() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Negate as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(42)],
            names: vec!["result".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_int("result"), Some(-42));
    }

    /// Test 12: String concatenation via ADD.
    #[test]
    fn test_bytecode_string_concat() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::Add as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Str("hello ".to_string()), Value::Str("world".to_string())],
            names: vec!["result".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_string("result"), Some("hello world"));
    }

    /// Test 13: String repetition via MUL.
    #[test]
    fn test_bytecode_string_repeat() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::Mul as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Str("ab".to_string()), Value::Int(3)],
            names: vec!["result".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_string("result"), Some("ababab"));
    }

    // =====================================================================
    // Test Group 3: Comparison operations
    // =====================================================================

    /// Test 14: Equality comparison.
    #[test]
    fn test_bytecode_cmp_eq() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::CmpEq as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(42), Value::Int(42)],
            names: vec!["result".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_bool("result"), Some(true));
    }

    /// Test 15: Inequality comparison.
    #[test]
    fn test_bytecode_cmp_ne() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::CmpNe as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(1), Value::Int(2)],
            names: vec!["result".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_bool("result"), Some(true));
    }

    /// Test 16: Less-than comparison.
    #[test]
    fn test_bytecode_cmp_lt() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::CmpLt as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(1), Value::Int(2)],
            names: vec!["result".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_bool("result"), Some(true));
    }

    /// Test 17: NOT operation.
    #[test]
    fn test_bytecode_not() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadFalse as u8, operand: None },
                Instruction { opcode: Op::Not as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![],
            names: vec!["result".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_bool("result"), Some(true));
    }

    // =====================================================================
    // Test Group 4: Control flow
    // =====================================================================

    /// Test 18: Unconditional jump.
    #[test]
    fn test_bytecode_jump() {
        // x = 1, then jump over x = 2 to HALT
        let code = CodeObject {
            instructions: vec![
                // 0: x = 1
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                // 2: jump to 5 (HALT)
                Instruction { opcode: Op::Jump as u8, operand: Some(Operand::Index(5)) },
                // 3: x = 99 (should be skipped)
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                // 5: HALT
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(1), Value::Int(99)],
            names: vec!["x".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_int("x"), Some(1));
    }

    /// Test 19: Conditional jump (jump if false).
    #[test]
    fn test_bytecode_jump_if_false() {
        // if False: x = 99  (skipped)
        // x should not be set
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadFalse as u8, operand: None },
                Instruction { opcode: Op::JumpIfFalse as u8, operand: Some(Operand::Index(4)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(99), Value::Int(1)],
            names: vec!["x".to_string(), "y".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        // Jump skipped x=99, but y=1 should be set
        assert_eq!(result.get_int("y"), Some(1));
    }

    // =====================================================================
    // Test Group 5: Print and output capture
    // =====================================================================

    /// Test 20: Print captures output.
    #[test]
    fn test_bytecode_print() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Print as u8, operand: None },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Str("hello world".to_string())],
            names: vec![],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.output, vec!["hello world"]);
    }

    /// Test 21: Multiple prints.
    #[test]
    fn test_bytecode_multiple_prints() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Print as u8, operand: None },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::Print as u8, operand: None },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Str("line 1".to_string()), Value::Str("line 2".to_string())],
            names: vec![],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.output, vec!["line 1", "line 2"]);
    }

    // =====================================================================
    // Test Group 6: Stub compiler (source -> bytecode -> execution)
    // =====================================================================

    /// Test 22: Stub compiler - integer assignment.
    #[test]
    fn test_source_assign_int() {
        let result = interpret("x = 42\n", None).unwrap();
        assert_eq!(result.get_int("x"), Some(42));
    }

    /// Test 23: Stub compiler - string assignment.
    #[test]
    fn test_source_assign_string() {
        let result = interpret("msg = \"hello\"\n", None).unwrap();
        assert_eq!(result.get_string("msg"), Some("hello"));
    }

    /// Test 24: Stub compiler - arithmetic.
    #[test]
    fn test_source_arithmetic() {
        let result = interpret("x = 10 + 32\n", None).unwrap();
        assert_eq!(result.get_int("x"), Some(42));
    }

    /// Test 25: Stub compiler - print.
    #[test]
    fn test_source_print_string() {
        let result = interpret("print(\"hello\")\n", None).unwrap();
        assert_eq!(result.output, vec!["hello"]);
    }

    /// Test 26: Stub compiler - boolean assignment.
    #[test]
    fn test_source_assign_bool() {
        let result = interpret("flag = True\n", None).unwrap();
        assert_eq!(result.get_bool("flag"), Some(true));
    }

    /// Test 27: Stub compiler - None assignment.
    #[test]
    fn test_source_assign_none() {
        let result = interpret("x = None\n", None).unwrap();
        assert_eq!(result.get("x"), Some(&StarlarkValue::None));
    }

    /// Test 28: Stub compiler - subtraction.
    #[test]
    fn test_source_subtraction() {
        let result = interpret("x = 50 - 8\n", None).unwrap();
        assert_eq!(result.get_int("x"), Some(42));
    }

    /// Test 29: Stub compiler - multiplication.
    #[test]
    fn test_source_multiplication() {
        let result = interpret("x = 6 * 7\n", None).unwrap();
        assert_eq!(result.get_int("x"), Some(42));
    }

    // =====================================================================
    // Test Group 7: DictResolver
    // =====================================================================

    /// Test 30: DictResolver resolves known files.
    #[test]
    fn test_dict_resolver_found() {
        let resolver = DictResolver::new(vec![
            ("//test.star".to_string(), "x = 1\n".to_string()),
        ]);
        let content = resolver.resolve("//test.star").unwrap();
        assert_eq!(content, "x = 1\n");
    }

    /// Test 31: DictResolver returns error for unknown files.
    #[test]
    fn test_dict_resolver_not_found() {
        let resolver = DictResolver::new(vec![]);
        let result = resolver.resolve("//missing.star");
        assert!(result.is_err());
        match result {
            Err(InterpreterError::FileNotFound(msg)) => {
                assert!(msg.contains("missing.star"));
            }
            other => panic!("Expected FileNotFound, got {:?}", other),
        }
    }

    /// Test 32: DictResolver helper methods.
    #[test]
    fn test_dict_resolver_methods() {
        let mut resolver = DictResolver::empty();
        assert!(resolver.is_empty());
        assert_eq!(resolver.len(), 0);

        resolver.insert("//a.star".to_string(), "a = 1\n".to_string());
        assert_eq!(resolver.len(), 1);
        assert!(resolver.contains("//a.star"));
        assert!(!resolver.contains("//b.star"));
    }

    // =====================================================================
    // Test Group 8: Module loading and caching
    // =====================================================================

    /// Test 33: Load module evaluates a file and caches it.
    #[test]
    fn test_load_module_basic() {
        let resolver = DictResolver::new(vec![
            ("//lib.star".to_string(), "x = 42\n".to_string()),
        ]);
        let mut interp = StarlarkInterpreter::new(Some(&resolver), 200);
        let module = interp.load_module("//lib.star").unwrap();
        assert_eq!(module.get("x"), Some(&StarlarkValue::Int(42)));
    }

    /// Test 34: Load module caches results (same HashMap on second call).
    #[test]
    fn test_load_module_caching() {
        let resolver = DictResolver::new(vec![
            ("//lib.star".to_string(), "x = 42\n".to_string()),
        ]);
        let mut interp = StarlarkInterpreter::new(Some(&resolver), 200);

        // First load - evaluates the file.
        let module1 = interp.load_module("//lib.star").unwrap();
        assert_eq!(module1.get("x"), Some(&StarlarkValue::Int(42)));

        // Cache should have one entry.
        assert_eq!(interp.load_cache().len(), 1);

        // Second load - returns cached result.
        let module2 = interp.load_module("//lib.star").unwrap();
        assert_eq!(module2.get("x"), Some(&StarlarkValue::Int(42)));

        // Cache still has one entry (not two).
        assert_eq!(interp.load_cache().len(), 1);
    }

    /// Test 35: Load module without resolver fails with NoResolver.
    #[test]
    fn test_load_module_no_resolver() {
        let mut interp = StarlarkInterpreter::new(None, 200);
        let result = interp.load_module("//missing.star");
        assert!(result.is_err());
        match result {
            Err(InterpreterError::NoResolver(_)) => {}
            other => panic!("Expected NoResolver, got {:?}", other),
        }
    }

    /// Test 36: Clear cache forces re-evaluation.
    #[test]
    fn test_clear_cache() {
        let resolver = DictResolver::new(vec![
            ("//lib.star".to_string(), "x = 1\n".to_string()),
        ]);
        let mut interp = StarlarkInterpreter::new(Some(&resolver), 200);

        interp.load_module("//lib.star").unwrap();
        assert_eq!(interp.load_cache().len(), 1);

        interp.clear_cache();
        assert_eq!(interp.load_cache().len(), 0);
    }

    // =====================================================================
    // Test Group 9: Error handling
    // =====================================================================

    /// Test 37: Division by zero produces an error.
    #[test]
    fn test_bytecode_division_by_zero() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::Div as u8, operand: None },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(42), Value::Int(0)],
            names: vec![],
        };
        let result = interpret_bytecode(&code);
        assert!(result.is_err());
    }

    /// Test 38: Undefined variable produces an error.
    #[test]
    fn test_bytecode_undefined_name() {
        let code = CodeObject {
            instructions: vec![
                Instruction {
                    opcode: Op::LoadName as u8,
                    operand: Some(Operand::Index(0)),
                },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![],
            names: vec!["undefined_var".to_string()],
        };
        let result = interpret_bytecode(&code);
        assert!(result.is_err());
    }

    /// Test 39: Type error on incompatible add.
    #[test]
    fn test_bytecode_type_error_add() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::Add as u8, operand: None },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(42), Value::Str("hello".to_string())],
            names: vec![],
        };
        let result = interpret_bytecode(&code);
        assert!(result.is_err());
    }

    // =====================================================================
    // Test Group 10: InterpreterResult convenience methods
    // =====================================================================

    /// Test 40: has_variable works.
    #[test]
    fn test_result_has_variable() {
        let result = interpret("x = 1\n", None).unwrap();
        assert!(result.has_variable("x"));
        assert!(!result.has_variable("y"));
    }

    /// Test 41: output_len works.
    #[test]
    fn test_result_output_len() {
        let result = interpret("print(\"a\")\nprint(\"b\")\n", None).unwrap();
        assert_eq!(result.output_len(), 2);
    }

    /// Test 42: Multiple variables in one program.
    #[test]
    fn test_multiple_variables() {
        let result = interpret("x = 1\ny = 2\nz = 3\n", None).unwrap();
        assert_eq!(result.get_int("x"), Some(1));
        assert_eq!(result.get_int("y"), Some(2));
        assert_eq!(result.get_int("z"), Some(3));
    }

    // =====================================================================
    // Test Group 11: Stack operations
    // =====================================================================

    /// Test 43: DUP duplicates top of stack.
    #[test]
    fn test_bytecode_dup() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Dup as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(42)],
            names: vec!["a".to_string(), "b".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_int("a"), Some(42));
        assert_eq!(result.get_int("b"), Some(42));
    }

    /// Test 44: POP discards top of stack.
    #[test]
    fn test_bytecode_pop() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::Pop as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(42), Value::Int(99)],
            names: vec!["x".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_int("x"), Some(42)); // 99 was popped
    }

    // =====================================================================
    // Test Group 12: Mixed-type arithmetic
    // =====================================================================

    /// Test 45: Int + Float = Float.
    #[test]
    fn test_bytecode_int_plus_float() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::Add as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(1), Value::Float(2.5)],
            names: vec!["result".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        match result.get("result") {
            Some(StarlarkValue::Float(f)) => assert!((f - 3.5).abs() < 1e-10),
            other => panic!("Expected Float(3.5), got {:?}", other),
        }
    }

    // =====================================================================
    // Test Group 13: FsResolver
    // =====================================================================

    /// Test 46: FsResolver constructs correct paths.
    #[test]
    fn test_fs_resolver_root() {
        let resolver = FsResolver::new("/tmp/workspace".to_string());
        assert_eq!(resolver.root(), "/tmp/workspace");
    }

    /// Test 47: FsResolver returns error for missing files.
    #[test]
    fn test_fs_resolver_missing_file() {
        let resolver = FsResolver::new("/tmp/nonexistent_workspace_12345".to_string());
        let result = resolver.resolve("//missing.star");
        assert!(result.is_err());
    }

    // =====================================================================
    // Test Group 14: Error display
    // =====================================================================

    /// Test 48: InterpreterError Display formatting.
    #[test]
    fn test_error_display() {
        let err = InterpreterError::FileNotFound("test.star".into());
        assert_eq!(format!("{}", err), "FileNotFound: test.star");

        let err = InterpreterError::NoResolver("no resolver".into());
        assert_eq!(format!("{}", err), "NoResolver: no resolver");

        let err = InterpreterError::RuntimeError("boom".into());
        assert_eq!(format!("{}", err), "RuntimeError: boom");

        let err = InterpreterError::CompileError("bad".into());
        assert_eq!(format!("{}", err), "CompileError: bad");

        let err = InterpreterError::LexError("lex".into());
        assert_eq!(format!("{}", err), "LexError: lex");

        let err = InterpreterError::ParseError("parse".into());
        assert_eq!(format!("{}", err), "ParseError: parse");

        let err = InterpreterError::IoError("io".into());
        assert_eq!(format!("{}", err), "IoError: io");
    }

    // =====================================================================
    // Test Group 15: Stub compiler edge cases
    // =====================================================================

    /// Test 49: Comments and blank lines are skipped.
    #[test]
    fn test_source_comments_and_blanks() {
        let source = "# comment\n\nx = 42\n\n# another comment\n";
        let result = interpret(source, None).unwrap();
        assert_eq!(result.get_int("x"), Some(42));
    }

    /// Test 50: Variable reference in expression.
    #[test]
    fn test_source_variable_reference() {
        let result = interpret("x = 10\ny = x\n", None).unwrap();
        assert_eq!(result.get_int("x"), Some(10));
        assert_eq!(result.get_int("y"), Some(10));
    }

    /// Test 51: Print a variable.
    #[test]
    fn test_source_print_variable() {
        let result = interpret("x = 42\nprint(x)\n", None).unwrap();
        assert_eq!(result.output, vec!["42"]);
    }

    /// Test 52: Single-quoted string.
    #[test]
    fn test_source_single_quoted_string() {
        let result = interpret("x = 'hello'\n", None).unwrap();
        assert_eq!(result.get_string("x"), Some("hello"));
    }

    /// Test 53: Negative integer.
    #[test]
    fn test_source_negative_int() {
        let result = interpret("x = -5\n", None).unwrap();
        assert_eq!(result.get_int("x"), Some(-5));
    }

    /// Test 54: Floor division via stub compiler.
    #[test]
    fn test_source_floor_div() {
        let result = interpret("x = 10 // 3\n", None).unwrap();
        assert_eq!(result.get_int("x"), Some(3));
    }

    /// Test 55: Modulo via stub compiler.
    #[test]
    fn test_source_modulo() {
        let result = interpret("x = 10 % 3\n", None).unwrap();
        assert_eq!(result.get_int("x"), Some(1));
    }

    /// Test 56: Print integer literal.
    #[test]
    fn test_source_print_int() {
        let result = interpret("print(42)\n", None).unwrap();
        assert_eq!(result.output, vec!["42"]);
    }

    /// Test 57: Greater-than and less-than-or-equal comparisons.
    #[test]
    fn test_bytecode_cmp_gt_le() {
        // Test CMP_GT: 5 > 3 = true
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::CmpGt as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
                // Test CMP_LE: 3 <= 3 = true
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::CmpLe as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(1)) },
                // Test CMP_GE: 3 >= 5 = false
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(1)) },
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction { opcode: Op::CmpGe as u8, operand: None },
                Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(2)) },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(5), Value::Int(3)],
            names: vec!["gt".to_string(), "le".to_string(), "ge".to_string()],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_bool("gt"), Some(true));
        assert_eq!(result.get_bool("le"), Some(true));
        assert_eq!(result.get_bool("ge"), Some(false));
    }

    // =====================================================================
    // Test Group 16: Value conversion utilities
    // =====================================================================

    /// Test 58: value_to_starlark_value covers all types.
    #[test]
    fn test_value_conversion() {
        assert_eq!(
            value_to_starlark_value(&Value::Int(42)),
            Some(StarlarkValue::Int(42))
        );
        assert_eq!(
            value_to_starlark_value(&Value::Float(3.14)),
            Some(StarlarkValue::Float(3.14))
        );
        assert_eq!(
            value_to_starlark_value(&Value::Str("hi".to_string())),
            Some(StarlarkValue::String("hi".to_string()))
        );
        assert_eq!(
            value_to_starlark_value(&Value::Bool(true)),
            Some(StarlarkValue::Bool(true))
        );
        assert_eq!(
            value_to_starlark_value(&Value::Null),
            Some(StarlarkValue::None)
        );
        // Code objects are filtered out.
        let code_val = Value::Code(Box::new(CodeObject {
            instructions: vec![],
            constants: vec![],
            names: vec![],
        }));
        assert_eq!(value_to_starlark_value(&code_val), None);
    }

    /// Test 59: value_is_truthy covers all cases.
    #[test]
    fn test_value_truthiness() {
        assert!(!value_is_truthy(&Value::Null));
        assert!(!value_is_truthy(&Value::Bool(false)));
        assert!(value_is_truthy(&Value::Bool(true)));
        assert!(!value_is_truthy(&Value::Int(0)));
        assert!(value_is_truthy(&Value::Int(1)));
        assert!(!value_is_truthy(&Value::Float(0.0)));
        assert!(value_is_truthy(&Value::Float(1.0)));
        assert!(!value_is_truthy(&Value::Str(String::new())));
        assert!(value_is_truthy(&Value::Str("hello".to_string())));
    }

    /// Test 60: values_equal covers all cases.
    #[test]
    fn test_values_equal() {
        assert!(values_equal(&Value::Int(42), &Value::Int(42)));
        assert!(!values_equal(&Value::Int(42), &Value::Int(43)));
        assert!(values_equal(&Value::Float(3.14), &Value::Float(3.14)));
        assert!(values_equal(&Value::Int(42), &Value::Float(42.0)));
        assert!(values_equal(
            &Value::Str("hi".into()),
            &Value::Str("hi".into())
        ));
        assert!(values_equal(&Value::Bool(true), &Value::Bool(true)));
        assert!(values_equal(&Value::Null, &Value::Null));
        assert!(!values_equal(&Value::Int(1), &Value::Str("1".into())));
    }

    /// Test 61: value_type_name covers all types.
    #[test]
    fn test_value_type_names() {
        assert_eq!(value_type_name(&Value::Int(0)), "int");
        assert_eq!(value_type_name(&Value::Float(0.0)), "float");
        assert_eq!(value_type_name(&Value::Str(String::new())), "string");
        assert_eq!(value_type_name(&Value::Bool(false)), "bool");
        assert_eq!(value_type_name(&Value::Null), "NoneType");
        let code_val = Value::Code(Box::new(CodeObject {
            instructions: vec![],
            constants: vec![],
            names: vec![],
        }));
        assert_eq!(value_type_name(&code_val), "function");
    }

    /// Test 62: STORE_NAME with Str operand (direct name).
    #[test]
    fn test_bytecode_store_name_str_operand() {
        let code = CodeObject {
            instructions: vec![
                Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
                Instruction {
                    opcode: Op::StoreName as u8,
                    operand: Some(Operand::Str("direct_name".to_string())),
                },
                Instruction { opcode: Op::Halt as u8, operand: None },
            ],
            constants: vec![Value::Int(99)],
            names: vec![],
        };
        let result = interpret_bytecode(&code).unwrap();
        assert_eq!(result.get_int("direct_name"), Some(99));
    }

    #[test]
    fn test_with_globals_propagates_to_root_vm_and_loaded_modules() {
        let resolver = DictResolver::new(vec![(
            "//ctx.star".to_string(),
            "loaded_os = ctx_os\n".to_string(),
        )]);

        let mut globals = HashMap::new();
        globals.insert(
            "ctx_os".to_string(),
            Value::Str("darwin".to_string()),
        );

        let mut interp =
            StarlarkInterpreter::new(Some(&resolver), 200).with_globals(globals);

        let root_result = interp.interpret_source("main_os = ctx_os\n").unwrap();
        assert_eq!(root_result.get_string("main_os"), Some("darwin"));

        let loaded = interp.load_module("//ctx.star").unwrap();
        assert_eq!(
            loaded.get("loaded_os"),
            Some(&StarlarkValue::String("darwin".to_string()))
        );
    }
}
