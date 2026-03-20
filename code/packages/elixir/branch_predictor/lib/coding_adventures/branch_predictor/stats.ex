defmodule CodingAdventures.BranchPredictor.Stats do
  @moduledoc """
  Prediction statistics — measuring how well a branch predictor performs.

  ## Why track stats?

  Every branch predictor needs a scorecard. When a CPU designer evaluates
  a predictor, the first question is always: "What's the accuracy?" A
  predictor that's 95% accurate causes a pipeline flush on only 5% of
  branches, while a 70% accurate predictor flushes on 30% — potentially
  halving throughput on a deeply pipelined machine.

  ## Counters

  We track three simple counters:

  - `predictions` — total number of branches seen
  - `correct` — how many the predictor got right
  - `incorrect` — how many it got wrong

  From these, we derive:

  - `accuracy` — correct / predictions x 100 (as a percentage)
  - `misprediction_rate` — incorrect / predictions x 100 (the complement)

  ## Edge case: no predictions yet

  If no predictions have been made, both rates return 0.0 rather than
  crashing with a division-by-zero error. This is a design choice — a
  predictor that hasn't seen any branches has no accuracy, not infinite
  accuracy.

  ## Real-world context

  - Intel's Pentium Pro achieved ~90% accuracy with a two-level adaptive predictor
  - Modern CPUs (since ~2015) achieve 95-99% accuracy using TAGE or perceptron predictors
  - Even a 1% improvement in accuracy can yield measurable speedups on branch-heavy code

  ## Immutability

  Since this is Elixir, the Stats struct is immutable. The `record/2` function
  returns a NEW Stats struct with updated counters. This makes it easy to
  snapshot stats at any point or compare stats from different time windows.
  """

  defstruct predictions: 0, correct: 0, incorrect: 0

  @type t :: %__MODULE__{
          predictions: non_neg_integer(),
          correct: non_neg_integer(),
          incorrect: non_neg_integer()
        }

  @doc """
  Create a new Stats struct with all counters at zero.

  ## Examples

      iex> CodingAdventures.BranchPredictor.Stats.new()
      %CodingAdventures.BranchPredictor.Stats{predictions: 0, correct: 0, incorrect: 0}
  """
  def new do
    %__MODULE__{}
  end

  @doc """
  Record the outcome of a single prediction.

  This is the primary function that the CPU core calls after every branch.
  It increments the total prediction count and either the correct or
  incorrect counter based on the outcome.

  ## Parameters

  - `stats` — the current Stats struct
  - `correct?` — true if the predictor guessed correctly, false otherwise

  ## Returns

  A new Stats struct with updated counters.

  ## Examples

      iex> stats = CodingAdventures.BranchPredictor.Stats.new()
      iex> stats = CodingAdventures.BranchPredictor.Stats.record(stats, true)
      iex> stats.predictions
      1
      iex> stats.correct
      1
      iex> stats = CodingAdventures.BranchPredictor.Stats.record(stats, false)
      iex> stats.incorrect
      1
  """
  def record(%__MODULE__{} = stats, correct?) when is_boolean(correct?) do
    if correct? do
      %{stats | predictions: stats.predictions + 1, correct: stats.correct + 1}
    else
      %{stats | predictions: stats.predictions + 1, incorrect: stats.incorrect + 1}
    end
  end

  @doc """
  Prediction accuracy as a percentage (0.0 to 100.0).

  Returns 0.0 if no predictions have been made yet, because we can't
  divide by zero, and "no data" is semantically closer to "0% accurate"
  than "100% accurate" in a benchmarking context.

  ## Examples

      iex> stats = %CodingAdventures.BranchPredictor.Stats{predictions: 100, correct: 87, incorrect: 13}
      iex> CodingAdventures.BranchPredictor.Stats.accuracy(stats)
      87.0
  """
  def accuracy(%__MODULE__{predictions: 0}), do: 0.0

  def accuracy(%__MODULE__{} = stats) do
    stats.correct / stats.predictions * 100.0
  end

  @doc """
  Misprediction rate as a percentage (0.0 to 100.0).

  This is the complement of accuracy: misprediction_rate = 100 - accuracy.
  CPU architects often think in terms of misprediction rate because each
  misprediction causes a pipeline flush — a concrete, measurable cost.

  ## Examples

      iex> stats = %CodingAdventures.BranchPredictor.Stats{predictions: 100, correct: 87, incorrect: 13}
      iex> CodingAdventures.BranchPredictor.Stats.misprediction_rate(stats)
      13.0
  """
  def misprediction_rate(%__MODULE__{predictions: 0}), do: 0.0

  def misprediction_rate(%__MODULE__{} = stats) do
    stats.incorrect / stats.predictions * 100.0
  end

  @doc """
  Reset all counters to zero.

  Called when starting a new benchmark or program execution. Without
  this, stats from a previous run would contaminate the new measurement.

  ## Examples

      iex> stats = %CodingAdventures.BranchPredictor.Stats{predictions: 50, correct: 40, incorrect: 10}
      iex> stats = CodingAdventures.BranchPredictor.Stats.reset(stats)
      iex> stats.predictions
      0
  """
  def reset(%__MODULE__{}) do
    %__MODULE__{}
  end
end
