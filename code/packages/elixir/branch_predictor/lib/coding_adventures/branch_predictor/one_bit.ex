defmodule CodingAdventures.BranchPredictor.OneBit do
  @moduledoc """
  One-bit branch predictor — one flip-flop per branch.

  ## How it works

  The one-bit predictor is the simplest dynamic predictor. Unlike static
  predictors (AlwaysTaken, BTFNT), it actually learns from the branch's
  history. Each branch address maps to a single bit of state that records
  the last outcome:

      bit = false -> predict NOT TAKEN
      bit = true  -> predict TAKEN

  After each branch resolves, the bit is updated to match the actual outcome.
  This means the predictor always predicts "whatever happened last time."

  ## The DFA formalism

  This predictor IS a Deterministic Finite Automaton. The DFA has just two
  states ("not_taken" and "taken") and two input symbols ("taken" and
  "not_taken"). On every branch outcome, the machine transitions to the
  state matching the actual outcome.

  The accepting state {"taken"} represents "predicts taken." In the DFA
  formalism, processing a sequence of branch outcomes and checking acceptance
  tells you what the predictor would predict for the NEXT branch.

      State diagram:

          +-----------+     taken      +-----------+
          | not_taken | ------------> |   taken   |
          | (predict  | <------------ | (predict  |
          | not taken)|  not_taken    |   taken)  |
          +-----------+               +-----------+
              |   ^                       |   ^
              |   |                       |   |
              +---+                       +---+
            not_taken                     taken

  ## Hardware implementation

  A small SRAM table indexed by the lower bits of the PC.
  Each entry is a single flip-flop (1 bit of storage).
  Total storage: table_size x 1 bit.
  For a 1024-entry table: 1024 bits = 128 bytes.

  ## The aliasing problem

  Since the table is indexed by `rem(pc, table_size)`, two different branches
  can map to the same entry. This is called "aliasing" or "interference."
  When branches alias, they corrupt each other's predictions.

  ## The double-misprediction problem

  Consider a loop that runs 10 times:

      Iteration 1:  bit=false -> predict NOT TAKEN -> actual TAKEN -> WRONG, set bit=true
      Iteration 2:  bit=true  -> predict TAKEN     -> actual TAKEN -> correct
      ...
      Iteration 9:  bit=true  -> predict TAKEN     -> actual TAKEN -> correct
      Iteration 10: bit=true  -> predict TAKEN     -> actual NOT TAKEN -> WRONG, set bit=false

      Next loop invocation:
      Iteration 1:  bit=false -> predict NOT TAKEN -> actual TAKEN -> WRONG!

  Result: 2 mispredictions per loop invocation. The two-bit predictor solves
  this by requiring TWO consecutive mispredictions to flip the prediction.
  """

  alias CodingAdventures.BranchPredictor.{Prediction, Stats}
  alias CodingAdventures.StateMachine.DFA

  # ── One-Bit DFA ────────────────────────────────────────────────────────────
  #
  # This IS the formal state machine that the 1-bit predictor implements.
  # We build it at compile time via a module attribute. The DFA.new/5 call
  # returns {:ok, dfa}, which we unwrap immediately since the definition
  # is known-valid at compile time.
  #
  # The ONE_BIT_DFA captures the complete transition logic:
  #   - Two states: "not_taken" (predicts not taken) and "taken" (predicts taken)
  #   - Two inputs: "taken" (branch was taken) and "not_taken" (branch was not taken)
  #   - Transitions: the machine always moves to the state matching the input
  #   - Accepting: {"taken"} — the state that predicts taken

  @one_bit_dfa (case DFA.new(
                   MapSet.new(["not_taken", "taken"]),
                   MapSet.new(["taken", "not_taken"]),
                   %{
                     {"not_taken", "taken"} => "taken",
                     {"not_taken", "not_taken"} => "not_taken",
                     {"taken", "taken"} => "taken",
                     {"taken", "not_taken"} => "not_taken"
                   },
                   "not_taken",
                   MapSet.new(["taken"])
                 ) do
                   {:ok, dfa} -> dfa
                 end)

  defstruct table_size: 1024, table: %{}, stats: %Stats{}

  @type t :: %__MODULE__{
          table_size: pos_integer(),
          table: %{non_neg_integer() => boolean()},
          stats: Stats.t()
        }

  @doc """
  Create a new one-bit predictor.

  ## Options

  - `:table_size` — number of entries in the prediction table (default: 1024).
    Must be a positive integer. In hardware, this would be a power of 2 for
    efficient address decoding, but we don't enforce that here.

  ## Examples

      iex> p = CodingAdventures.BranchPredictor.OneBit.new()
      iex> p.table_size
      1024

      iex> p = CodingAdventures.BranchPredictor.OneBit.new(table_size: 256)
      iex> p.table_size
      256
  """
  def new(opts \\ []) do
    table_size = Keyword.get(opts, :table_size, 1024)
    %__MODULE__{table_size: table_size}
  end

  @doc """
  Predict based on the last outcome of this branch.

  On a cold start (branch not yet seen), defaults to NOT TAKEN.
  This is a common design choice — the bit starts at 0 (false).

  Returns `{prediction, predictor}`.

  ## Examples

      iex> p = CodingAdventures.BranchPredictor.OneBit.new()
      iex> {pred, _p} = CodingAdventures.BranchPredictor.OneBit.predict(p, 0x100)
      iex> pred.predicted_taken
      false
  """
  def predict(%__MODULE__{} = predictor, pc) do
    index = index(predictor, pc)
    taken = Map.get(predictor.table, index, false)
    {%Prediction{predicted_taken: taken, confidence: 0.5}, predictor}
  end

  @doc """
  Update the prediction table with the actual outcome.

  Uses the DFA transition table to compute the next state. The DFA event
  is "taken" or "not_taken", and the resulting state (also "taken" or
  "not_taken") becomes the new prediction bit.

  Records accuracy BEFORE updating the table, so we compare against what
  the predictor would have predicted (not what it will predict next).

  Returns a new predictor struct with updated table and stats.

  ## Examples

      iex> p = CodingAdventures.BranchPredictor.OneBit.new()
      iex> p = CodingAdventures.BranchPredictor.OneBit.update(p, 0x100, true)
      iex> {pred, _p} = CodingAdventures.BranchPredictor.OneBit.predict(p, 0x100)
      iex> pred.predicted_taken
      true
  """
  def update(%__MODULE__{} = predictor, pc, taken, _target \\ nil) do
    index = index(predictor, pc)
    predicted = Map.get(predictor.table, index, false)

    # Record accuracy BEFORE updating
    new_stats = Stats.record(predictor.stats, predicted == taken)

    # Use the DFA transition table to compute the next state
    current_dfa_state = if predicted, do: "taken", else: "not_taken"
    event = if taken, do: "taken", else: "not_taken"
    next_dfa_state = @one_bit_dfa.transitions[{current_dfa_state, event}]
    new_bit = next_dfa_state == "taken"

    %{predictor | table: Map.put(predictor.table, index, new_bit), stats: new_stats}
  end

  @doc "Get prediction accuracy statistics."
  def stats(%__MODULE__{stats: stats}), do: stats

  @doc "Reset the prediction table and statistics."
  def reset(%__MODULE__{} = predictor) do
    %{predictor | table: %{}, stats: %Stats{}}
  end

  @doc """
  Return the DFA that formally defines this predictor's state transitions.

  The one-bit predictor is a 2-state DFA:
  - States: {"not_taken", "taken"}
  - Alphabet: {"taken", "not_taken"}
  - Accepting: {"taken"} (states that predict taken)

  This DFA can be visualized, verified, and traced using the state_machine
  library's tools (to_dot, validate, process_sequence, etc.).
  """
  def dfa, do: @one_bit_dfa

  # ── Private helpers ──────────────────────────────────────────────────────

  defp index(%__MODULE__{table_size: table_size}, pc) do
    rem(pc, table_size)
  end
end
