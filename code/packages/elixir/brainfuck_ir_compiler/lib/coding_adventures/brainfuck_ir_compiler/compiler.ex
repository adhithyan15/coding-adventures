defmodule CodingAdventures.BrainfuckIrCompiler.CompileResult do
  @moduledoc """
  The output of a successful Brainfuck compilation.

  - `program`    — the compiled `IrProgram` (linear IR instruction sequence)
  - `source_map` — the `SourceMapChain` sidecar (segments 1 and 2 filled)
  """

  alias CodingAdventures.CompilerIr.IrProgram
  alias CodingAdventures.CompilerSourceMap.SourceMapChain

  defstruct [:program, :source_map]

  @type t :: %__MODULE__{
          program: IrProgram.t(),
          source_map: SourceMapChain.t()
        }
end

defmodule CodingAdventures.BrainfuckIrCompiler.Compiler do
  @moduledoc """
  Brainfuck AOT Compiler — translates a Brainfuck AST into IR.

  This is the Brainfuck-specific **frontend** of the AOT compiler pipeline.
  It knows Brainfuck semantics (tape, cells, pointer, loops, I/O) and
  translates them into target-independent IR instructions. It does NOT
  know about RISC-V, ARM, ELF, or any specific machine target.

  The compiler produces two outputs:

  1. An `IrProgram` containing the compiled IR instructions.
  2. A `SourceMapChain` with segments 1 (SourceToAst) and 2 (AstToIr).

  ## Register allocation

  Brainfuck needs very few registers:

  | Virtual | Role                                   |
  |---------|----------------------------------------|
  | v0      | tape base address (`&tape`)            |
  | v1      | tape pointer offset (0-based index)    |
  | v2      | temporary — cell value for loads/stores|
  | v3      | temporary — bounds check comparison    |
  | v4      | syscall argument register              |
  | v5      | max pointer = `tape_size - 1`          |
  | v6      | zero constant (for bounds checks)      |

  This fixed allocation maps directly to a small set of physical registers
  in any ISA. Future languages (BASIC) with more complexity will use a
  register allocator in the backend.

  ## Syscall numbers

  | Number | Operation                            |
  |--------|--------------------------------------|
  | 1      | write byte (from v4) to stdout       |
  | 2      | read byte from stdin into v4         |
  | 10     | exit (halt with code in v4)          |

  ## Brainfuck → IR mapping

  ```
  >  →  ADD_IMM v1, v1, 1
  <  →  ADD_IMM v1, v1, -1
  +  →  LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, 1; AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1
  -  →  LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, -1; AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1
  .  →  LOAD_BYTE v2, v0, v1; ADD_IMM v4, v2, 0; SYSCALL 1
  ,  →  SYSCALL 2; STORE_BYTE v4, v0, v1
  [  →  LABEL loop_N_start; LOAD_BYTE v2, v0, v1; BRANCH_Z v2, loop_N_end
  ]  →  JUMP loop_N_start; LABEL loop_N_end
  ```
  """

  alias CodingAdventures.CompilerIr.{
    IrProgram,
    IrInstruction,
    IrDataDecl,
    IrRegister,
    IrImmediate,
    IrLabel,
    IDGenerator
  }

  alias CodingAdventures.CompilerSourceMap.{
    SourceMapChain,
    SourcePosition,
    SourceToAst,
    AstToIr
  }

  alias CodingAdventures.BrainfuckIrCompiler.{BuildConfig, CompileResult}
  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.Lexer.Token

  # ── Virtual register indices ─────────────────────────────────────────────────
  @reg_tape_base 0  # v0: base address of the tape
  @reg_tape_ptr  1  # v1: current cell offset (0-based index)
  @reg_temp      2  # v2: temporary for cell values
  @reg_temp2     3  # v3: temporary for bounds checks
  @reg_sys_arg   4  # v4: syscall argument
  @reg_max_ptr   5  # v5: tape_size - 1 (for bounds checks)
  @reg_zero      6  # v6: constant 0 (for bounds checks)

  # ── Syscall numbers ──────────────────────────────────────────────────────────
  @syscall_write 1   # write byte in v4 to stdout
  @syscall_read  2   # read byte from stdin into v4
  @syscall_exit  10  # halt with exit code in v4

  # ── Internal compiler state ──────────────────────────────────────────────────
  #
  # The state is threaded through all compile functions as a plain map.
  # Elixir's immutability makes this explicit and easy to reason about.

  defstruct [
    :config,
    :filename,
    :id_gen,
    :node_id_gen,
    :program,
    :source_map,
    :loop_count
  ]

  @type state :: %__MODULE__{
          config: BuildConfig.t(),
          filename: String.t(),
          id_gen: IDGenerator.t(),
          node_id_gen: non_neg_integer(),
          program: IrProgram.t(),
          source_map: SourceMapChain.t(),
          loop_count: non_neg_integer()
        }

  @doc """
  Compile a Brainfuck AST into an `IrProgram` and `SourceMapChain`.

  The `ast` must be the root `ASTNode` with `rule_name: "program"` as
  produced by `CodingAdventures.Brainfuck.Parser.parse/1`.

  The `filename` is used in source map entries to identify the source file
  (e.g. `"hello.bf"`).

  ## Examples

      {:ok, ast} = CodingAdventures.Brainfuck.parse("+.")
      {:ok, result} = Compiler.compile(ast, "hello.bf", BuildConfig.release_config())
      result.program.entry_label  # => "_start"

  ## Errors

  Returns `{:error, message}` if:
  - The AST root is not a `"program"` node.
  - The `tape_size` in the config is <= 0.
  - An unexpected AST node type is encountered.
  """
  @spec compile(ASTNode.t(), String.t(), BuildConfig.t()) ::
          {:ok, CompileResult.t()} | {:error, String.t()}
  def compile(%ASTNode{rule_name: rule_name} = ast, filename, %BuildConfig{} = config) do
    cond do
      rule_name != "program" ->
        {:error, "expected 'program' AST node, got #{inspect(rule_name)}"}

      config.tape_size <= 0 ->
        {:error, "invalid tape_size #{config.tape_size}: must be positive"}

      true ->
        state = %__MODULE__{
          config: config,
          filename: filename,
          id_gen: IDGenerator.new(),
          node_id_gen: 0,
          program: IrProgram.new("_start"),
          source_map: SourceMapChain.new(),
          loop_count: 0
        }

        # Add tape data declaration
        state = update_program(state, fn p ->
          IrProgram.add_data(p, %IrDataDecl{
            label: "tape",
            size: config.tape_size,
            init: 0
          })
        end)

        # Emit prologue
        state = emit_prologue(state)

        # Compile the program body
        case compile_program(state, ast) do
          {:ok, state} ->
            # Emit epilogue
            state = emit_epilogue(state)
            {:ok, %CompileResult{program: state.program, source_map: state.source_map}}

          {:error, _} = err ->
            err
        end
    end
  end

  # ── Internal helpers ─────────────────────────────────────────────────────────

  # Update the IrProgram inside the compiler state.
  defp update_program(state, fun) do
    %{state | program: fun.(state.program)}
  end

  # Update the SourceMapChain inside the compiler state.
  defp update_source_map(state, fun) do
    %{state | source_map: fun.(state.source_map)}
  end

  # Emit an instruction; returns {id, new_state}.
  defp emit(state, opcode, operands \\ []) do
    {id, gen} = IDGenerator.next(state.id_gen)

    instr = %IrInstruction{
      opcode: opcode,
      operands: operands,
      id: id
    }

    state = %{state | id_gen: gen}
    state = update_program(state, fn p -> IrProgram.add_instruction(p, instr) end)
    {id, state}
  end

  # Emit a label (no ID, produces no machine code).
  defp emit_label(state, name) do
    instr = %IrInstruction{
      opcode: :label,
      operands: [%IrLabel{name: name}],
      id: -1
    }

    update_program(state, fn p -> IrProgram.add_instruction(p, instr) end)
  end

  # Consume the next node ID (for source mapping).
  defp next_node_id(state) do
    {state.node_id_gen, %{state | node_id_gen: state.node_id_gen + 1}}
  end

  # ── Prologue ─────────────────────────────────────────────────────────────────
  #
  # Sets up the execution environment:
  #   - LABEL _start
  #   - LOAD_ADDR v0, tape   (tape base address)
  #   - LOAD_IMM  v1, 0      (tape pointer = 0)
  #   - [debug] LOAD_IMM v5, tape_size-1  (max valid pointer)
  #   - [debug] LOAD_IMM v6, 0            (lower bound constant)

  defp emit_prologue(state) do
    state = emit_label(state, "_start")

    # v0 = &tape
    {_, state} =
      emit(state, :load_addr, [
        %IrRegister{index: @reg_tape_base},
        %IrLabel{name: "tape"}
      ])

    # v1 = 0
    {_, state} =
      emit(state, :load_imm, [
        %IrRegister{index: @reg_tape_ptr},
        %IrImmediate{value: 0}
      ])

    # Debug: set up bounds check registers
    if state.config.insert_bounds_checks do
      # v5 = tape_size - 1
      {_, state} =
        emit(state, :load_imm, [
          %IrRegister{index: @reg_max_ptr},
          %IrImmediate{value: state.config.tape_size - 1}
        ])

      # v6 = 0
      {_, state} =
        emit(state, :load_imm, [
          %IrRegister{index: @reg_zero},
          %IrImmediate{value: 0}
        ])

      state
    else
      state
    end
  end

  # ── Epilogue ─────────────────────────────────────────────────────────────────
  #
  # Terminates the program:
  #   - HALT
  #   - [debug] __trap_oob handler

  defp emit_epilogue(state) do
    {_, state} = emit(state, :halt)

    if state.config.insert_bounds_checks do
      state = emit_label(state, "__trap_oob")

      # Load error exit code
      {_, state} =
        emit(state, :load_imm, [
          %IrRegister{index: @reg_sys_arg},
          %IrImmediate{value: 1}
        ])

      # Exit with error code 1
      {_, state} = emit(state, :syscall, [%IrImmediate{value: @syscall_exit}])
      state
    else
      state
    end
  end

  # ── AST walking ──────────────────────────────────────────────────────────────

  defp compile_program(state, %ASTNode{children: children}) do
    Enum.reduce_while(children, {:ok, state}, fn child, {:ok, acc_state} ->
      case child do
        %ASTNode{} ->
          case compile_node(acc_state, child) do
            {:ok, new_state} -> {:cont, {:ok, new_state}}
            {:error, _} = err -> {:halt, err}
          end

        _ ->
          # Skip tokens at program level
          {:cont, {:ok, acc_state}}
      end
    end)
  end

  defp compile_node(state, %ASTNode{rule_name: rule_name} = node) do
    case rule_name do
      "instruction" ->
        # Instruction wraps either a loop or a command
        Enum.reduce_while(node.children, {:ok, state}, fn child, {:ok, acc_state} ->
          case child do
            %ASTNode{} ->
              case compile_node(acc_state, child) do
                {:ok, ns} -> {:cont, {:ok, ns}}
                err -> {:halt, err}
              end

            _ ->
              {:cont, {:ok, acc_state}}
          end
        end)

      "command" ->
        compile_command(state, node)

      "loop" ->
        compile_loop(state, node)

      other ->
        {:error, "unexpected AST node type: #{inspect(other)}"}
    end
  end

  # ── Command compilation ───────────────────────────────────────────────────────
  #
  # Each Brainfuck command maps to a specific sequence of IR instructions.
  # See the module doc for the full mapping table.

  defp compile_command(state, %ASTNode{} = node) do
    tok = extract_token(node)

    if tok == nil do
      {:error, "command node has no token"}
    else
      {ast_node_id, state} = next_node_id(state)

      # Record source → AST mapping
      state =
        update_source_map(state, fn sm ->
          pos = %SourcePosition{
            file: state.filename,
            line: tok.line,
            column: tok.column,
            length: 1
          }

          %{sm | source_to_ast: SourceToAst.add(sm.source_to_ast, pos, ast_node_id)}
        end)

      # Compile and collect IR IDs
      case tok.value do
        ">" ->
          {ir_ids, state} = compile_right(state)
          state = record_ast_to_ir(state, ast_node_id, ir_ids)
          {:ok, state}

        "<" ->
          {ir_ids, state} = compile_left(state)
          state = record_ast_to_ir(state, ast_node_id, ir_ids)
          {:ok, state}

        "+" ->
          {ir_ids, state} = emit_cell_mutation(state, 1)
          state = record_ast_to_ir(state, ast_node_id, ir_ids)
          {:ok, state}

        "-" ->
          {ir_ids, state} = emit_cell_mutation(state, -1)
          state = record_ast_to_ir(state, ast_node_id, ir_ids)
          {:ok, state}

        "." ->
          {ir_ids, state} = compile_output(state)
          state = record_ast_to_ir(state, ast_node_id, ir_ids)
          {:ok, state}

        "," ->
          {ir_ids, state} = compile_input(state)
          state = record_ast_to_ir(state, ast_node_id, ir_ids)
          {:ok, state}

        other ->
          {:error, "unknown command token: #{inspect(other)}"}
      end
    end
  end

  # Helper: record AST node → IR IDs in the source map.
  defp record_ast_to_ir(state, ast_node_id, ir_ids) do
    update_source_map(state, fn sm ->
      %{sm | ast_to_ir: AstToIr.add(sm.ast_to_ir, ast_node_id, ir_ids)}
    end)
  end

  # ── RIGHT: move tape pointer right ──────────────────────────────────────────
  #
  # Release: ADD_IMM v1, v1, 1
  # Debug:   CMP_GT v3, v1, v5 ; BRANCH_NZ v3, __trap_oob ; ADD_IMM v1, v1, 1

  defp compile_right(state) do
    {ids, state} =
      if state.config.insert_bounds_checks do
        emit_bounds_check_right(state)
      else
        {[], state}
      end

    {id, state} =
      emit(state, :add_imm, [
        %IrRegister{index: @reg_tape_ptr},
        %IrRegister{index: @reg_tape_ptr},
        %IrImmediate{value: 1}
      ])

    {ids ++ [id], state}
  end

  # ── LEFT: move tape pointer left ─────────────────────────────────────────────
  #
  # Release: ADD_IMM v1, v1, -1
  # Debug:   CMP_LT v3, v1, v6 ; BRANCH_NZ v3, __trap_oob ; ADD_IMM v1, v1, -1

  defp compile_left(state) do
    {ids, state} =
      if state.config.insert_bounds_checks do
        emit_bounds_check_left(state)
      else
        {[], state}
      end

    {id, state} =
      emit(state, :add_imm, [
        %IrRegister{index: @reg_tape_ptr},
        %IrRegister{index: @reg_tape_ptr},
        %IrImmediate{value: -1}
      ])

    {ids ++ [id], state}
  end

  # ── INC / DEC: cell mutation ──────────────────────────────────────────────────
  #
  # LOAD_BYTE  v2, v0, v1        ← load current cell
  # ADD_IMM    v2, v2, delta      ← increment/decrement
  # AND_IMM    v2, v2, 255        ← mask to byte (if enabled)
  # STORE_BYTE v2, v0, v1        ← store back

  defp emit_cell_mutation(state, delta) do
    # Load current cell
    {id1, state} =
      emit(state, :load_byte, [
        %IrRegister{index: @reg_temp},
        %IrRegister{index: @reg_tape_base},
        %IrRegister{index: @reg_tape_ptr}
      ])

    # Add delta
    {id2, state} =
      emit(state, :add_imm, [
        %IrRegister{index: @reg_temp},
        %IrRegister{index: @reg_temp},
        %IrImmediate{value: delta}
      ])

    # Mask to byte range if enabled
    {mask_ids, state} =
      if state.config.mask_byte_arithmetic do
        {id3, state} =
          emit(state, :and_imm, [
            %IrRegister{index: @reg_temp},
            %IrRegister{index: @reg_temp},
            %IrImmediate{value: 255}
          ])

        {[id3], state}
      else
        {[], state}
      end

    # Store back
    {id4, state} =
      emit(state, :store_byte, [
        %IrRegister{index: @reg_temp},
        %IrRegister{index: @reg_tape_base},
        %IrRegister{index: @reg_tape_ptr}
      ])

    {[id1, id2] ++ mask_ids ++ [id4], state}
  end

  # ── OUTPUT: write current cell to stdout ──────────────────────────────────────
  #
  # LOAD_BYTE v2, v0, v1          ← load current cell
  # ADD_IMM   v4, v2, 0           ← copy to syscall arg without relying on v6
  # SYSCALL   1                   ← write byte

  defp compile_output(state) do
    {id1, state} =
      emit(state, :load_byte, [
        %IrRegister{index: @reg_temp},
        %IrRegister{index: @reg_tape_base},
        %IrRegister{index: @reg_tape_ptr}
      ])

    {id2, state} =
      emit(state, :add_imm, [
        %IrRegister{index: @reg_sys_arg},
        %IrRegister{index: @reg_temp},
        %IrImmediate{value: 0}
      ])

    {id3, state} = emit(state, :syscall, [%IrImmediate{value: @syscall_write}])

    {[id1, id2, id3], state}
  end

  # ── INPUT: read byte from stdin into current cell ─────────────────────────────
  #
  # SYSCALL    2                  ← read byte (result in v4)
  # STORE_BYTE v4, v0, v1        ← store to current cell

  defp compile_input(state) do
    {id1, state} = emit(state, :syscall, [%IrImmediate{value: @syscall_read}])

    {id2, state} =
      emit(state, :store_byte, [
        %IrRegister{index: @reg_sys_arg},
        %IrRegister{index: @reg_tape_base},
        %IrRegister{index: @reg_tape_ptr}
      ])

    {[id1, id2], state}
  end

  # ── Bounds checking ───────────────────────────────────────────────────────────
  #
  # RIGHT check (>) — is ptr already at max before moving right?
  #   CMP_GT  v3, v1, v5        ← is ptr > max?
  #   BRANCH_NZ v3, __trap_oob  ← if so, trap
  #
  # LEFT check (<) — is ptr already at 0 before moving left?
  #   CMP_LT  v3, v1, v6        ← is ptr < 0?
  #   BRANCH_NZ v3, __trap_oob  ← if so, trap

  defp emit_bounds_check_right(state) do
    {id1, state} =
      emit(state, :cmp_gt, [
        %IrRegister{index: @reg_temp2},
        %IrRegister{index: @reg_tape_ptr},
        %IrRegister{index: @reg_max_ptr}
      ])

    {id2, state} =
      emit(state, :branch_nz, [
        %IrRegister{index: @reg_temp2},
        %IrLabel{name: "__trap_oob"}
      ])

    {[id1, id2], state}
  end

  defp emit_bounds_check_left(state) do
    {id1, state} =
      emit(state, :cmp_lt, [
        %IrRegister{index: @reg_tape_ptr},
        %IrRegister{index: @reg_tape_ptr},
        %IrRegister{index: @reg_zero}
      ])

    {id2, state} =
      emit(state, :branch_nz, [
        %IrRegister{index: @reg_tape_ptr},
        %IrLabel{name: "__trap_oob"}
      ])

    {[id1, id2], state}
  end

  # ── Loop compilation ──────────────────────────────────────────────────────────
  #
  # A Brainfuck loop [body] compiles to:
  #
  #   LABEL      loop_N_start
  #   LOAD_BYTE  v2, v0, v1          ← load current cell
  #   BRANCH_Z   v2, loop_N_end      ← skip body if cell == 0
  #   ...compile body...
  #   JUMP       loop_N_start        ← repeat
  #   LABEL      loop_N_end
  #
  # Loops nest arbitrarily deep. Each loop gets a unique number N
  # (from state.loop_count) to make labels unique.

  defp compile_loop(state, %ASTNode{} = node) do
    loop_num = state.loop_count
    state = %{state | loop_count: loop_num + 1}

    start_label = "loop_#{loop_num}_start"
    end_label = "loop_#{loop_num}_end"

    {ast_node_id, state} = next_node_id(state)

    # Record source position for the loop construct
    state =
      if node.start_line != nil and node.start_line > 0 do
        update_source_map(state, fn sm ->
          pos = %SourcePosition{
            file: state.filename,
            line: node.start_line,
            column: node.start_column || 1,
            length: 1
          }

          %{sm | source_to_ast: SourceToAst.add(sm.source_to_ast, pos, ast_node_id)}
        end)
      else
        state
      end

    # Emit loop start label
    state = emit_label(state, start_label)

    # Load current cell
    {id1, state} =
      emit(state, :load_byte, [
        %IrRegister{index: @reg_temp},
        %IrRegister{index: @reg_tape_base},
        %IrRegister{index: @reg_tape_ptr}
      ])

    # Branch if cell == 0
    {id2, state} =
      emit(state, :branch_z, [
        %IrRegister{index: @reg_temp},
        %IrLabel{name: end_label}
      ])

    # Compile loop body (skip bracket tokens, only recurse into ASTNodes)
    body_result =
      Enum.reduce_while(node.children, {:ok, state}, fn child, {:ok, acc_state} ->
        case child do
          %ASTNode{} ->
            case compile_node(acc_state, child) do
              {:ok, ns} -> {:cont, {:ok, ns}}
              err -> {:halt, err}
            end

          _ ->
            # Skip bracket tokens ([, ])
            {:cont, {:ok, acc_state}}
        end
      end)

    case body_result do
      {:error, _} = err ->
        err

      {:ok, state} ->
        # Jump back to loop start
        {id3, state} = emit(state, :jump, [%IrLabel{name: start_label}])

        # Emit loop end label
        state = emit_label(state, end_label)

        # Record AST → IR for the loop construct
        state = record_ast_to_ir(state, ast_node_id, [id1, id2, id3])

        {:ok, state}
    end
  end

  # ── Token extraction ──────────────────────────────────────────────────────────
  #
  # The AST structure from the grammar-driven parser is:
  #   command → Token (a leaf node wrapping a single token)
  #
  # extractToken digs through the AST to find the leaf token.

  defp extract_token(%ASTNode{} = node) do
    # Check if this is a leaf (single Token child)
    if ASTNode.leaf?(node) do
      ASTNode.token(node)
    else
      # Search children for a Token or a leaf ASTNode
      Enum.find_value(node.children, fn child ->
        case child do
          %Token{} -> child
          %ASTNode{} -> extract_token(child)
          _ -> nil
        end
      end)
    end
  end
end
