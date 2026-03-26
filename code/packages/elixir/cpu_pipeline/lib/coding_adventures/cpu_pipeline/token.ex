defmodule CodingAdventures.CpuPipeline.Token do
  @moduledoc """
  PipelineToken -- a unit of work flowing through the pipeline.

  Think of it as a tray on an assembly line. The tray starts empty at the
  IF stage, gets filled with decoded information at ID, gets computed
  results at EX, gets memory data at MEM, and delivers results at WB.

  The token is ISA-independent. The ISA decoder fills in the fields via
  callbacks. The pipeline itself never looks at instruction semantics --
  it only moves tokens between stages and handles stalls/flushes.

  ## Token Lifecycle

      IF stage:  FetchFunc fills in PC and raw_instruction
      ID stage:  DecodeFunc fills in opcode, registers, control signals
      EX stage:  ExecuteFunc fills in alu_result, branch_taken, branch_target
      MEM stage: MemoryFunc fills in mem_data (for loads)
      WB stage:  WritebackFunc uses write_data to update register file

  ## Bubbles

  A "bubble" is a special token that represents NO instruction. Bubbles
  are inserted when the pipeline stalls (to fill the gap left by frozen
  stages) or when the pipeline flushes (to replace discarded speculative
  instructions). A bubble flows through the pipeline like a normal token
  but does nothing at each stage.

  In hardware, a bubble is a NOP (no-operation) instruction. In our
  simulator, it is a token with `is_bubble: true`.
  """

  @type t :: %__MODULE__{
          pc: integer(),
          raw_instruction: integer(),
          opcode: String.t(),
          rs1: integer(),
          rs2: integer(),
          rd: integer(),
          immediate: integer(),
          reg_write: boolean(),
          mem_read: boolean(),
          mem_write: boolean(),
          is_branch: boolean(),
          is_halt: boolean(),
          alu_result: integer(),
          mem_data: integer(),
          write_data: integer(),
          branch_taken: boolean(),
          branch_target: integer(),
          is_bubble: boolean(),
          stage_entered: %{optional(String.t()) => integer()},
          forwarded_from: String.t()
        }

  defstruct pc: 0,
            raw_instruction: 0,
            opcode: "",
            rs1: -1,
            rs2: -1,
            rd: -1,
            immediate: 0,
            reg_write: false,
            mem_read: false,
            mem_write: false,
            is_branch: false,
            is_halt: false,
            alu_result: 0,
            mem_data: 0,
            write_data: 0,
            branch_taken: false,
            branch_target: 0,
            is_bubble: false,
            stage_entered: %{},
            forwarded_from: ""

  @doc """
  Creates a new empty token with default register values (-1 means unused).

  The token starts with all register fields set to -1 and all control
  signals set to false. The fetch callback will fill in the PC and raw
  instruction; the decode callback fills in everything else.
  """
  @spec new() :: t()
  def new do
    %__MODULE__{rs1: -1, rs2: -1, rd: -1, stage_entered: %{}}
  end

  @doc """
  Creates a new bubble token.

  A bubble is a "do nothing" instruction that occupies a pipeline stage
  without performing any useful work. It is the pipeline equivalent of
  a "no-op" on an assembly line.
  """
  @spec new_bubble() :: t()
  def new_bubble do
    %__MODULE__{is_bubble: true, rs1: -1, rs2: -1, rd: -1, stage_entered: %{}}
  end

  @doc """
  Returns a human-readable representation of the token.

  - Bubbles display as "---"
  - Normal tokens display their opcode and PC (e.g., "ADD@100")
  - Tokens without an opcode display "instr@PC"
  """
  @spec to_string(t()) :: String.t()
  def to_string(%__MODULE__{is_bubble: true}), do: "---"
  def to_string(%__MODULE__{opcode: opcode, pc: pc}) when opcode != "" do
    "#{opcode}@#{pc}"
  end
  def to_string(%__MODULE__{pc: pc}), do: "instr@#{pc}"

  @doc """
  Returns a deep copy of the token.

  Since Elixir data structures are immutable, a "clone" is simply the
  identity function -- the original cannot be mutated. This function
  exists for API parity with the Go implementation.
  """
  @spec clone(t() | nil) :: t() | nil
  def clone(nil), do: nil
  def clone(%__MODULE__{} = token), do: token
end

# =========================================================================
# StageCategory -- classifies pipeline stages by their function
# =========================================================================

defmodule CodingAdventures.CpuPipeline.StageCategory do
  @moduledoc """
  Classifies pipeline stages by their function.

  Every stage in a pipeline does one of these five jobs, regardless of
  how many stages the pipeline has. A 5-stage pipeline has one stage per
  category. A 13-stage pipeline might have 2 fetch stages, 2 decode
  stages, 3 execute stages, etc.

  Categories:
    - `:fetch`     -- stages that read instructions from the instruction cache
    - `:decode`    -- stages that decode the instruction and read registers
    - `:execute`   -- stages that perform computation (ALU, branch resolution)
    - `:memory`    -- stages that access data memory (loads and stores)
    - `:writeback` -- stages that write results back to the register file
  """

  @type t :: :fetch | :decode | :execute | :memory | :writeback

  @doc "Returns a human-readable name for the stage category."
  @spec to_string(t()) :: String.t()
  def to_string(:fetch), do: "fetch"
  def to_string(:decode), do: "decode"
  def to_string(:execute), do: "execute"
  def to_string(:memory), do: "memory"
  def to_string(:writeback), do: "writeback"
  def to_string(_), do: "unknown"
end

# =========================================================================
# PipelineStage -- definition of a single stage in the pipeline
# =========================================================================

defmodule CodingAdventures.CpuPipeline.PipelineStage do
  @moduledoc """
  Defines a single stage in the pipeline.

  A stage has a short name (used in diagrams), a description (for humans),
  and a category (for the pipeline to know what callback to invoke).

  ## Examples

      %PipelineStage{name: "IF", description: "Instruction Fetch", category: :fetch}
      %PipelineStage{name: "EX1", description: "Execute - ALU", category: :execute}
  """

  alias CodingAdventures.CpuPipeline.StageCategory

  @type t :: %__MODULE__{
          name: String.t(),
          description: String.t(),
          category: StageCategory.t()
        }

  defstruct name: "", description: "", category: :fetch

  @doc "Returns the stage name for display in diagrams."
  @spec to_string(t()) :: String.t()
  def to_string(%__MODULE__{name: name}), do: name
end

# =========================================================================
# PipelineConfig -- configuration for the pipeline
# =========================================================================

defmodule CodingAdventures.CpuPipeline.PipelineConfig do
  @moduledoc """
  Holds the configuration for a pipeline.

  The key insight: a pipeline's behavior is determined entirely by its
  stage configuration and execution width. Everything else (instruction
  semantics, hazard handling) is injected via callbacks.
  """

  alias CodingAdventures.CpuPipeline.PipelineStage

  @type t :: %__MODULE__{
          stages: [PipelineStage.t()],
          execution_width: pos_integer()
        }

  defstruct stages: [], execution_width: 1

  @doc "Returns the number of stages in the pipeline."
  @spec num_stages(t()) :: non_neg_integer()
  def num_stages(%__MODULE__{stages: stages}), do: length(stages)

  @doc """
  Validates that the configuration is well-formed.

  Rules:
    - Must have at least 2 stages
    - Execution width must be at least 1
    - All stage names must be unique
    - There must be at least one fetch stage and one writeback stage

  Returns `:ok` or `{:error, reason}`.
  """
  @spec validate(t()) :: :ok | {:error, String.t()}
  def validate(%__MODULE__{stages: stages, execution_width: width}) do
    cond do
      length(stages) < 2 ->
        {:error, "pipeline must have at least 2 stages, got #{length(stages)}"}

      width < 1 ->
        {:error, "execution width must be at least 1, got #{width}"}

      true ->
        with :ok <- check_unique_names(stages),
             :ok <- check_required_categories(stages) do
          :ok
        end
    end
  end

  defp check_unique_names(stages) do
    names = Enum.map(stages, & &1.name)
    unique_names = Enum.uniq(names)

    if length(names) != length(unique_names) do
      dup = names -- unique_names |> List.first()
      {:error, "duplicate stage name: \"#{dup}\""}
    else
      :ok
    end
  end

  defp check_required_categories(stages) do
    categories = Enum.map(stages, & &1.category)
    has_fetch = :fetch in categories
    has_writeback = :writeback in categories

    cond do
      not has_fetch ->
        {:error, "pipeline must have at least one fetch stage"}

      not has_writeback ->
        {:error, "pipeline must have at least one writeback stage"}

      true ->
        :ok
    end
  end
end

# =========================================================================
# HazardAction -- what the hazard detector tells the pipeline to do
# =========================================================================

defmodule CodingAdventures.CpuPipeline.HazardResponse do
  @moduledoc """
  The full response from the hazard detection callback.

  Tells the pipeline what to do and provides additional context
  (forwarded values, stall duration, flush target).

  ## Hazard Actions

  These are "traffic signals" for the pipeline:

    - `:none`             -- Green light -- pipeline flows normally
    - `:stall`            -- Red light -- freeze earlier stages, insert bubble
    - `:flush`            -- Emergency stop -- discard speculative instructions
    - `:forward_from_ex`  -- Shortcut -- grab value from EX stage output
    - `:forward_from_mem` -- Shortcut -- grab value from MEM stage output

  Priority: `:flush` > `:stall` > `:forward_*` > `:none`
  """

  @type action :: :none | :forward_from_ex | :forward_from_mem | :stall | :flush

  @type t :: %__MODULE__{
          action: action(),
          forward_value: integer(),
          forward_source: String.t(),
          stall_stages: integer(),
          flush_count: integer(),
          redirect_pc: integer()
        }

  defstruct action: :none,
            forward_value: 0,
            forward_source: "",
            stall_stages: 0,
            flush_count: 0,
            redirect_pc: 0
end
