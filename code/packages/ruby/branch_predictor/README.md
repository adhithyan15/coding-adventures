# Branch Predictor (Ruby)

Branch prediction simulators built from first principles -- a Ruby port of the Python `branch-predictor` package.

## What It Does

In CPU design, the branch predictor sits at the front of the pipeline and guesses whether each branch instruction will be taken or not. This package implements several predictor strategies as educational simulators:

- **AlwaysTakenPredictor** -- static, always predicts "taken" (~60-70% accurate)
- **AlwaysNotTakenPredictor** -- static, always predicts "not taken" (~30-40%)
- **BackwardTakenForwardNotTaken** -- static direction heuristic (~65-75%)
- **OneBitPredictor** -- dynamic, 1-bit per branch (learns last outcome)
- **TwoBitPredictor** -- dynamic, 2-bit saturating counter (classic textbook)
- **BranchTargetBuffer** -- caches WHERE branches go (target addresses)

## How It Fits in the Stack

This is a standalone package with no dependencies on other coding-adventures gems. It models the branch prediction unit that sits in the fetch stage of the CPU pipeline.

## Usage

```ruby
require "coding_adventures_branch_predictor"

predictor = CodingAdventures::BranchPredictor::TwoBitPredictor.new(table_size: 1024)

# Simulate a branch at PC 0x100
pred = predictor.predict(pc: 0x100)
puts pred.taken?      # => false (cold start)

# Feed back the actual outcome
predictor.update(pc: 0x100, taken: true)

# Now it predicts taken
pred = predictor.predict(pc: 0x100)
puts pred.taken?      # => true

# Check accuracy
puts predictor.stats.accuracy  # => percentage
```

## Running Tests

```bash
bundle install
bundle exec rake test
```
