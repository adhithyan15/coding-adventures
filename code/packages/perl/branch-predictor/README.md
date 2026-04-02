# CodingAdventures::BranchPredictor

Branch prediction algorithms implemented in Perl — a Perl port of the Elixir
`CodingAdventures.BranchPredictor` package.

## What is branch prediction?

In a pipelined CPU, the processor must decide which instruction to fetch BEFORE
it knows where the current branch goes. A wrong guess causes a **pipeline flush**:
several cycles of partially-executed work are discarded. On a 13-stage pipeline
(e.g., ARM Cortex-A78), one misprediction wastes ~11 cycles.

At 3 GHz with 20% branch frequency and 5% misprediction rate, that is roughly
33 million wasted cycles per second — visible as measurable slowdowns in tight loops.

## Predictors

| Predictor       | Accuracy | State per branch |
|-----------------|----------|-----------------|
| AlwaysNotTaken  | ~40%     | none            |
| AlwaysTaken     | ~60%     | none            |
| BTFNT           | ~70%     | 1 target address |
| OneBit          | ~80%     | 1 bit           |
| TwoBit          | ~90%     | 2 bits          |

### Static predictors (`Static.pm`)

- **AlwaysTaken** — predicts every branch taken. Works well for loops (which usually
  iterate many times before exiting).
- **AlwaysNotTaken** — predicts every branch not taken. Works well for rare error paths.
- **BTFNT** (Backward Taken, Forward Not Taken) — exploits code structure: backward
  branches are loop back-edges (usually taken), forward branches are `if`-bodies
  (often skipped). Caches the last-known target to determine direction.

### OneBit (`OneBit.pm`)

Remembers the last outcome per branch address. "Predict whatever happened last time."
Fast to learn but suffers **two mispredictions per loop invocation** — one when the
loop exits and one when it re-enters.

### TwoBit (`TwoBit.pm`)

A saturating 2-bit counter with four states:

```
SNT (Strongly Not Taken) → predict NOT TAKEN
WNT (Weakly Not Taken)   → predict NOT TAKEN
WT  (Weakly Taken)       → predict TAKEN
ST  (Strongly Taken)     → predict TAKEN
```

After a loop exits (not taken), the counter drops from ST → WT. On the next loop
entry, WT still predicts TAKEN — no extra misprediction. This **hysteresis** is why
2-bit beats 1-bit for looping code. Used in the Intel Pentium and Alpha 21064.

### BTB — Branch Target Buffer (`BTB.pm`)

Answers "WHERE does the branch go?" while the direction predictor answers "WILL it
be taken?". Without a BTB, even a perfect direction predictor needs an extra cycle
to compute the target. The BTB is a direct-mapped cache:

```
index = pc % size
hit   = index has entry AND entry.tag == pc
```

## Usage

```perl
use CodingAdventures::BranchPredictor;

# Two-bit predictor
my $p = CodingAdventures::BranchPredictor::TwoBit->new();

# Simulate a branch at PC 0x100 taken 10 times
for my $i (1..10) {
    my ($pred, $new_p) = $p->predict(0x100);
    $p = $new_p->update(0x100, 1);   # actually taken
}
printf "Accuracy: %.1f%%\n", $p->get_stats->accuracy;

# Branch Target Buffer
my $btb = CodingAdventures::BranchPredictor::BTB->new(size => 256);
$btb = $btb->update(0x100, 0x200, 'conditional');
my ($target, $btb2) = $btb->lookup(0x100);  # returns 0x200
printf "Hit rate: %.1f%%\n", $btb2->hit_rate;
```

## Module structure

```
CodingAdventures::BranchPredictor
├── Stats       — prediction accuracy tracking
├── Prediction  — immutable result type
├── Static
│   ├── AlwaysTaken
│   ├── AlwaysNotTaken
│   └── BTFNT
├── OneBit      — 1-bit predictor
├── TwoBit      — 2-bit saturating counter
└── BTB         — branch target buffer
```

## Installation

```
cpanm .
```

## Running tests

```
prove -l -v t/
```
