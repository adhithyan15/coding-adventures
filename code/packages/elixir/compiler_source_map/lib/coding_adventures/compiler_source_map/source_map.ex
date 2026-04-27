defmodule CodingAdventures.CompilerSourceMap.SourcePosition do
  @moduledoc """
  A span of characters in a source file.

  Think of `SourcePosition` as a highlighter pen marking a region of
  source code. The `{line, column}` pair marks the start; `length` tells
  you how many characters are highlighted.

  ## Brainfuck example

  In Brainfuck every command is exactly one character, so `length` is
  always 1. The `+` at line 1, column 3 of `hello.bf` is:

      %SourcePosition{file: "hello.bf", line: 1, column: 3, length: 1}

  ## Future languages

  In BASIC, a keyword like `PRINT` would have `length: 5`. A string
  literal `"hello"` would have `length: 7` (including the quotes).

  ## Fields

  - `file`   — source file path (e.g. `"hello.bf"`, `"program.bas"`)
  - `line`   — 1-based line number
  - `column` — 1-based column number
  - `length` — number of characters in the span
  """

  defstruct [:file, :line, :column, :length]

  @type t :: %__MODULE__{
          file: String.t(),
          line: pos_integer(),
          column: pos_integer(),
          length: pos_integer()
        }

  @doc """
  Return a human-readable representation.

  ## Examples

      iex> sp = %SourcePosition{file: "hello.bf", line: 1, column: 3, length: 1}
      iex> SourcePosition.to_string(sp)
      "hello.bf:1:3 (len=1)"
  """
  @spec to_string(t()) :: String.t()
  def to_string(%__MODULE__{file: f, line: l, column: c, length: n}) do
    "#{f}:#{l}:#{c} (len=#{n})"
  end
end

defmodule CodingAdventures.CompilerSourceMap.SourceToAst do
  @moduledoc """
  Segment 1: source text positions → AST node IDs.

  This segment is produced by the parser or language frontend. It maps
  every meaningful source position to the AST node that represents it.

  ## Example

  The `+` at line 1, column 3 of `hello.bf` maps to AST node #42:

      SourceToAst.add(seg, %SourcePosition{file: "hello.bf", line: 1, column: 3, length: 1}, 42)

  ## Lookups

  - `lookup_by_node_id/2` — given an AST node ID, find its source span.
  """

  alias CodingAdventures.CompilerSourceMap.SourcePosition

  @type entry :: %{pos: SourcePosition.t(), ast_node_id: integer()}

  defstruct entries: []

  @type t :: %__MODULE__{
          entries: [entry()]
        }

  @doc "Record a mapping from a source position to an AST node ID."
  @spec add(t(), SourcePosition.t(), integer()) :: t()
  def add(%__MODULE__{entries: entries} = seg, %SourcePosition{} = pos, ast_node_id) do
    %{seg | entries: entries ++ [%{pos: pos, ast_node_id: ast_node_id}]}
  end

  @doc """
  Return the source position for the given AST node ID, or `nil`.

  Used for reverse lookups: given a machine-code address, trace back
  through all pipeline stages to find the source position.
  """
  @spec lookup_by_node_id(t(), integer()) :: SourcePosition.t() | nil
  def lookup_by_node_id(%__MODULE__{entries: entries}, ast_node_id) do
    case Enum.find(entries, fn e -> e.ast_node_id == ast_node_id end) do
      nil -> nil
      entry -> entry.pos
    end
  end
end

defmodule CodingAdventures.CompilerSourceMap.AstToIr do
  @moduledoc """
  Segment 2: AST node IDs → IR instruction IDs.

  A single AST node often produces multiple IR instructions. For example,
  a Brainfuck `+` command produces four instructions:

      LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE

  So the mapping is **one-to-many**: ast_node_42 → [ir_7, ir_8, ir_9, ir_10].

  ## Lookups

  - `lookup_by_ast_node_id/2` — which IR IDs did this AST node produce?
  - `lookup_by_ir_id/2`       — which AST node produced this IR instruction?
  """

  defstruct entries: []

  @type entry :: %{ast_node_id: integer(), ir_ids: [integer()]}

  @type t :: %__MODULE__{
          entries: [entry()]
        }

  @doc "Record that the given AST node produced the given IR instruction IDs."
  @spec add(t(), integer(), [integer()]) :: t()
  def add(%__MODULE__{entries: entries} = seg, ast_node_id, ir_ids) do
    %{seg | entries: entries ++ [%{ast_node_id: ast_node_id, ir_ids: ir_ids}]}
  end

  @doc "Return the IR instruction IDs for the given AST node, or `nil`."
  @spec lookup_by_ast_node_id(t(), integer()) :: [integer()] | nil
  def lookup_by_ast_node_id(%__MODULE__{entries: entries}, ast_node_id) do
    case Enum.find(entries, fn e -> e.ast_node_id == ast_node_id end) do
      nil -> nil
      entry -> entry.ir_ids
    end
  end

  @doc """
  Return the AST node ID that produced the given IR instruction, or `-1`.

  Used for reverse lookups during debugging: given an IR instruction ID,
  which source command emitted it?
  """
  @spec lookup_by_ir_id(t(), integer()) :: integer()
  def lookup_by_ir_id(%__MODULE__{entries: entries}, ir_id) do
    result =
      Enum.find(entries, fn e ->
        Enum.member?(e.ir_ids, ir_id)
      end)

    case result do
      nil -> -1
      entry -> entry.ast_node_id
    end
  end
end

defmodule CodingAdventures.CompilerSourceMap.IrToIr do
  @moduledoc """
  Segment 3: IR instruction IDs → optimised IR instruction IDs.

  One `IrToIr` segment is produced per optimiser pass. The `pass_name`
  field identifies which pass produced this mapping (e.g. `"identity"`,
  `"contraction"`, `"clear_loop"`, `"dead_store"`).

  ## Three cases per instruction

  1. **Preserved**: original_id → [same_id] (instruction unchanged)
  2. **Replaced**: original_id → [new_id_1, ...] (split or transformed)
  3. **Deleted**: original_id is in `deleted` set (optimised away)

  ## Example: contraction pass

  Three `ADD_IMM 1` instructions (IDs 7, 8, 9) are folded into one
  `ADD_IMM 3` (ID 100):

      7 → [100], 8 → [100], 9 → [100]
  """

  defstruct entries: [], deleted: MapSet.new(), pass_name: ""

  @type entry :: %{original_id: integer(), new_ids: [integer()]}

  @type t :: %__MODULE__{
          entries: [entry()],
          deleted: MapSet.t(),
          pass_name: String.t()
        }

  @doc "Create an `IrToIr` segment for the named optimiser pass."
  @spec new(String.t()) :: t()
  def new(pass_name) do
    %__MODULE__{pass_name: pass_name}
  end

  @doc "Record that the original instruction was replaced by the new ones."
  @spec add_mapping(t(), integer(), [integer()]) :: t()
  def add_mapping(%__MODULE__{entries: entries} = seg, original_id, new_ids) do
    %{seg | entries: entries ++ [%{original_id: original_id, new_ids: new_ids}]}
  end

  @doc "Record that the original instruction was deleted (optimised away)."
  @spec add_deletion(t(), integer()) :: t()
  def add_deletion(%__MODULE__{entries: entries, deleted: deleted} = seg, original_id) do
    %{
      seg
      | entries: entries ++ [%{original_id: original_id, new_ids: []}],
        deleted: MapSet.put(deleted, original_id)
    }
  end

  @doc "Return the new IDs for the given original ID, or `nil` if deleted or not found."
  @spec lookup_by_original_id(t(), integer()) :: [integer()] | nil
  def lookup_by_original_id(%__MODULE__{deleted: deleted, entries: entries}, original_id) do
    if MapSet.member?(deleted, original_id) do
      nil
    else
      case Enum.find(entries, fn e -> e.original_id == original_id end) do
        nil -> nil
        entry -> entry.new_ids
      end
    end
  end

  @doc """
  Return the original ID that produced the given new ID, or `-1`.

  When multiple originals map to the same new ID (e.g. contraction),
  returns the first one found.
  """
  @spec lookup_by_new_id(t(), integer()) :: integer()
  def lookup_by_new_id(%__MODULE__{entries: entries}, new_id) do
    result =
      Enum.find(entries, fn e ->
        Enum.member?(e.new_ids, new_id)
      end)

    case result do
      nil -> -1
      entry -> entry.original_id
    end
  end
end

defmodule CodingAdventures.CompilerSourceMap.IrToMachineCode do
  @moduledoc """
  Segment 4: IR instruction IDs → machine code byte offsets.

  Each entry is a triple: `(ir_id, mc_offset, mc_length)`.

  For example, a `LOAD_BYTE` IR instruction might produce 8 bytes of
  RISC-V machine code starting at offset 0x14 in the `.text` section:

      %{ir_id: 5, mc_offset: 0x14, mc_length: 8}

  ## Lookups

  - `lookup_by_ir_id/2`   — which machine-code bytes did this IR emit?
  - `lookup_by_mc_offset/2` — which IR instruction contains this byte?
  """

  defstruct entries: []

  @type entry :: %{ir_id: integer(), mc_offset: integer(), mc_length: integer()}

  @type t :: %__MODULE__{
          entries: [entry()]
        }

  @doc "Record that the given IR instruction produced machine code at the given offset."
  @spec add(t(), integer(), integer(), integer()) :: t()
  def add(%__MODULE__{entries: entries} = seg, ir_id, mc_offset, mc_length) do
    %{seg | entries: entries ++ [%{ir_id: ir_id, mc_offset: mc_offset, mc_length: mc_length}]}
  end

  @doc """
  Return `{offset, length}` for the given IR instruction ID, or `{-1, 0}`.
  """
  @spec lookup_by_ir_id(t(), integer()) :: {integer(), integer()}
  def lookup_by_ir_id(%__MODULE__{entries: entries}, ir_id) do
    case Enum.find(entries, fn e -> e.ir_id == ir_id end) do
      nil -> {-1, 0}
      entry -> {entry.mc_offset, entry.mc_length}
    end
  end

  @doc """
  Return the IR instruction ID whose machine code contains the given byte
  offset, or `-1` if not found.

  A machine code offset "contains" an IR instruction if:

      entry.mc_offset <= offset < entry.mc_offset + entry.mc_length
  """
  @spec lookup_by_mc_offset(t(), integer()) :: integer()
  def lookup_by_mc_offset(%__MODULE__{entries: entries}, offset) do
    result =
      Enum.find(entries, fn e ->
        offset >= e.mc_offset and offset < e.mc_offset + e.mc_length
      end)

    case result do
      nil -> -1
      entry -> entry.ir_id
    end
  end
end

defmodule CodingAdventures.CompilerSourceMap.SourceMapChain do
  @moduledoc """
  The full compiler pipeline sidecar — all source map segments in one struct.

  The chain flows through every stage of the pipeline:

  ```
  Frontend (brainfuck_ir_compiler)
    → fills source_to_ast + ast_to_ir

  Optimiser (compiler_ir_optimizer)
    → appends ir_to_ir segments (one per pass)

  Backend (codegen_riscv)
    → fills ir_to_machine_code
  ```

  ## Composite queries

  The chain supports two end-to-end queries:

  - `source_to_mc/2` — given a source position, find the machine-code bytes.
  - `mc_to_source/2` — given a machine-code offset, find the source position.

  These compose all four segments automatically, following forward or reverse
  arrows through each transformation stage.

  ## Why a chain instead of a flat table?

  A flat table (machine-code offset → source position) is useful for the
  final consumer (debugger, profiler, error reporter). But it doesn't help
  when debugging the *compiler itself*:

  - "Why did the optimiser delete instruction #42?"
    → Inspect the `IrToIr` segment for that pass.

  - "Which AST node produced this IR instruction?"
    → Inspect `AstToIr`.

  - "The machine code for this instruction seems wrong — what IR produced it?"
    → Inspect `IrToMachineCode` in reverse.

  The chain makes the compiler pipeline **transparent and debuggable at
  every stage**.
  """

  alias CodingAdventures.CompilerSourceMap.{
    SourcePosition,
    SourceToAst,
    AstToIr,
    IrToIr,
    IrToMachineCode
  }

  defstruct source_to_ast: nil,
            ast_to_ir: nil,
            ir_to_ir: [],
            ir_to_machine_code: nil

  @type t :: %__MODULE__{
          source_to_ast: SourceToAst.t(),
          ast_to_ir: AstToIr.t(),
          ir_to_ir: [IrToIr.t()],
          ir_to_machine_code: IrToMachineCode.t() | nil
        }

  @doc "Create an empty source map chain ready for use."
  @spec new() :: t()
  def new do
    %__MODULE__{
      source_to_ast: %SourceToAst{},
      ast_to_ir: %AstToIr{},
      ir_to_ir: [],
      ir_to_machine_code: nil
    }
  end

  @doc "Append an `IrToIr` segment from an optimiser pass."
  @spec add_optimizer_pass(t(), IrToIr.t()) :: t()
  def add_optimizer_pass(%__MODULE__{ir_to_ir: passes} = chain, %IrToIr{} = segment) do
    %{chain | ir_to_ir: passes ++ [segment]}
  end

  # ── Composite queries ──────────────────────────────────────────────────────

  @doc """
  Compose all segments to look up machine code offset(s) for a source position.

  Returns a list of `{ir_id, mc_offset, mc_length}` triples, or `[]` if
  the chain is incomplete or no mapping exists.

  ## Algorithm

  1. `SourceToAst`: source position → AST node ID
  2. `AstToIr`: AST node ID → IR instruction IDs
  3. `IrToIr` (each pass): follow IR IDs through each optimiser pass
  4. `IrToMachineCode`: final IR IDs → machine code offsets
  """
  @spec source_to_mc(t(), SourcePosition.t()) ::
          [{ir_id :: integer(), mc_offset :: integer(), mc_length :: integer()}]
  def source_to_mc(%__MODULE__{ir_to_machine_code: nil}, _pos), do: []

  def source_to_mc(%__MODULE__{} = chain, %SourcePosition{} = pos) do
    # Step 1: source → AST node ID
    ast_node_id =
      chain.source_to_ast.entries
      |> Enum.find(fn e ->
        e.pos.file == pos.file and
          e.pos.line == pos.line and
          e.pos.column == pos.column
      end)
      |> case do
        nil -> -1
        entry -> entry.ast_node_id
      end

    if ast_node_id == -1 do
      []
    else
      # Step 2: AST node → IR IDs
      ir_ids = AstToIr.lookup_by_ast_node_id(chain.ast_to_ir, ast_node_id)

      if ir_ids == nil do
        []
      else
        # Step 3: follow through optimiser passes
        current_ids =
          Enum.reduce(chain.ir_to_ir, ir_ids, fn pass, ids ->
            ids
            |> Enum.flat_map(fn id ->
              if MapSet.member?(pass.deleted, id) do
                []
              else
                IrToIr.lookup_by_original_id(pass, id) || []
              end
            end)
          end)

        if current_ids == [] do
          []
        else
          # Step 4: IR IDs → machine code
          current_ids
          |> Enum.flat_map(fn id ->
            case IrToMachineCode.lookup_by_ir_id(chain.ir_to_machine_code, id) do
              {-1, _} -> []
              {offset, length} -> [{id, offset, length}]
            end
          end)
        end
      end
    end
  end

  @doc """
  Compose all segments in reverse to look up the source position for a
  machine-code offset.

  Returns a `SourcePosition` or `nil` if the chain is incomplete or
  no mapping exists.

  ## Algorithm (reverse of `source_to_mc/2`)

  1. `IrToMachineCode`: MC offset → IR instruction ID
  2. `IrToIr` (each pass, in reverse): follow IR ID back through passes
  3. `AstToIr`: IR ID → AST node ID
  4. `SourceToAst`: AST node ID → source position
  """
  @spec mc_to_source(t(), integer()) :: SourcePosition.t() | nil
  def mc_to_source(%__MODULE__{ir_to_machine_code: nil}, _mc_offset), do: nil

  def mc_to_source(%__MODULE__{} = chain, mc_offset) do
    # Step 1: MC offset → IR ID
    ir_id = IrToMachineCode.lookup_by_mc_offset(chain.ir_to_machine_code, mc_offset)

    if ir_id == -1 do
      nil
    else
      # Step 2: follow back through optimiser passes (in reverse order)
      current_id =
        chain.ir_to_ir
        |> Enum.reverse()
        |> Enum.reduce_while(ir_id, fn pass, id ->
          original_id = IrToIr.lookup_by_new_id(pass, id)

          if original_id == -1 do
            {:halt, -1}
          else
            {:cont, original_id}
          end
        end)

      if current_id == -1 do
        nil
      else
        # Step 3: IR ID → AST node ID
        ast_node_id = AstToIr.lookup_by_ir_id(chain.ast_to_ir, current_id)

        if ast_node_id == -1 do
          nil
        else
          # Step 4: AST node ID → source position
          SourceToAst.lookup_by_node_id(chain.source_to_ast, ast_node_id)
        end
      end
    end
  end
end
