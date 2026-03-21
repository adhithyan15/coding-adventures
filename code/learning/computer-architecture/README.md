# Computer Architecture

Computer architecture is one of the central themes of this repository.

This track is where the repo moves beyond "a program runs" and into questions like:

- how instructions flow through a machine
- why some execution models use stacks and others use registers
- why caches exist
- why branch prediction matters
- what hazards are
- how a core is assembled from smaller pieces

## Topics

- [Computing stack](./computing-stack.md)
- [Instruction-set models](./instruction-set-models.md)
- [Pipelines, caches, and speculation](./pipelines-caches-and-speculation.md)

## Package Coverage

| Package family | Main learning entry |
|----------------|---------------------|
| `logic-gates`, `arithmetic`, `fp-arithmetic`, `cpu-simulator` | [Computing stack](./computing-stack.md) |
| `arm-simulator`, `riscv-simulator`, `wasm-simulator`, `intel4004-simulator`, `jvm-simulator`, `clr-simulator` | [Instruction-set models](./instruction-set-models.md) |
| `cache`, `branch-predictor`, `hazard-detection`, `pipeline`, `core` | [Pipelines, caches, and speculation](./pipelines-caches-and-speculation.md) |

## How To Use This Track

If you are new to architecture, start with:

1. [Computing stack](./computing-stack.md)
2. [Instruction-set models](./instruction-set-models.md)
3. [Pipelines, caches, and speculation](./pipelines-caches-and-speculation.md)

That order moves from broad to specific:

- first: what the layers are
- second: what kinds of machines exist
- third: why modern cores are much more than a fetch-decode-execute loop
