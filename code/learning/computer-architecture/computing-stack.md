# The Computing Stack

This repository is organized around a simple but powerful idea:

high-level software only feels magical when you cannot see the layers underneath it.

The computing stack is those layers.

## The Big Picture

When you write something like:

```text
x = 1 + 2
```

a lot has to happen before a machine can execute that idea.

One useful way to see the stack is:

```text
Source code
-> lexer
-> parser
-> compiler or translator
-> instructions
-> machine or virtual machine
-> CPU datapath
-> arithmetic units
-> logic gates
```

Each layer converts one representation into another.

## Why Build The Whole Stack?

Because every layer answers a different question.

- **Lexer**: how do we break text into meaningful pieces?
- **Parser**: how do we recover structure from those pieces?
- **Compiler**: how do we translate structure into executable form?
- **VM or ISA simulator**: how are instructions actually executed?
- **CPU / ALU**: how does arithmetic happen mechanically?
- **Logic gates**: what is the irreducible foundation underneath all of it?

The repository is trying to make those answers visible.

## The Stack In This Repository

### Layer 10: Logic gates

Relevant packages:

- `logic-gates`

This is the floor.

At this level, there are no "numbers" in the human sense, no variables, no loops, and no syntax trees. There are only binary signals and the rules that transform them.

Examples:

- NOT flips a bit
- AND outputs 1 only if both inputs are 1
- XOR is the key gate for addition

Why it matters:

Because arithmetic and control logic are eventually made out of these simple parts.

### Layer 9: Arithmetic

Relevant packages:

- `arithmetic`
- `fp-arithmetic`

This layer answers:

- how do we add binary numbers?
- how do carry bits propagate?
- how do multiplication and subtraction work?
- how do floating-point formats represent real numbers imperfectly?

The jump from logic gates to arithmetic is one of the most satisfying transitions in computer science. XOR, AND, and OR stop being abstract truth tables and start acting like machinery.

### Layer 8: CPU simulation

Relevant packages:

- `cpu-simulator`
- `clock`

Now we have enough machinery to talk about:

- registers
- program counters
- memory
- instruction fetch
- instruction decode
- execution

This is the level where the machine starts to look like a processor instead of a pile of circuits.

### Layer 7: Instruction-set simulators

Relevant packages:

- `arm-simulator`
- `riscv-simulator`
- `wasm-simulator`
- `intel4004-simulator`
- `jvm-simulator`
- `clr-simulator`

This layer answers:

- what instructions exist?
- how are operands represented?
- where do instructions read their inputs from?
- where do they write their outputs?

This is where different execution models become visible.

For example:

- ARM and RISC-V are register-oriented
- WASM, JVM, and CLR are stack-oriented
- Intel 4004 is a much older and narrower historical design

### Layers 2-6: Language frontends and execution

Relevant packages:

- `grammar-tools`
- `lexer`
- `parser`
- language-specific lexer and parser packages
- `bytecode-compiler`
- `virtual-machine`
- `assembler`
- `jit-compiler`

This layer connects source code back to execution.

Instead of starting from hardware and moving upward, this part starts from human-readable programs and moves downward.

That is why the repository has a fork in the middle:

```text
source code
-> lexer
-> parser
-> interpreted path: bytecode compiler -> virtual machine
-> compiled path: assembler / ISA path -> machine-style execution
```

## Two Main Execution Stories

### Story 1: Interpreted or VM-based execution

Example path:

```text
source code
-> tokens
-> AST
-> bytecode
-> virtual machine
```

This is conceptually similar to:

- Python bytecode execution
- Ruby VM execution
- JVM bytecode execution
- CLR bytecode execution

Why this path matters:

- it shows how high-level structure can be lowered into a compact instruction format
- it reveals stack-based execution clearly
- it creates a bridge between language tooling and runtime systems

### Story 2: Compiled or machine-oriented execution

Example path:

```text
source code
-> tokens
-> AST
-> assembly or machine-like representation
-> ISA simulator
```

Why this path matters:

- it connects programming languages directly to machine models
- it shows how an ISA acts like the contract between software and hardware
- it makes instruction encoding and execution more concrete

## What Makes The Architecture Track Special

A simple CPU simulator tells you **what** happens.

The architecture packages tell you **how it happens fast**.

That is a different question.

Examples:

- A simple CPU model says "the branch was taken."
- architecture asks "how many cycles were lost because we guessed wrong?"

- A simple CPU model says "load this value from memory."
- architecture asks "was that an L1 hit, an L2 hit, or a miss to DRAM?"

- A simple CPU model says "instruction B uses the result of instruction A."
- architecture asks "can that value be forwarded, or must the pipeline stall?"

That is why the architecture material is so important in this repository: it turns correctness into performance reasoning.

## A Helpful Mental Model

One way to remember the stack is:

- **language layers** explain meaning
- **instruction layers** explain execution
- **architecture layers** explain performance
- **logic layers** explain physical possibility

Each layer is solving a different class of problem.

## How The Repository Uses Repetition

This repository implements many of the same ideas in multiple languages.

That is not duplication for its own sake.

It is a teaching technique.

If you can recognize:

- the same graph algorithm in Python and Go
- the same cache idea in Ruby and TypeScript
- the same VM structure across multiple implementations

then you have started learning the concept itself rather than memorizing one syntax.

## Where To Go Next

After this overview:

- read [Instruction-set models](./instruction-set-models.md) to compare register, stack, and accumulator machines
- read [Pipelines, caches, and speculation](./pipelines-caches-and-speculation.md) to understand the architecture side of performance
- read [Language tooling](../language-tooling/README.md) to connect the frontend side back to this hardware story
