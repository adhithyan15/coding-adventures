defmodule CodingAdventures.StarlarkInterpreter.Interpreter do
  @moduledoc """
  Starlark Interpreter — The complete execution pipeline with load() support.

  ## Chapter 1: The Pipeline

  This module chains together all four stages of Starlark execution:

      source code → tokens → AST → bytecode → execution

  Each stage is handled by a different package:

  1. **Tokenizer + Parser** — built into the compiler package's `compile_starlark/1`
  2. **Compiler** — `starlark_ast_to_bytecode_compiler` translates AST to bytecode
  3. **VM** — `starlark_vm` executes bytecode on a stack machine

  This module adds the critical `load()` function that makes BUILD files work.

  ## Chapter 2: How load() Works

  When a BUILD file contains:

      load("//rules/python.star", "py_library")

  The compiler translates this into two opcodes:

      LOAD_MODULE 0    # names[0] = "//rules/python.star"
      IMPORT_FROM 1    # names[1] = "py_library"
      STORE_NAME  1    # Store "py_library" in current scope

  The default `LOAD_MODULE` handler in the VM is a stub that pushes an empty
  dict. This module **overrides** it with a real handler that:

  1. Resolves the file path using a configurable file resolver
  2. Recursively interprets the loaded file through the same pipeline
  3. Caches the result so each file is evaluated at most once
  4. Pushes the loaded file's variables as a dict onto the stack

  ## Chapter 3: File Resolvers

  The interpreter doesn't know where files live on disk. Instead, it accepts
  a **file resolver** — either a map (for testing) or a function (for production):

      # Map resolver (for tests)
      resolver = %{"//rules/test.star" => "def foo():\\n    return 42\\n"}

      # Function resolver (for production)
      resolver = fn label ->
        path = String.replace(label, "//", "/path/to/repo/")
        File.read!(path)
      end

  ## Chapter 4: Caching

  Loaded files are cached so each file is evaluated at most once. This matches
  Bazel's semantics where loaded files are frozen after first evaluation.
  The cache maps labels to their exported variables:

      %{"//rules/python.star" => %{"py_library" => <function>, ...}}

  Subsequent `load()` calls for the same file return cached symbols instantly.
  """

  alias CodingAdventures.VirtualMachine.GenericVM
  alias CodingAdventures.StarlarkAstToBytecodeCompiler.Compiler
  alias CodingAdventures.StarlarkAstToBytecodeCompiler.Opcodes, as: Op
  alias CodingAdventures.StarlarkVm.Vm, as: StarlarkVmFactory
  alias CodingAdventures.StarlarkVm.Handlers

  # ===========================================================================
  # File Resolution
  # ===========================================================================

  @doc """
  Resolve a label to file contents using the configured resolver.

  Supports two forms:
  - A map (`%{label => content}`) for testing
  - A function (`fn label -> content end`) for production

  Raises `RuntimeError` if the label cannot be resolved.

  ## Examples

      iex> resolve_file(%{"//a.star" => "x = 1\\n"}, "//a.star")
      "x = 1\\n"

      iex> resolve_file(nil, "//a.star")
      ** (RuntimeError) load() called but no file_resolver configured
  """
  def resolve_file(nil, label) do
    raise "load() called but no file_resolver configured. Cannot resolve: #{label}"
  end

  def resolve_file(resolver, label) when is_map(resolver) do
    case Map.fetch(resolver, label) do
      {:ok, contents} -> contents
      :error -> raise "load(): file not found in resolver: #{label}"
    end
  end

  def resolve_file(resolver, label) when is_function(resolver, 1) do
    resolver.(label)
  end

  # ===========================================================================
  # interpret/2 — The Main Entry Point
  # ===========================================================================

  @doc """
  Execute Starlark source code and return the result.

  This is the simplest API — one function call does everything:

  1. Compiles the source to bytecode (tokenize → parse → compile)
  2. Creates a fresh VM with all 46 opcodes and 23 builtins registered
  3. Overrides the LOAD_MODULE handler with file-resolving logic
  4. Executes the bytecode
  5. Returns a `%StarlarkResult{}` with variables, output, and traces

  ## Options

  - `:file_resolver` — How to resolve `load()` paths. Can be a map
    (`%{label => content}`) or a function (`fn label -> content end`).
    Default: `nil` (load() calls will raise).
  - `:max_recursion_depth` — Maximum call stack depth. Default: 200.
  - `:load_cache` — Pre-populated cache of already-loaded files. Default: `%{}`.
    Useful for sharing a cache across multiple `interpret` calls.

  ## Examples

      # Simple execution
      result = interpret("x = 1 + 2\\nprint(x)\\n")
      result.variables["x"]  #=> 3
      result.output           #=> ["3"]

      # With load()
      files = %{
        "//rules/math.star" => "def double(n):\\n    return n * 2\\n"
      }
      result = interpret(
        "load(\\"//rules/math.star\\", \\"double\\")\\nresult = double(21)\\n",
        file_resolver: files
      )
      result.variables["result"]  #=> 42
  """
  def interpret(source, opts \\ []) when is_binary(source) do
    file_resolver = Keyword.get(opts, :file_resolver, nil)
    max_depth = Keyword.get(opts, :max_recursion_depth, 200)
    load_cache = Keyword.get(opts, :load_cache, %{})

    # -----------------------------------------------------------------------
    # Step 1: Compile source to bytecode
    # -----------------------------------------------------------------------
    # The compiler handles tokenization, parsing, and code generation in one
    # call. It returns a CodeObject with instructions, constants, and names.
    code = Compiler.compile_starlark(source)

    # -----------------------------------------------------------------------
    # Step 2: Create a VM with load() support
    # -----------------------------------------------------------------------
    vm = StarlarkVmFactory.create_starlark_vm(max_recursion_depth: max_depth)

    # Override LOAD_MODULE with our file-resolving handler
    vm = register_load_handler(vm, file_resolver, load_cache, opts)

    # -----------------------------------------------------------------------
    # Step 3: Execute the bytecode
    # -----------------------------------------------------------------------
    {traces, vm} = GenericVM.execute(vm, code)

    # -----------------------------------------------------------------------
    # Step 4: Package up the result
    # -----------------------------------------------------------------------
    %Handlers.StarlarkResult{
      variables: vm.variables,
      output: vm.output,
      traces: traces
    }
  end

  # ===========================================================================
  # interpret_file/2 — Execute a Starlark File
  # ===========================================================================

  @doc """
  Execute a Starlark file by reading it from the filesystem.

  Reads the file at `path`, ensures it ends with a newline (parser requirement),
  then delegates to `interpret/2`.

  ## Options

  Same as `interpret/2`.

  ## Examples

      result = interpret_file("path/to/program.star")
      result.variables["name"]  #=> "mylib"
  """
  def interpret_file(path, opts \\ []) when is_binary(path) do
    source = File.read!(path)

    # Ensure source ends with newline (parser requirement)
    source = if String.ends_with?(source, "\n") do
      source
    else
      source <> "\n"
    end

    interpret(source, opts)
  end

  # ===========================================================================
  # LOAD_MODULE Handler
  # ===========================================================================

  @doc false
  defp register_load_handler(vm, file_resolver, load_cache, opts) do
    # We use an Agent to hold the mutable cache state. In Elixir, the VM state
    # is immutable — it's threaded through handlers. But the load cache needs
    # to persist across recursive interpret() calls. We use a simple map ref
    # stored in the VM's extras.

    # Store cache and resolver in VM extras so the handler can access them
    vm = GenericVM.put_extra(vm, :load_cache, load_cache)
    vm = GenericVM.put_extra(vm, :file_resolver, file_resolver)
    vm = GenericVM.put_extra(vm, :interpreter_opts, opts)

    # Override LOAD_MODULE with our implementation
    handler = fn vm_state, instr, code_obj ->
      handle_load_module(vm_state, instr, code_obj)
    end

    GenericVM.register_opcode(vm, Op.load_module(), handler)
  end

  @doc false
  defp handle_load_module(vm, instr, code_obj) do
    # -----------------------------------------------------------------------
    # How This Works
    # -----------------------------------------------------------------------
    #
    # The compiler compiles `load("//rules/python.star", "sym")` into:
    #
    #     LOAD_MODULE 0    # names[0] = "//rules/python.star"
    #     DUP              # Keep module on stack for multiple imports
    #     IMPORT_FROM 1    # names[1] = "sym"
    #     STORE_NAME 1     # Store as "sym"
    #     POP              # Remove module dict from stack
    #
    # This handler:
    # 1. Reads the module label from the names pool
    # 2. Checks the load cache
    # 3. If not cached, resolves the file and recursively interprets it
    # 4. Pushes the module's variables as a dict onto the stack

    index = instr.operand
    module_label = Enum.at(code_obj.names, index)

    load_cache = GenericVM.get_extra(vm, :load_cache, %{})
    file_resolver = GenericVM.get_extra(vm, :file_resolver, nil)
    interp_opts = GenericVM.get_extra(vm, :interpreter_opts, [])

    # Check cache first — each file is evaluated at most once
    {module_vars, updated_cache} = if Map.has_key?(load_cache, module_label) do
      {Map.get(load_cache, module_label), load_cache}
    else
      # Resolve and execute the file
      contents = resolve_file(file_resolver, module_label)

      # Ensure trailing newline
      contents = if String.ends_with?(contents, "\n") do
        contents
      else
        contents <> "\n"
      end

      # Recursively interpret the loaded file
      # Pass through the same resolver and cache so transitive loads work
      result = interpret(contents, Keyword.merge(interp_opts, [
        file_resolver: file_resolver,
        load_cache: load_cache
      ]))

      vars = result.variables
      {vars, Map.put(load_cache, module_label, vars)}
    end

    # Update the cache in VM extras
    vm = GenericVM.put_extra(vm, :load_cache, updated_cache)

    # Push the module's exported variables as a dict onto the stack
    vm = GenericVM.push(vm, module_vars)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end
end
