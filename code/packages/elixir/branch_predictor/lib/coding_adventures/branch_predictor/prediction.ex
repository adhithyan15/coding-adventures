defmodule CodingAdventures.BranchPredictor.Prediction do
  @moduledoc """
  A branch prediction — the predictor's guess before the branch executes.

  ## What's in a prediction?

  When the CPU's fetch stage encounters a branch instruction, the branch
  predictor produces a Prediction with three pieces of information:

  1. **predicted_taken** — will the branch jump to its target address? This is
     the core question. If taken, the CPU should fetch from the target. If not
     taken, it should fetch the next sequential instruction (PC + 4).

  2. **confidence** — how sure is the predictor? This ranges from 0.0
     (pure guess) to 1.0 (certain). Confidence is used by tournament/hybrid
     predictors that maintain multiple sub-predictors and pick the one with
     the highest confidence for each branch.

  3. **address** — the predicted target address, if known. This comes from the
     Branch Target Buffer (BTB), not the direction predictor itself. A value
     of nil means "I know it's taken, but I don't know the target address."

  ## Why a struct?

  Predictions are values, not mutable state. Once the predictor makes a
  guess, that guess should never change. In Elixir, structs are naturally
  immutable, making this a perfect fit.

  ## Examples

      # A confident prediction that the branch is taken, jumping to 0x400
      %Prediction{predicted_taken: true, confidence: 0.9, address: 0x400}

      # A low-confidence prediction from a cold-start predictor
      %Prediction{predicted_taken: false, confidence: 0.0}
  """

  defstruct [:predicted_taken, confidence: 0.0, address: nil]

  @type t :: %__MODULE__{
          predicted_taken: boolean(),
          confidence: float(),
          address: non_neg_integer() | nil
        }

  @doc """
  Create a new Prediction.

  ## Parameters

  - `attrs` — a keyword list or map with `:predicted_taken` (required), and
    optionally `:confidence` (default 0.0) and `:address` (default nil).

  ## Examples

      iex> CodingAdventures.BranchPredictor.Prediction.new(predicted_taken: true, confidence: 0.9, address: 0x400)
      %CodingAdventures.BranchPredictor.Prediction{predicted_taken: true, confidence: 0.9, address: 1024}

      iex> CodingAdventures.BranchPredictor.Prediction.new(predicted_taken: false)
      %CodingAdventures.BranchPredictor.Prediction{predicted_taken: false, confidence: 0.0, address: nil}
  """
  def new(attrs) when is_list(attrs) do
    struct!(__MODULE__, attrs)
  end

  def new(attrs) when is_map(attrs) do
    struct!(__MODULE__, Map.to_list(attrs))
  end
end
