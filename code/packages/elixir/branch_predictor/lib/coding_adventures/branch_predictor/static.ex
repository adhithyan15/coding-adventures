defmodule CodingAdventures.BranchPredictor.Static do
  @moduledoc """
  Static branch predictors — the simplest strategies, requiring no learning.

  Static predictors make the same prediction every time, regardless of history.
  They require zero hardware (no tables, no counters, no state) and serve as
  baselines against which dynamic predictors are measured.

  Three strategies are implemented:

  1. **AlwaysTaken** — always predicts "taken"
     Accuracy: ~60-70% on typical code. Why? Most branches are loop back-edges,
     which are taken on every iteration except the last.

  2. **AlwaysNotTaken** — always predicts "not taken"
     Accuracy: ~30-40% on typical code. The worst reasonable strategy, but the
     simplest to implement — just fetch the next sequential instruction. This
     is what the Intel 8086 effectively did (no branch predictor at all).

  3. **BTFNT (Backward Taken, Forward Not Taken)** — see btfnt.ex
     Accuracy: ~65-75% on typical code. Backward branches (target < pc) are
     usually loop back-edges -> predict taken. Forward branches (target > pc)
     are usually if-else -> predict not-taken. Used in MIPS R4000, SPARC V8.
  """
end

defmodule CodingAdventures.BranchPredictor.Static.AlwaysTaken do
  @moduledoc """
  Always predicts "taken". Simple but surprisingly effective (~60% accurate).

  ## Why it works

  Most branches in real programs are loop back-edges, which are taken on
  every iteration except the last. A loop that runs 100 times has 100
  branches: 99 taken + 1 not-taken = 99% accuracy on that loop alone.
  The overall ~60% comes from mixing loops with if-else branches.

  ## Hardware cost

  Zero. No tables, no counters, no state at all. The prediction logic is
  just a wire tied to 1. You can't get simpler than this.

  ## When it works well

  - Tight loops: `for i in 0..999` — 999/1000 correct
  - Unconditional jumps — 100% correct (they're always taken)

  ## When it fails

  - Random if/else branches — ~50% correct (coin flip)
  - Early loop exits — misses every exit

  ## Examples

      iex> alias CodingAdventures.BranchPredictor.Static.AlwaysTaken
      iex> p = AlwaysTaken.new()
      iex> {pred, _p} = AlwaysTaken.predict(p, 0x100)
      iex> pred.predicted_taken
      true
  """

  alias CodingAdventures.BranchPredictor.{Prediction, Stats}

  defstruct stats: %Stats{}

  @type t :: %__MODULE__{stats: Stats.t()}

  @doc "Create a new AlwaysTaken predictor."
  def new, do: %__MODULE__{}

  @doc """
  Always predict taken, with zero confidence (it's just a guess).

  The pc is ignored — the prediction is always the same regardless
  of which branch we're looking at.

  Returns `{prediction, predictor}` (predictor unchanged since there's
  no state to update during prediction).
  """
  def predict(%__MODULE__{} = predictor, _pc) do
    {%Prediction{predicted_taken: true, confidence: 0.0}, predictor}
  end

  @doc """
  Record whether the always-taken guess was correct.

  We predicted taken, so we're correct when the branch actually was taken.
  The pc and target are unused — there's no per-branch state.
  """
  def update(%__MODULE__{} = predictor, _pc, taken, _target \\ nil) do
    %{predictor | stats: Stats.record(predictor.stats, taken)}
  end

  @doc "Get prediction accuracy statistics."
  def stats(%__MODULE__{stats: stats}), do: stats

  @doc "Reset statistics (no predictor state to clear)."
  def reset(%__MODULE__{}), do: %__MODULE__{}
end

defmodule CodingAdventures.BranchPredictor.Static.AlwaysNotTaken do
  @moduledoc """
  Always predicts "not taken". The simplest possible predictor.

  ## Why it exists

  This is the baseline against which all other predictors are measured.
  If your fancy predictor can't beat "always not taken", something is wrong.

  ## Hardware advantage

  The "not taken" path is just the next sequential instruction (PC + 4),
  which the fetch unit is already computing. No target address calculation
  needed. This is why the earliest processors (Intel 8086, 1978) implicitly
  used this strategy — they had no branch prediction unit, so they just
  kept fetching sequentially.

  ## Examples

      iex> alias CodingAdventures.BranchPredictor.Static.AlwaysNotTaken
      iex> p = AlwaysNotTaken.new()
      iex> {pred, _p} = AlwaysNotTaken.predict(p, 0x100)
      iex> pred.predicted_taken
      false
  """

  alias CodingAdventures.BranchPredictor.{Prediction, Stats}

  defstruct stats: %Stats{}

  @type t :: %__MODULE__{stats: Stats.t()}

  @doc "Create a new AlwaysNotTaken predictor."
  def new, do: %__MODULE__{}

  @doc """
  Always predict not taken, with zero confidence.

  Returns `{prediction, predictor}` (predictor unchanged).
  """
  def predict(%__MODULE__{} = predictor, _pc) do
    {%Prediction{predicted_taken: false, confidence: 0.0}, predictor}
  end

  @doc """
  Record whether the always-not-taken guess was correct.

  We predicted NOT taken, so we're correct when the branch was NOT taken.
  """
  def update(%__MODULE__{} = predictor, _pc, taken, _target \\ nil) do
    %{predictor | stats: Stats.record(predictor.stats, not taken)}
  end

  @doc "Get prediction accuracy statistics."
  def stats(%__MODULE__{stats: stats}), do: stats

  @doc "Reset statistics (no predictor state to clear)."
  def reset(%__MODULE__{}), do: %__MODULE__{}
end
