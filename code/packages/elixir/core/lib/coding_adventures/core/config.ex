defmodule CodingAdventures.Core.Config do
  @moduledoc """
  CoreConfig holds every tunable parameter for a processor core.

  This is the "spec sheet" for the core. A CPU architect decides these
  values based on the target workload, power budget, and die area.

  Changing any parameter affects measurable performance:

      Deeper pipeline         -> higher clock speed, worse misprediction penalty
      Larger register file    -> fewer spills to memory
      More memory             -> larger programs can run
  """

  alias CodingAdventures.CpuPipeline.{Pipeline, PipelineConfig}

  # =========================================================================
  # RegisterFileConfig
  # =========================================================================

  defmodule RegisterFileConfig do
    @moduledoc """
    Configuration for the general-purpose register file.

    Real-world register file sizes:

        MIPS:     32 registers, 32-bit  (R0 hardwired to zero)
        ARMv8:    31 registers, 64-bit  (X0-X30, no zero register)
        RISC-V:   32 registers, 32/64-bit (x0 hardwired to zero)
        x86-64:   16 registers, 64-bit  (RAX, RBX, ..., R15)
    """

    @type t :: %__MODULE__{
            count: pos_integer(),
            width: pos_integer(),
            zero_register: boolean()
          }

    defstruct count: 16, width: 32, zero_register: true
  end

  # =========================================================================
  # FPUnitConfig
  # =========================================================================

  defmodule FPUnitConfig do
    @moduledoc """
    Configuration for the optional floating-point unit.

    Not all cores have an FP unit. Microcontrollers and efficiency cores
    often omit it to save area and power.
    """

    @type t :: %__MODULE__{
            formats: [String.t()],
            pipeline_depth: pos_integer()
          }

    defstruct formats: [], pipeline_depth: 3
  end

  # =========================================================================
  # CoreConfig
  # =========================================================================

  @type t :: %__MODULE__{
          name: String.t(),
          pipeline: PipelineConfig.t(),
          hazard_detection: boolean(),
          forwarding: boolean(),
          register_file: RegisterFileConfig.t() | nil,
          fp_unit: FPUnitConfig.t() | nil,
          memory_size: pos_integer(),
          memory_latency: pos_integer()
        }

  defstruct name: "Default",
            pipeline: nil,
            hazard_detection: true,
            forwarding: true,
            register_file: nil,
            fp_unit: nil,
            memory_size: 65536,
            memory_latency: 100

  @doc """
  Returns a minimal, sensible configuration for testing.

  This is the "teaching core" -- a 5-stage pipeline with 16 registers.
  Equivalent to a 1980s RISC microprocessor.
  """
  @spec default_config() :: t()
  def default_config do
    %__MODULE__{
      name: "Default",
      pipeline: Pipeline.classic_5_stage(),
      hazard_detection: true,
      forwarding: true,
      register_file: nil,
      fp_unit: nil,
      memory_size: 65536,
      memory_latency: 100
    }
  end

  @doc """
  Returns a minimal teaching core configuration.

  Inspired by the MIPS R2000 (1985):
    - 5-stage pipeline (IF, ID, EX, MEM, WB)
    - 16 registers, 32-bit, zero register enabled
    - No floating point
    - 64KB memory

  Expected IPC: ~0.7-0.9 on simple programs.
  """
  @spec simple_config() :: t()
  def simple_config do
    %__MODULE__{
      name: "Simple",
      pipeline: Pipeline.classic_5_stage(),
      hazard_detection: true,
      forwarding: true,
      register_file: %RegisterFileConfig{count: 16, width: 32, zero_register: true},
      fp_unit: nil,
      memory_size: 65536,
      memory_latency: 100
    }
  end

  @doc """
  Approximates the ARM Cortex-A78 performance core.

  The Cortex-A78 (2020) is used in Snapdragon 888 and Dimensity 9000:
    - 13-stage pipeline (deep for high frequency)
    - 31 registers, 64-bit (ARMv8)
    - FP32 and FP64 support
    - 1MB memory

  Expected IPC: ~0.85-0.95 (our model is in-order; real A78 is out-of-order).
  """
  @spec cortex_a78_like_config() :: t()
  def cortex_a78_like_config do
    %__MODULE__{
      name: "CortexA78Like",
      pipeline: Pipeline.deep_13_stage(),
      hazard_detection: true,
      forwarding: true,
      register_file: %RegisterFileConfig{count: 31, width: 64, zero_register: false},
      fp_unit: %FPUnitConfig{formats: ["fp32", "fp64"], pipeline_depth: 4},
      memory_size: 1_048_576,
      memory_latency: 100
    }
  end
end

# =========================================================================
# MultiCoreConfig
# =========================================================================

defmodule CodingAdventures.Core.MultiCoreConfig do
  @moduledoc """
  Configuration for a multi-core processor.

  In a multi-core system, each core has its own private state but shares
  main memory. The memory controller serializes requests from multiple cores.
  """

  alias CodingAdventures.Core.Config

  @type t :: %__MODULE__{
          num_cores: pos_integer(),
          core_config: Config.t(),
          memory_size: pos_integer(),
          memory_latency: pos_integer()
        }

  defstruct num_cores: 2,
            core_config: nil,
            memory_size: 1_048_576,
            memory_latency: 100

  @doc "Returns a 2-core configuration for testing."
  @spec default_config() :: t()
  def default_config do
    %__MODULE__{
      num_cores: 2,
      core_config: Config.simple_config(),
      memory_size: 1_048_576,
      memory_latency: 100
    }
  end
end
