# Pipelines, Caches, And Speculation

If the basic CPU story is:

```text
fetch -> decode -> execute
```

the architecture story is:

```text
how do we keep that process busy enough to be fast?
```

This document is the bridge between a simple CPU simulator and the deeper architecture packages in the repository.

Relevant packages:

- `cache`
- `branch-predictor`
- `hazard-detection`
- `pipeline`
- `core`

## Why Modern Performance Is Hard

A naive processor can execute instructions correctly while still being slow for three big reasons:

1. it waits on memory too often
2. it wastes work when branches change control flow
3. it runs into data dependencies between neighboring instructions

The architecture packages in this repository exist to study those three problems.

## Pipelines

Pipelining is the idea of overlapping instruction execution.

Instead of finishing one instruction completely before starting the next, we divide instruction handling into stages.

Classic teaching example:

```text
IF  = instruction fetch
ID  = instruction decode
EX  = execute
MEM = memory access
WB  = write back
```

Then multiple instructions can be in flight at once:

```text
Cycle 1: I1 in IF
Cycle 2: I1 in ID, I2 in IF
Cycle 3: I1 in EX, I2 in ID, I3 in IF
```

### What pipelining improves

It improves **throughput**.

That means:

- more instructions completed per unit time

It does **not** automatically reduce the latency of one instruction.

### Why pipelines complicate everything

Because overlapping work means instructions can interfere with each other.

That leads to hazards.

## Hazards

Relevant package:

- `hazard-detection`

Hazards are situations where the straightforward pipeline flow would produce the wrong result or require unavailable hardware.

### Data hazards

Instruction B needs a value that instruction A has not finished producing yet.

Example:

```text
ADD R1, R2, R3
SUB R4, R1, R5
```

The second instruction wants `R1`, but the first instruction may not have written it back yet.

### Control hazards

The processor fetches instructions before it knows whether a branch changes control flow.

Example:

```text
BEQ R1, R2, target
next instruction
next next instruction
```

If the branch is taken, the already-fetched sequential instructions were the wrong guess.

### Structural hazards

Two operations want the same hardware resource at the same time.

This is less emphasized in simple teaching designs when resources are duplicated, but it is still conceptually important.

## Forwarding

One response to data hazards is **forwarding**, also called bypassing.

The key insight is:

the result may already exist in a pipeline register before it is written back to the architectural register file.

So instead of waiting for write-back, the processor forwards the value directly to the dependent instruction.

That reduces stalls dramatically.

## Stalls And Bubbles

If forwarding cannot solve the problem, the pipeline may need to stall.

A stall means:

- some stages stop advancing for a cycle
- effectively a bubble or no-op is inserted

One classic example is the load-use hazard:

```text
LOAD R1, [R2]
ADD  R3, R1, R4
```

The loaded data may not be ready early enough, so the dependent instruction must wait.

## Branch Prediction

Relevant package:

- `branch-predictor`

Branches are dangerous for deep pipelines because the machine wants to keep fetching instructions, but the correct next instruction depends on a condition that may not be resolved yet.

So the processor guesses.

That guess is branch prediction.

### Why prediction matters

If the guess is correct:

- the pipeline stays full
- performance stays high

If the guess is wrong:

- the wrong-path instructions are flushed
- cycles are wasted

The deeper the pipeline, the more expensive the mistake tends to be.

### Common predictors

At a high level:

- static predictors use fixed rules
- 1-bit predictors remember the last outcome
- 2-bit predictors are more stable around loops
- branch target buffers remember where taken branches go

This repository is especially interested in branch prediction because it is a clean way to connect:

- control flow
- speculation
- performance

## Caches

Relevant package:

- `cache`

CPUs are much faster than main memory.

That mismatch is often called the **memory wall**.

Without caches, the processor would spend huge amounts of time waiting for data.

### What a cache does

A cache keeps a small amount of recently useful data close to the CPU.

It relies on two locality patterns:

- **temporal locality**: if you used something recently, you may use it again soon
- **spatial locality**: if you used one memory address, nearby addresses may be useful soon too

### Memory hierarchy intuition

```text
registers -> L1 -> L2 -> L3 -> DRAM
```

Higher up:

- smaller
- faster
- more expensive per byte

Lower down:

- larger
- slower
- cheaper per byte

### Why caches belong in the architecture track

Because correctness alone does not explain performance.

Two programs can execute the same instructions and still perform very differently depending on:

- access pattern
- cache size
- associativity
- replacement policy
- hit and miss behavior

## Putting The Pieces Together

Here is the broader story:

### The pipeline wants a steady stream of useful work

It wants:

- instructions ready to fetch
- operands ready to use
- memory ready when needed

### Branches threaten that stream

So:

- branch predictors guess future control flow

### Dependencies threaten that stream

So:

- forwarding units bypass values
- hazard detection inserts stalls or flushes when necessary

### Memory latency threatens that stream

So:

- caches try to keep data and instructions nearby

That is the architecture game in one sentence:

keep the machine busy without letting it do the wrong work.

## The Core Package

Relevant package:

- `core`

The `core` package matters because it composes the architecture ideas into a single processor model.

It is where the repository can ask questions like:

- what happens if the pipeline gets deeper?
- what happens if the branch predictor improves?
- what happens if L1 cache gets bigger?
- what happens if forwarding is disabled?

That is where architecture stops being a set of isolated facts and becomes a system of tradeoffs.

## A Concrete Performance Thought Experiment

Imagine two cores that implement the same ISA.

### Core A

- 5-stage pipeline
- small L1 cache
- static branch predictor

### Core B

- deeper pipeline
- larger caches
- 2-bit branch predictor

They can run the same program correctly.

But they may behave very differently on:

- branch-heavy code
- loop-heavy code
- array-processing code
- pointer-heavy code with poor locality

That is exactly why this repository treats architecture as a core topic rather than a side note.

## Mental Checklist For Architecture Reading

When reading one of the architecture packages, it helps to ask:

- what source of latency is this package trying to hide or reduce?
- what kind of wrong work can happen here?
- what state must be tracked to make the decision?
- what tradeoff is being made: simplicity, speed, area, or predictability?

Examples:

- caches trade memory cost and complexity for lower average access latency
- branch predictors trade hardware state for fewer control-flow stalls
- forwarding trades datapath complexity for fewer data stalls
- deeper pipelines trade simpler stage timing for larger misprediction penalties

## How This Connects Back To The Repository

The architecture packages are not just decorative advanced topics.

They connect directly back to the rest of the codebase:

- ISA simulators need a core model to execute on
- the core depends on pipeline behavior
- the pipeline depends on hazard detection and prediction
- memory behavior depends on caches
- instruction timing depends on all of the above

That means architecture is one of the main places where the repository becomes a systems project rather than just a collection of language exercises.

## Where To Go Next

If this document made the pieces feel more connected, the next good move is to read the corresponding specs:

- `D00-deep-cpu-architecture.md`
- `D01-cache.md`
- `D02-branch-predictor.md`
- `D03-hazard-detection.md`
- `D04-pipeline.md`
- `D05-core.md`

Those specs define the design. This learning note is meant to make the motivation and relationships easier to hold in your head while reading them.
