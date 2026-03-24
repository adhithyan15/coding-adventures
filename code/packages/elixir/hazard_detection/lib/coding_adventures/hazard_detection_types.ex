defmodule CodingAdventures.PipelineSlot do
  @moduledoc """
  ISA-independent description of an instruction in one pipeline stage.
  """

  defstruct valid: false,
            pc: 0,
            source_regs: [],
            dest_reg: nil,
            dest_value: nil,
            is_branch: false,
            branch_taken: false,
            branch_predicted_taken: false,
            mem_read: false,
            mem_write: false,
            uses_alu: true,
            uses_fp: false

  @type t :: %__MODULE__{
          valid: boolean(),
          pc: non_neg_integer(),
          source_regs: [integer()],
          dest_reg: integer() | nil,
          dest_value: integer() | nil,
          is_branch: boolean(),
          branch_taken: boolean(),
          branch_predicted_taken: boolean(),
          mem_read: boolean(),
          mem_write: boolean(),
          uses_alu: boolean(),
          uses_fp: boolean()
        }
end

defmodule CodingAdventures.HazardResult do
  @moduledoc """
  Full hazard detection verdict for one cycle.
  """

  defstruct action: :none,
            forwarded_value: nil,
            forwarded_from: "",
            stall_cycles: 0,
            flush_count: 0,
            reason: ""

  @type action :: :none | :forward_ex | :forward_mem | :stall | :flush

  @type t :: %__MODULE__{
          action: action(),
          forwarded_value: integer() | nil,
          forwarded_from: String.t(),
          stall_cycles: non_neg_integer(),
          flush_count: non_neg_integer(),
          reason: String.t()
        }
end
