defmodule CodingAdventures.BranchPredictor.TwoBit do
  @moduledoc """
  Two-bit saturating counter predictor — the classic, used in most textbooks.

  ## The four states

  The two-bit predictor improves on the one-bit predictor by adding hysteresis.
  Instead of flipping the prediction on every misprediction, it takes TWO
  consecutive mispredictions to change the predicted direction. This is achieved
  with a 2-bit saturating counter:

      STRONGLY NOT TAKEN (SNT) <-> WEAKLY NOT TAKEN (WNT) <-> WEAKLY TAKEN (WT) <-> STRONGLY TAKEN (ST)
            (00)                        (01)                       (10)                    (11)

  On "taken" outcome: move right (increment, saturate at ST)
  On "not taken" outcome: move left (decrement, saturate at SNT)

  The prediction threshold is at the midpoint:
  - States SNT, WNT -> predict NOT TAKEN
  - States WT, ST   -> predict TAKEN

  ## The DFA formalism

  Like the one-bit predictor, the two-bit predictor is formally defined as
  a DFA. The four states are {"SNT", "WNT", "WT", "ST"} and the alphabet
  is {"taken", "not_taken"}. The accepting states {"WT", "ST"} are the states
  that predict "taken".

  All transition logic is delegated to the DFA's transition table, making
  the formal specification the single source of truth.

  ## Why hysteresis works

  Consider a loop that runs 10 times:

      First invocation:
      Iter 1:  state=WNT -> predict NOT TAKEN -> actual TAKEN -> WRONG, state->WT
      Iter 2:  state=WT  -> predict TAKEN     -> actual TAKEN -> correct, state->ST
      ...
      Iter 9:  state=ST  -> predict TAKEN     -> actual TAKEN -> correct (saturated)
      Iter 10: state=ST  -> predict TAKEN     -> actual NOT TAKEN -> WRONG, state->WT

      Second invocation:
      Iter 1:  state=WT  -> predict TAKEN     -> actual TAKEN -> correct! state->ST

  Only 1 misprediction on re-entry (vs 2 for the one-bit predictor). The
  "weakly taken" state acts as a buffer.

  ## Historical usage

  - Alpha 21064: 2-bit counters with 2048 entries
  - Intel Pentium: 2-bit counters with 256 entries
  - Early ARM (ARM7): 2-bit counters with 64 entries
  - MIPS R10000: 2-bit counters as base predictor in tournament scheme

  ## Confidence mapping

  The strong/weak distinction maps naturally to confidence:
  - STRONGLY states -> 1.0 (high confidence)
  - WEAKLY states   -> 0.5 (low confidence)

  This is useful for tournament predictors that pick the most confident
  sub-predictor for each branch.
  """

  alias CodingAdventures.BranchPredictor.{Prediction, Stats}
  alias CodingAdventures.StateMachine.DFA

  # ── State name constants ──────────────────────────────────────────────────
  #
  # We use short abbreviations matching the Python implementation. These are
  # the state names in the DFA's formal definition.

  @snt "SNT"
  @wnt "WNT"
  @wt "WT"
  @st "ST"

  # ── Two-Bit DFA ──────────────────────────────────────────────────────────
  #
  # The formal state machine for the 2-bit saturating counter. Every
  # (state, input) pair maps to exactly one next state. The accepting
  # states {WT, ST} are the states that predict "taken."
  #
  # The transition table encodes the saturating counter logic:
  #   - "taken" input: move right (increment) on the state diagram
  #   - "not_taken" input: move left (decrement) on the state diagram
  #   - Saturation: ST + taken -> ST (can't go higher), SNT + not_taken -> SNT

  @two_bit_dfa (case DFA.new(
                   MapSet.new([@snt, @wnt, @wt, @st]),
                   MapSet.new(["taken", "not_taken"]),
                   %{
                     {@snt, "taken"} => @wnt,
                     {@snt, "not_taken"} => @snt,
                     {@wnt, "taken"} => @wt,
                     {@wnt, "not_taken"} => @snt,
                     {@wt, "taken"} => @st,
                     {@wt, "not_taken"} => @wnt,
                     {@st, "taken"} => @st,
                     {@st, "not_taken"} => @wt
                   },
                   @wnt,
                   MapSet.new([@wt, @st])
                 ) do
                   {:ok, dfa} -> dfa
                 end)

  defstruct table_size: 1024, initial_state: "WNT", table: %{}, stats: %Stats{}

  @type t :: %__MODULE__{
          table_size: pos_integer(),
          initial_state: String.t(),
          table: %{non_neg_integer() => String.t()},
          stats: Stats.t()
        }

  @doc """
  Create a new two-bit predictor.

  ## Options

  - `:table_size` — number of entries in the prediction table (default: 1024).
  - `:initial_state` — starting state for all counter entries (default: "WNT").
    Common choices:
    - "WNT" (Weakly Not Taken): conservative, requires 1 taken to flip
    - "WT" (Weakly Taken): optimistic, starts predicting taken

  ## Examples

      iex> p = CodingAdventures.BranchPredictor.TwoBit.new()
      iex> p.table_size
      1024
      iex> p.initial_state
      "WNT"

      iex> p = CodingAdventures.BranchPredictor.TwoBit.new(table_size: 256, initial_state: "WT")
      iex> p.initial_state
      "WT"
  """
  def new(opts \\ []) do
    table_size = Keyword.get(opts, :table_size, 1024)
    initial_state = Keyword.get(opts, :initial_state, @wnt)

    %__MODULE__{table_size: table_size, initial_state: initial_state}
  end

  @doc """
  Predict based on the 2-bit counter for this branch.

  Reads the counter state and returns taken/not-taken based on the
  threshold (WT, ST -> taken; SNT, WNT -> not-taken).

  Returns `{prediction, predictor}`.

  ## Examples

      iex> p = CodingAdventures.BranchPredictor.TwoBit.new()
      iex> {pred, _p} = CodingAdventures.BranchPredictor.TwoBit.predict(p, 0x100)
      iex> pred.predicted_taken
      false
  """
  def predict(%__MODULE__{} = predictor, pc) do
    index = index(predictor, pc)
    state = get_state(predictor, index)
    taken = predicts_taken?(state)

    confidence =
      if state in [@st, @snt] do
        1.0
      else
        0.5
      end

    {%Prediction{predicted_taken: taken, confidence: confidence}, predictor}
  end

  @doc """
  Update the 2-bit counter based on the actual outcome.

  Increments on taken, decrements on not-taken, saturating at boundaries.
  Records accuracy BEFORE updating to compare against the actual prediction.

  Returns a new predictor struct with updated table and stats.

  ## Examples

      iex> p = CodingAdventures.BranchPredictor.TwoBit.new()
      iex> p = CodingAdventures.BranchPredictor.TwoBit.update(p, 0x100, true)
      iex> {pred, _p} = CodingAdventures.BranchPredictor.TwoBit.predict(p, 0x100)
      iex> pred.predicted_taken
      true
  """
  def update(%__MODULE__{} = predictor, pc, taken, _target \\ nil) do
    index = index(predictor, pc)
    state = get_state(predictor, index)

    # Record accuracy BEFORE updating
    new_stats = Stats.record(predictor.stats, predicts_taken?(state) == taken)

    # Transition the state using the DFA
    next_state =
      if taken do
        taken_outcome(state)
      else
        not_taken_outcome(state)
      end

    %{predictor | table: Map.put(predictor.table, index, next_state), stats: new_stats}
  end

  @doc "Get prediction accuracy statistics."
  def stats(%__MODULE__{stats: stats}), do: stats

  @doc "Reset the prediction table and statistics."
  def reset(%__MODULE__{} = predictor) do
    %{predictor | table: %{}, stats: %Stats{}}
  end

  @doc """
  Compute the next state on a "taken" branch outcome.

  Delegates to the DFA transition table. This is the formal "increment"
  operation on the saturating counter.

  ## Examples

      iex> CodingAdventures.BranchPredictor.TwoBit.taken_outcome("SNT")
      "WNT"
      iex> CodingAdventures.BranchPredictor.TwoBit.taken_outcome("ST")
      "ST"
  """
  def taken_outcome(state) do
    @two_bit_dfa.transitions[{state, "taken"}]
  end

  @doc """
  Compute the next state on a "not_taken" branch outcome.

  Delegates to the DFA transition table. This is the formal "decrement"
  operation on the saturating counter.

  ## Examples

      iex> CodingAdventures.BranchPredictor.TwoBit.not_taken_outcome("ST")
      "WT"
      iex> CodingAdventures.BranchPredictor.TwoBit.not_taken_outcome("SNT")
      "SNT"
  """
  def not_taken_outcome(state) do
    @two_bit_dfa.transitions[{state, "not_taken"}]
  end

  @doc """
  Check if a state predicts "taken".

  In the DFA formalism, the accepting states are the ones that predict taken.
  States WT and ST are accepting; SNT and WNT are not.

  In hardware, this is just bit 1 of the 2-bit counter — a single wire,
  zero logic gates.

  ## Examples

      iex> CodingAdventures.BranchPredictor.TwoBit.predicts_taken?("ST")
      true
      iex> CodingAdventures.BranchPredictor.TwoBit.predicts_taken?("WT")
      true
      iex> CodingAdventures.BranchPredictor.TwoBit.predicts_taken?("WNT")
      false
      iex> CodingAdventures.BranchPredictor.TwoBit.predicts_taken?("SNT")
      false
  """
  def predicts_taken?(state) do
    MapSet.member?(@two_bit_dfa.accepting, state)
  end

  @doc """
  Return the DFA that formally defines this predictor's state transitions.

  The two-bit predictor is a 4-state DFA:
  - States: {"SNT", "WNT", "WT", "ST"}
  - Alphabet: {"taken", "not_taken"}
  - Accepting: {"WT", "ST"} (states that predict taken)
  - Initial: "WNT"

  This DFA can be visualized, verified, and traced using the state_machine
  library's tools.
  """
  def dfa, do: @two_bit_dfa

  @doc """
  Inspect the current state for a branch address (for testing/debugging).

  Returns the DFA state name (one of "SNT", "WNT", "WT", "ST") for the
  table entry that the given PC maps to.
  """
  def get_state_for_pc(%__MODULE__{} = predictor, pc) do
    get_state(predictor, index(predictor, pc))
  end

  # ── Private helpers ──────────────────────────────────────────────────────

  defp index(%__MODULE__{table_size: table_size}, pc) do
    rem(pc, table_size)
  end

  defp get_state(%__MODULE__{table: table, initial_state: initial}, index) do
    Map.get(table, index, initial)
  end
end
