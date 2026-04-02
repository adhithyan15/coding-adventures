# coding-adventures-branch-predictor (Lua)

Branch prediction algorithms for CPU pipeline simulation. This package
implements the classic hierarchy of branch predictors, from simple static
strategies to the 2-bit saturating counter used in real processors.

## What is branch prediction?

A branch predictor guesses the outcome of a branch instruction (if/else,
loop, function return) before the branch condition is evaluated. In a
pipelined CPU, a wrong guess wastes several cycles (pipeline flush). A
95% accurate predictor turns a potential 11-cycle penalty into a ~0.5-cycle
average cost.

## Predictors

| Predictor       | Accuracy | State per branch |
|-----------------|----------|-----------------|
| AlwaysNotTaken  | ~40%     | 0 bits          |
| AlwaysTaken     | ~60%     | 0 bits          |
| BTFNT           | ~70%     | 0 bits + target |
| OneBit          | ~80%     | 1 bit           |
| TwoBit          | ~90%     | 2 bits          |

The Branch Target Buffer (BTB) provides the predicted target address
alongside any direction predictor.

## Installation

```
luarocks make --local coding-adventures-branch-predictor-0.1.0-1.rockspec
```

## Usage

```lua
local bp = require("coding_adventures.branch_predictor")

-- 2-bit saturating counter predictor
local pred = bp.TwoBit.new()

-- Predict a branch
local prediction, pred = pred:predict(0x100)
print(prediction.predicted_taken)  -- false (cold start = WNT)

-- Update after actual outcome
pred = pred:update(0x100, true)    -- branch was taken

-- Check accuracy
print(pred:get_stats():accuracy()) -- 0.0% (first was wrong)

-- BTB for target address caching
local btb = bp.BTB.new(256)
btb = btb:update(0x100, 0x50)     -- branch at 0x100 goes to 0x50
local target, btb = btb:lookup(0x100)
print(target)                      -- 0x50
```

## Layer position

```
D02 — Branch Predictor
  Used by: Pipeline (D04), Core (D05)
  Depends on: (no sibling deps — all algorithms implemented from scratch)
```
