# Instruction-Set Models

Not all machines "feel" the same internally, even when they are all executing instructions.

One of the most useful architecture lessons in this repository is that different instruction sets expose different machine models:

- register machines
- stack machines
- accumulator machines

Understanding those models makes the ISA simulator packages much easier to reason about.

## What An ISA Actually Is

An ISA, or instruction set architecture, is the contract between software and hardware.

It defines things like:

- what instructions exist
- how they are encoded
- how many registers there are
- how memory is addressed
- how control flow works

But an ISA does **not** define the whole micro-architecture.

It does not say:

- how deep the pipeline is
- whether branches are predicted well
- what the cache sizes are

That is why two processors can implement the same ISA and have very different performance.

## Model 1: Register Machines

Relevant packages:

- `arm-simulator`
- `riscv-simulator`

In a register machine, instructions usually name their operands explicitly.

Example shape:

```text
ADD R3, R1, R2
```

This means:

- read `R1`
- read `R2`
- add them
- write the result to `R3`

### Why register machines are attractive

- operands are explicit
- dataflow is easier to inspect
- compilers can keep hot values in registers
- they map well to modern general-purpose CPUs

### Tradeoff

The encoding has to spend bits naming registers, so instructions may be wider or need more careful encoding design.

## Model 2: Stack Machines

Relevant packages:

- `wasm-simulator`
- `jvm-simulator`
- `clr-simulator`
- `virtual-machine`
- `bytecode-compiler`

In a stack machine, most instructions do not name registers directly. Instead, they operate on the top of an operand stack.

Example:

```text
PUSH 1
PUSH 2
ADD
```

Execution:

```text
stack = []
PUSH 1 -> [1]
PUSH 2 -> [1, 2]
ADD    -> [3]
```

### Why stack machines are attractive

- instruction encoding is compact
- compilers can emit instructions without doing full register allocation
- evaluation order is very explicit
- they are great for virtual machines and portable bytecode

### Tradeoff

The dataflow is less explicit in each instruction. You often need to understand the surrounding stack state to understand what one instruction is doing.

## Model 3: Accumulator Machines

Relevant package:

- `intel4004-simulator`

An accumulator machine has one distinguished register, often called the accumulator, that many operations implicitly use.

Example shape:

```text
LOAD A, value
ADD  A, other
STORE A, destination
```

The machine keeps returning to the accumulator as the main working register.

### Why this model existed

It simplifies hardware and instruction encoding, especially in early processors where transistor budgets were tiny.

### Tradeoff

It is more restrictive. Programs spend more time moving values in and out of the one especially important register.

## Comparing The Three Models

| Model | Example family | Where operands live | Typical strength |
|-------|----------------|--------------------|------------------|
| Register machine | ARM, RISC-V | named registers | explicit dataflow |
| Stack machine | WASM, JVM, CLR, VMs | operand stack | compact bytecode |
| Accumulator machine | Intel 4004 | one main register plus helpers | hardware simplicity |

## Same Computation, Different Models

Let us compute:

```text
x = 1 + 2
```

### Register-machine style

```text
MOV R1, 1
MOV R2, 2
ADD R3, R1, R2
STORE x, R3
```

### Stack-machine style

```text
PUSH 1
PUSH 2
ADD
STORE x
```

### Accumulator-machine style

```text
LOAD A, 1
ADD  A, 2
STORE x, A
```

Each machine is doing the same conceptual work, but the machine model changes what the instructions need to say.

## Why This Matters For The Repository

The repository spans all three styles.

That is a big teaching advantage because it lets you compare:

- source-language lowering into bytecode stacks
- explicit register movement in RISC-like machines
- historical minimalist designs

You get to see that instruction design is not arbitrary. It reflects tradeoffs in hardware complexity, encoding density, compiler strategy, and execution model.

## VM Bytecode And Real Hardware

A useful mental shortcut is:

- VM bytecode often looks stack-machine-like
- many real CPUs look register-machine-like

That is why this repository's interpreted path and compiled path are both useful.

The interpreted path helps you understand evaluation order and runtime representation.

The compiled path helps you understand explicit architectural state.

## Control Flow In Different Models

All three models still need:

- branches
- jumps
- function calls
- returns

But the surrounding data model affects how convenient those operations are.

For example:

- a stack machine may naturally support expression evaluation
- a register machine may give the compiler more direct control over scheduling and value lifetime

## Architecture Versus ISA

It is worth saying this twice because it is one of the most important distinctions in the repo:

The ISA says **what instructions mean**.

The micro-architecture says **how those instructions are executed efficiently**.

So:

- `arm-simulator` and `riscv-simulator` are about ISA behavior
- `cache`, `branch-predictor`, `hazard-detection`, `pipeline`, and `core` are about micro-architecture

Those layers are related, but they are not the same thing.

## Questions To Ask When Studying A New ISA

When you read one of the simulator packages, it helps to ask:

- Are operands explicit or implicit?
- Is this closer to a register machine or a stack machine?
- How many architectural registers exist?
- Does the ISA encourage compact code or explicit data movement?
- How easy is it for a compiler to target this model?

Those questions turn instruction listings into design discussions instead of just syntax.

## Where To Go Next

After understanding the ISA models, the next step is to ask how modern processors keep them fast.

That leads directly to:

- [Pipelines, caches, and speculation](./pipelines-caches-and-speculation.md)
