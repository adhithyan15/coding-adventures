defmodule CodingAdventures.BranchPredictor.Static.BTFNT do
  @moduledoc """
  Backward Taken, Forward Not Taken — a direction-based heuristic.

  ## The insight

  The direction of a branch (forward vs backward) is a strong signal:

  - **Backward branches** (target < pc) are almost always loop back-edges.
    Loops iterate many times, so the back-edge is taken on every iteration
    except the last. Predicting "taken" is correct ~95% of the time for loops.

  - **Forward branches** (target > pc) are usually if-then-else constructs.
    The "then" case often falls through (not taken), especially for error
    checking: `if error: handle_error()`. Predicting "not taken" is correct
    ~60% of the time for if-else.

  - **Equal** (target == pc) is a degenerate infinite loop. Predict taken.

  ## Cold start problem

  BTFNT needs to know the branch target to determine direction. On the first
  encounter of a branch, we don't know the target yet. The predictor defaults
  to "not taken" (the safe fallback) and stores the target from the update
  call for future predictions.

  ## Historical usage

  - MIPS R4000 (1991): used BTFNT as the primary prediction strategy
  - SPARC V8: used BTFNT with a branch annulling mechanism
  - Early ARM processors: used BTFNT before adding dynamic predictors

  ## How the pipeline uses BTFNT

  In a real CPU, BTFNT requires knowing the branch target at prediction time.
  For direct branches, the target is encoded in the instruction, so it's
  available after decode. For indirect branches, the target is in a register,
  so it's not known until execute. This means BTFNT only works well for
  direct branches — indirect branches still need a BTB.

  The BTFNT predictor remembers the last known target for each branch address,
  so on subsequent encounters, it can predict without waiting for decode.

  ## Examples

      iex> alias CodingAdventures.BranchPredictor.Static.BTFNT
      iex> p = BTFNT.new()
      iex> # Cold start — no target known yet, defaults to not-taken
      iex> {pred, _p} = BTFNT.predict(p, 0x108)
      iex> pred.predicted_taken
      false
  """

  alias CodingAdventures.BranchPredictor.{Prediction, Stats}

  defstruct targets: %{}, stats: %Stats{}

  @type t :: %__MODULE__{
          targets: %{non_neg_integer() => non_neg_integer()},
          stats: Stats.t()
        }

  @doc "Create a new BTFNT predictor."
  def new, do: %__MODULE__{}

  @doc """
  Predict based on branch direction: backward=taken, forward=not-taken.

  If we haven't seen this branch before (no known target), we default
  to NOT taken — the safe choice that doesn't require a target address.

  Returns `{prediction, predictor}`.
  """
  def predict(%__MODULE__{} = predictor, pc) do
    case Map.get(predictor.targets, pc) do
      nil ->
        # Cold start — we don't know the target direction yet.
        {%Prediction{predicted_taken: false, confidence: 0.0}, predictor}

      target ->
        # Backward branch (target <= pc) -> taken (loop back-edge)
        # Forward branch (target > pc)   -> not taken (if-else)
        taken = target <= pc
        {%Prediction{predicted_taken: taken, confidence: 0.5, address: target}, predictor}
    end
  end

  @doc """
  Record the branch outcome and learn the target address.

  The key learning here is remembering the target address for future
  predictions. The BTFNT predictor doesn't adapt its strategy — it
  always uses the direction heuristic — but it needs to know the target.
  """
  def update(%__MODULE__{} = predictor, pc, taken, target \\ nil) do
    # Store the target so we can use it for future direction-based predictions
    new_targets =
      if target != nil do
        Map.put(predictor.targets, pc, target)
      else
        predictor.targets
      end

    # Determine what we would have predicted
    known_target = Map.get(new_targets, pc)

    predicted_taken =
      case known_target do
        nil -> false
        t -> t <= pc
      end

    new_stats = Stats.record(predictor.stats, predicted_taken == taken)
    %{predictor | targets: new_targets, stats: new_stats}
  end

  @doc "Get prediction accuracy statistics."
  def stats(%__MODULE__{stats: stats}), do: stats

  @doc "Reset all state — target cache and statistics."
  def reset(%__MODULE__{}), do: %__MODULE__{}
end
