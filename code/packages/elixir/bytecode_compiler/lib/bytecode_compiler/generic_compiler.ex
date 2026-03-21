defmodule CodingAdventures.BytecodeCompiler.GenericCompiler do
  @moduledoc """
  GenericCompiler — A Pluggable AST-to-Bytecode Compiler
  ======================================================

  This is the heart of the bytecode compiler package. It implements a
  *pluggable* compilation framework: instead of knowing how to compile
  any particular language, it lets you register handler functions for
  each AST node type. When it encounters a node during compilation, it
  looks up the matching handler and calls it.

  ## The Big Idea

  Think of the compiler as a switchboard operator. An AST node arrives
  with a label (its `rule_name`). The operator checks the directory
  (the `dispatch` map) and connects the call to the right handler.

  If no handler is registered and the node has exactly one child, the
  compiler "passes through" to that child. This is crucial for grammars
  that use extra rules for operator precedence — those wrapper nodes
  don't need explicit handlers.

  ## Immutable State

  Unlike imperative compilers that mutate internal state, this compiler
  is purely functional. Every operation takes a compiler struct and
  returns a *new* compiler struct with the updated state. This is
  idiomatic Elixir and makes the compiler easy to reason about.

  ## Architecture

      +------------------+
      |  GenericCompiler  |
      |                  |
      |  instructions: []|  <-- bytecode we're building
      |  constants:   [] |  <-- literal values (42, "hello")
      |  names:       [] |  <-- variable/function names
      |  dispatch:    %{}|  <-- rule_name => handler fn
      |  scope:       nil|  <-- local variable tracking
      +------------------+

  ## AST Node Format

  AST nodes are plain maps with two shapes:

  - **Rule nodes**: `%{rule_name: "expr", children: [...]}`
    These are interior nodes produced by the parser.

  - **Token nodes**: `%{type: "NUMBER", value: "42"}`
    These are leaf nodes (terminals) from the lexer.

  This keeps things simple — no need for a separate AST package.

  ## Handler Signature

  Every handler is a function of arity 2:

      fn(compiler, node) -> compiler

  The handler receives the current compiler state and the AST node,
  and returns the updated compiler state after emitting instructions.
  """

  alias CodingAdventures.VirtualMachine.Types.Instruction
  alias CodingAdventures.VirtualMachine.Types.CodeObject

  # ---------------------------------------------------------------------------
  # Struct Definition
  # ---------------------------------------------------------------------------
  #
  # The compiler state is a simple struct with five fields:
  #
  # - `instructions` — the list of bytecode instructions we've emitted so far.
  #   Stored in forward order (first emitted = first in list). We append to the
  #   end, which is O(n) in Elixir, but compilation is not performance-critical
  #   and this keeps the code simple.
  #
  # - `constants` — the constant pool. When we encounter a literal like `42`,
  #   we store it here and reference it by index in the instruction operand.
  #   Deduplication ensures each unique value appears only once.
  #
  # - `names` — the name pool. Variable names like `"x"` are stored here and
  #   referenced by index, just like constants.
  #
  # - `dispatch` — the rule handler registry. Maps rule name strings to
  #   handler functions. This is what makes the compiler "pluggable."
  #
  # - `scope` — tracks local variables within the current scope. Used for
  #   functions that have their own local variable space.

  defstruct instructions: [],
            constants: [],
            names: [],
            dispatch: %{},
            scope: nil

  # ---------------------------------------------------------------------------
  # CompilerScope — Local Variable Tracking
  # ---------------------------------------------------------------------------
  #
  # When compiling a function body, we need to track which local variables
  # exist and what slot index each one occupies. A `CompilerScope` is like
  # a page in a notebook: each variable gets the next available line number.
  #
  # Scopes can be nested (a function inside a function), so each scope has
  # a `parent` pointer back to the enclosing scope.
  #
  # Example:
  #
  #     scope = %CompilerScope{locals: %{"x" => 0, "y" => 1}, parent: nil}
  #
  # Here, `x` is in slot 0 and `y` is in slot 1.

  defmodule CompilerScope do
    @moduledoc """
    Tracks local variables within a compilation scope.

    Each scope maintains a map of variable names to slot indices.
    Scopes can be nested via the `parent` field, enabling lexical
    scoping for nested function definitions.
    """

    defstruct locals: %{}, parent: nil

    @doc """
    Add a local variable to the scope.

    If the variable already exists, returns its existing index (no
    duplicate slots). If it's new, assigns the next available index.

    Returns `{index, updated_scope}`.

    ## Example

        iex> scope = %CompilerScope{}
        iex> {0, scope} = CompilerScope.add_local(scope, "x")
        iex> {1, scope} = CompilerScope.add_local(scope, "y")
        iex> {0, _scope} = CompilerScope.add_local(scope, "x")  # already exists
    """
    def add_local(%__MODULE__{} = scope, name) do
      case Map.get(scope.locals, name) do
        nil ->
          index = map_size(scope.locals)
          {index, %{scope | locals: Map.put(scope.locals, name, index)}}

        existing ->
          {existing, scope}
      end
    end

    @doc """
    Look up a local variable's slot index.

    Returns the index if found, or `nil` if the variable isn't in this scope.
    """
    def get_local(%__MODULE__{} = scope, name) do
      Map.get(scope.locals, name)
    end

    @doc """
    Returns how many local variables are in this scope.
    """
    def num_locals(%__MODULE__{} = scope) do
      map_size(scope.locals)
    end
  end

  # ---------------------------------------------------------------------------
  # Error Types
  # ---------------------------------------------------------------------------
  #
  # Two custom exceptions help users understand what went wrong:
  #
  # - `CompilerError` — general compilation failures (e.g., exiting a scope
  #   when we're not in one).
  #
  # - `UnhandledRuleError` — raised when an AST node's rule_name has no
  #   registered handler AND the node has multiple children (so we can't
  #   just pass through).

  defmodule CompilerError do
    @moduledoc "Raised for general compilation errors."
    defexception [:message]
  end

  defmodule UnhandledRuleError do
    @moduledoc "Raised when an AST node has no registered handler and cannot pass through."
    defexception [:message]
  end

  # ---------------------------------------------------------------------------
  # Constructor
  # ---------------------------------------------------------------------------

  @doc """
  Create a fresh compiler with empty state.

  This is your starting point. From here, register rules, then compile.

  ## Example

      compiler = GenericCompiler.new()
  """
  def new, do: %__MODULE__{}

  # ---------------------------------------------------------------------------
  # Plugin Registration
  # ---------------------------------------------------------------------------
  #
  # The `register_rule/3` function is how you teach the compiler about your
  # language. Each call adds one entry to the dispatch map.
  #
  # The handler must be a function of arity 2: fn(compiler, node) -> compiler.
  # It receives the current compiler state and the AST node that matched,
  # and must return the updated compiler state.

  @doc """
  Register a handler function for a given AST rule name.

  The handler will be called whenever `compile_node/2` encounters a node
  whose `rule_name` matches the given string.

  ## Parameters

  - `compiler` — the current compiler state
  - `rule_name` — a string like `"number"` or `"binary_op"`
  - `handler` — a function `fn(compiler, node) -> compiler`

  ## Example

      compiler = GenericCompiler.register_rule(compiler, "number", fn compiler, node ->
        token = hd(node.children)
        value = String.to_integer(token.value)
        {index, compiler} = GenericCompiler.add_constant(compiler, value)
        {_idx, compiler} = GenericCompiler.emit(compiler, 0x01, index)
        compiler
      end)
  """
  def register_rule(%__MODULE__{} = compiler, rule_name, handler)
      when is_binary(rule_name) and is_function(handler, 2) do
    %{compiler | dispatch: Map.put(compiler.dispatch, rule_name, handler)}
  end

  # ---------------------------------------------------------------------------
  # Instruction Emission
  # ---------------------------------------------------------------------------
  #
  # These functions build the instruction list. Each `emit` call creates an
  # `Instruction` struct and appends it to the list.
  #
  # `emit/2` and `emit/3` return `{index, compiler}` where `index` is the
  # position of the newly emitted instruction. This is useful for jump
  # patching — you emit a jump with a placeholder target, remember the
  # index, and later patch it with the real target.

  @doc """
  Emit a bytecode instruction.

  Returns `{index, compiler}` where `index` is the position of the
  emitted instruction in the instruction list.

  ## Parameters

  - `opcode` — the instruction opcode (an integer)
  - `operand` — optional operand (defaults to `nil`)

  ## Example

      # Emit LOAD_CONST with operand 0
      {idx, compiler} = GenericCompiler.emit(compiler, 0x01, 0)

      # Emit ADD with no operand
      {idx, compiler} = GenericCompiler.emit(compiler, 0x10)
  """
  def emit(%__MODULE__{} = compiler, opcode, operand \\ nil) do
    instr = %Instruction{opcode: opcode, operand: operand}
    index = length(compiler.instructions)
    compiler = %{compiler | instructions: compiler.instructions ++ [instr]}
    {index, compiler}
  end

  @doc """
  Emit a jump instruction with a placeholder target of 0.

  Returns `{index, compiler}` — save the index so you can call
  `patch_jump/3` later to fill in the real target.

  ## How Jump Patching Works

  1. Emit the jump: `{jump_idx, compiler} = emit_jump(compiler, 0x20)`
  2. Compile the body (more instructions get emitted)
  3. Patch the target: `compiler = patch_jump(compiler, jump_idx)`

  The jump now points to the instruction *after* the body.
  """
  def emit_jump(%__MODULE__{} = compiler, opcode) do
    emit(compiler, opcode, 0)
  end

  @doc """
  Patch a previously emitted jump instruction with a target address.

  If `target` is not provided, defaults to `current_offset/1` — i.e.,
  the jump will land on the *next* instruction to be emitted.

  ## Parameters

  - `index` — the position of the jump instruction to patch
  - `target` — the target instruction index (defaults to current offset)
  """
  def patch_jump(%__MODULE__{} = compiler, index, target \\ nil) do
    target = target || current_offset(compiler)
    old_instr = Enum.at(compiler.instructions, index)
    new_instr = %Instruction{opcode: old_instr.opcode, operand: target}
    instructions = List.replace_at(compiler.instructions, index, new_instr)
    %{compiler | instructions: instructions}
  end

  @doc """
  Returns the current instruction count.

  This is the index where the *next* emitted instruction will land.
  Useful for jump targets: "jump to whatever comes next."
  """
  def current_offset(%__MODULE__{} = compiler) do
    length(compiler.instructions)
  end

  # ---------------------------------------------------------------------------
  # Constant and Name Pools
  # ---------------------------------------------------------------------------
  #
  # The constant pool stores literal values (numbers, strings, etc.) that
  # appear in the source code. Instead of embedding `42` directly in an
  # instruction, we store it in the pool and reference it by index.
  #
  # Why? Because instructions have a fixed format (opcode + operand), and
  # operands are always integers. By using pool indices, we can reference
  # values of any type.
  #
  # The name pool works the same way but for identifiers (variable names,
  # function names). `STORE_NAME 0` means "store into the variable whose
  # name is at index 0 in the name pool."
  #
  # Both pools deduplicate: adding the same value twice returns the same
  # index, keeping the pool compact.

  @doc """
  Add a value to the constant pool.

  Returns `{index, compiler}`. If the value already exists in the pool
  (checked by strict equality `===`), returns the existing index.

  ## Example

      {0, compiler} = GenericCompiler.add_constant(compiler, 42)
      {0, compiler} = GenericCompiler.add_constant(compiler, 42)  # same index
      {1, compiler} = GenericCompiler.add_constant(compiler, 99)  # new index
  """
  def add_constant(%__MODULE__{} = compiler, value) do
    case Enum.find_index(compiler.constants, fn c -> c === value end) do
      nil ->
        index = length(compiler.constants)
        {index, %{compiler | constants: compiler.constants ++ [value]}}

      existing_index ->
        {existing_index, compiler}
    end
  end

  @doc """
  Add a name to the name pool.

  Returns `{index, compiler}`. Deduplicates just like `add_constant/2`.

  ## Example

      {0, compiler} = GenericCompiler.add_name(compiler, "x")
      {0, compiler} = GenericCompiler.add_name(compiler, "x")  # same index
      {1, compiler} = GenericCompiler.add_name(compiler, "y")  # new index
  """
  def add_name(%__MODULE__{} = compiler, name) when is_binary(name) do
    case Enum.find_index(compiler.names, fn n -> n == name end) do
      nil ->
        index = length(compiler.names)
        {index, %{compiler | names: compiler.names ++ [name]}}

      existing_index ->
        {existing_index, compiler}
    end
  end

  # ---------------------------------------------------------------------------
  # Scope Management
  # ---------------------------------------------------------------------------
  #
  # Scopes track local variables. When compiling a function body, we enter
  # a new scope. Parameters become the first local variables. When the
  # function body is done, we exit the scope, returning to the parent.
  #
  # Example for a function `def add(a, b)`:
  #
  #     {scope, compiler} = enter_scope(compiler, ["a", "b"])
  #     # scope.locals = %{"a" => 0, "b" => 1}
  #     # ... compile the function body ...
  #     {scope, compiler} = exit_scope(compiler)
  #     # back to parent scope

  @doc """
  Enter a new scope, optionally pre-populating with parameter names.

  Returns `{scope, compiler}` where `scope` is the newly created
  `CompilerScope` struct.

  ## Parameters

  - `params` — a list of parameter name strings (default: `[]`)
  """
  def enter_scope(%__MODULE__{} = compiler, params \\ []) do
    scope = %CompilerScope{parent: compiler.scope}

    scope =
      Enum.reduce(params, scope, fn name, acc ->
        {_index, acc} = CompilerScope.add_local(acc, name)
        acc
      end)

    {scope, %{compiler | scope: scope}}
  end

  @doc """
  Exit the current scope, returning to the parent.

  Returns `{exited_scope, compiler}` where `exited_scope` is the scope
  we just left (useful for inspecting how many locals were declared).

  Raises `CompilerError` if there's no scope to exit.
  """
  def exit_scope(%__MODULE__{scope: nil}) do
    raise CompilerError, message: "Cannot exit scope — not in any scope"
  end

  def exit_scope(%__MODULE__{scope: scope} = compiler) do
    {scope, %{compiler | scope: scope.parent}}
  end

  # ---------------------------------------------------------------------------
  # Nested Compilation
  # ---------------------------------------------------------------------------
  #
  # Sometimes we need to compile a sub-tree into its own independent
  # CodeObject — for example, a function body. The function's bytecode
  # is separate from the module's bytecode.
  #
  # `compile_nested/2` saves the current instruction/constant/name state,
  # compiles the sub-tree in a fresh context, packages the result as a
  # CodeObject, and restores the original state.

  @doc """
  Compile a sub-tree into a standalone CodeObject.

  This is used for compiling function bodies. The sub-tree gets its own
  instruction list, constant pool, and name pool. The parent compiler's
  state is preserved.

  Returns `{code_object, compiler}`.
  """
  def compile_nested(%__MODULE__{} = compiler, node) do
    # Save current compilation state
    saved = {compiler.instructions, compiler.constants, compiler.names}

    # Start fresh for the nested compilation
    compiler = %{compiler | instructions: [], constants: [], names: []}

    # Compile the sub-tree
    compiler = compile_node(compiler, node)

    # Package the result
    nested = %CodeObject{
      instructions: compiler.instructions,
      constants: compiler.constants,
      names: compiler.names
    }

    # Restore the parent's state
    {saved_instructions, saved_constants, saved_names} = saved

    compiler = %{
      compiler
      | instructions: saved_instructions,
        constants: saved_constants,
        names: saved_names
    }

    {nested, compiler}
  end

  # ---------------------------------------------------------------------------
  # AST Dispatch — The Core Algorithm
  # ---------------------------------------------------------------------------
  #
  # `compile_node/2` is where the magic happens. Given an AST node, it:
  #
  # 1. Checks if it's a token (leaf node) — if so, it's a no-op by default.
  #    Structural tokens like NEWLINE or INDENT don't produce bytecode.
  #
  # 2. Checks if a handler is registered for the node's rule_name.
  #    If yes, calls the handler.
  #
  # 3. If no handler and the node has exactly ONE child, passes through
  #    to that child. This is essential for precedence-encoding rules:
  #
  #        expr -> term -> factor -> number
  #
  #    If only `number` has a handler, `expr`, `term`, and `factor` all
  #    pass through automatically.
  #
  # 4. If no handler and multiple children, raises UnhandledRuleError.
  #    We can't guess which child to compile or in what order.

  @doc """
  Compile a single AST node.

  Dispatches to the registered handler for the node's rule_name, or
  passes through if the node has exactly one child and no handler.

  Raises `UnhandledRuleError` for multi-child nodes without handlers.
  """
  def compile_node(%__MODULE__{} = compiler, %{type: _} = token) do
    # Token nodes (leaves) are no-ops by default.
    # Structural tokens like NEWLINE don't produce bytecode.
    compile_token(compiler, token)
  end

  def compile_node(%__MODULE__{} = compiler, %{rule_name: rule_name, children: children} = node) do
    handler = Map.get(compiler.dispatch, rule_name)

    cond do
      handler != nil ->
        # Found a registered handler — let it do its thing
        handler.(compiler, node)

      length(children) == 1 ->
        # Pass-through: single-child node with no handler.
        # This is how precedence-encoding rules work — `expr -> term`
        # just forwards to `term` without needing its own handler.
        compile_node(compiler, hd(children))

      true ->
        # Multiple children, no handler — we don't know what to do.
        raise UnhandledRuleError,
          message:
            "No handler registered for rule '#{rule_name}' " <>
              "and it has #{length(children)} children (not a pass-through). " <>
              "Register a handler with register_rule('#{rule_name}', handler)."
    end
  end

  @doc """
  Compile a token node. Default implementation is a no-op.

  Override this in language-specific compilers if tokens need special
  handling.
  """
  def compile_token(%__MODULE__{} = compiler, _token) do
    compiler
  end

  # ---------------------------------------------------------------------------
  # Top-Level Compile
  # ---------------------------------------------------------------------------
  #
  # `compile/3` is the entry point for compiling a complete AST. It:
  #
  # 1. Walks the AST via `compile_node/2`
  # 2. Emits a HALT instruction at the end (so the VM knows to stop)
  # 3. Packages everything into a `CodeObject`
  #
  # The `halt_opcode` parameter lets different VMs use different opcodes
  # for their halt instruction (default is 0xFF).

  @doc """
  Compile a complete AST into a CodeObject.

  Walks the entire tree, emitting instructions, then appends a HALT
  instruction. Returns `{code_object, compiler}`.

  ## Parameters

  - `ast` — the root AST node to compile
  - `halt_opcode` — the opcode to use for HALT (default: `0xFF`)

  ## Example

      {code_object, _compiler} = GenericCompiler.compile(compiler, ast)
      # code_object.instructions ends with %Instruction{opcode: 0xFF}
  """
  def compile(%__MODULE__{} = compiler, ast, halt_opcode \\ 0xFF) do
    compiler = compile_node(compiler, ast)
    {_index, compiler} = emit(compiler, halt_opcode)

    code = %CodeObject{
      instructions: compiler.instructions,
      constants: compiler.constants,
      names: compiler.names
    }

    {code, compiler}
  end
end
