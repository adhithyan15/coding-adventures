defmodule CodingAdventures.BranchPredictor do
  @moduledoc """
  Branch prediction — guessing the future to keep the CPU pipeline full.

  ## What is branch prediction?

  Modern CPUs use deep pipelines (15-20 stages) to achieve high throughput.
  Each instruction spends one cycle per stage, and multiple instructions are
  in-flight simultaneously. But branches (if/else, loops, function calls)
  create a problem: the CPU doesn't know WHERE to fetch the next instruction
  until the branch resolves, many stages later.

  Without prediction, the CPU would stall for 10+ cycles on every branch.
  Since ~20% of instructions are branches, this would destroy performance.

  The solution: GUESS. The branch predictor sits in the fetch stage and makes
  two predictions before the branch even decodes:

    1. Direction: will this branch be TAKEN or NOT TAKEN?
    2. Target: if taken, WHERE does it go? (via the Branch Target Buffer)

  If the guess is correct (90-99% of the time on modern CPUs), there's zero
  cost. If wrong, the pipeline flushes and restarts — a 10-15 cycle penalty.

  ## The math

  Without prediction:
    20% branches x 10 cycle stall = 2.0 cycles/instruction penalty

  With 95% accurate prediction:
    20% branches x 5% miss rate x 15 cycle flush = 0.15 cycles/instruction

  That's a 13x reduction in branch-related penalties!

  ## Predictors in this package

  From simplest to most sophisticated:

  - `CodingAdventures.BranchPredictor.Static` — zero-hardware predictors:
    - `AlwaysTaken` — always predicts taken (~60% accurate)
    - `AlwaysNotTaken` — always predicts not taken (~35% accurate)
    - `BTFNT` — backward taken, forward not taken (~70% accurate)

  - `CodingAdventures.BranchPredictor.OneBit` — one flip-flop per branch.
    Predicts "whatever happened last time." Simple but suffers from the
    double-misprediction problem on loops.

  - `CodingAdventures.BranchPredictor.TwoBit` — two-bit saturating counter.
    The classic textbook predictor. Adds hysteresis so a single anomalous
    outcome doesn't flip the prediction. Used in Alpha 21064, early ARM.

  - `CodingAdventures.BranchPredictor.BTB` — Branch Target Buffer.
    Caches WHERE branches go, complementing the direction predictors above.

  ## DFA integration

  The one-bit and two-bit predictors are formally defined as Deterministic
  Finite Automata using the `CodingAdventures.StateMachine.DFA` module.
  Each predictor's state transition logic is captured in a DFA, making the
  state machine specification the single source of truth. This means:

  - The transition tables can be visualized with Graphviz
  - The DFA can be formally verified (completeness, reachability)
  - Every transition is traceable through the DFA's execution log

  ## Immutability

  All predictors in this Elixir implementation are immutable structs.
  Every operation (predict, update, reset) returns a NEW struct rather than
  mutating in place. This is a natural fit for the functional paradigm and
  makes it easy to snapshot predictor state at any point.
  """

  alias CodingAdventures.BranchPredictor.{BTB, OneBit, Prediction, Static, Stats, TwoBit}

  # Re-export key types for convenience
  defdelegate new_prediction(attrs), to: Prediction, as: :new
  defdelegate new_stats(), to: Stats, as: :new
  defdelegate new_one_bit(opts \\ []), to: OneBit, as: :new
  defdelegate new_two_bit(opts \\ []), to: TwoBit, as: :new
  defdelegate new_btb(opts \\ []), to: BTB, as: :new
  defdelegate new_always_taken(), to: Static.AlwaysTaken, as: :new
  defdelegate new_always_not_taken(), to: Static.AlwaysNotTaken, as: :new
  defdelegate new_btfnt(), to: Static.BTFNT, as: :new
end
