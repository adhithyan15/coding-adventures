# Branch Predictor (Elixir)

Branch prediction algorithms for CPU pipeline simulation — teaching CPUs to guess the future.

## What is this?

This package simulates the branch prediction algorithms used in real CPU cores. Branch prediction is one of the most critical performance features in modern processors. Without it, a deeply pipelined CPU (15-20 stages) would stall on every branch instruction, losing 10-15 cycles each time.

This is an Elixir port of the Python `branch-predictor` package, following the same architecture but leveraging Elixir's immutable data structures for a naturally functional design.

## How it fits in the stack

```
Layer 8: Programs (CPU simulator, assembler)
Layer 7: Branch Predictor  <-- you are here
Layer 6: State Machine (DFA, NFA, PDA)
Layer 5: Logic Gates, ALU, Memory
```

The branch predictor depends on the `state_machine` package because the one-bit and two-bit predictors are formally defined as Deterministic Finite Automata (DFAs). The DFA transition tables are the single source of truth for state transitions.

## Predictors

| Predictor | Accuracy | Hardware Cost | Description |
|-----------|----------|---------------|-------------|
| AlwaysTaken | ~60% | Zero | Always predicts taken |
| AlwaysNotTaken | ~35% | Zero | Always predicts not taken |
| BTFNT | ~70% | Minimal | Backward=taken, forward=not-taken |
| OneBit | ~80% | 1 bit/entry | Predicts last outcome |
| TwoBit | ~90% | 2 bits/entry | Saturating counter with hysteresis |

## Usage

```elixir
alias CodingAdventures.BranchPredictor.TwoBit
alias CodingAdventures.BranchPredictor.BTB
alias CodingAdventures.BranchPredictor.Stats

# Create a two-bit predictor
p = TwoBit.new(table_size: 1024)

# Predict a branch
{prediction, p} = TwoBit.predict(p, 0x100)
# => %Prediction{predicted_taken: false, confidence: 0.5}

# Update with actual outcome
p = TwoBit.update(p, 0x100, true)

# Check accuracy
stats = TwoBit.stats(p)
Stats.accuracy(stats)
# => 0.0 (first prediction was wrong)

# Branch Target Buffer
btb = BTB.new(size: 256)
btb = BTB.update(btb, 0x100, 0x200, "conditional")
{target, btb} = BTB.lookup(btb, 0x100)
# => {0x200, btb}
```

## DFA Integration

The one-bit and two-bit predictors expose their formal DFA definitions:

```elixir
dfa = TwoBit.dfa()
# => %DFA{states: MapSet.new(["SNT", "WNT", "WT", "ST"]), ...}

# Visualize with Graphviz
CodingAdventures.StateMachine.DFA.to_dot(dfa)
```

## Immutability

All predictors are immutable structs. Every operation returns a new struct:

```elixir
p1 = TwoBit.new()
p2 = TwoBit.update(p1, 0x100, true)
# p1 is unchanged, p2 has the update
```

## Running tests

```bash
mix deps.get
mix test --cover
```

## Dependencies

- `coding_adventures_state_machine` — provides the DFA implementation used by one-bit and two-bit predictors
