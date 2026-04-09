# CodingAdventures::RegisterVM

A generic register-based virtual machine (VM) with an accumulator model and
feedback vectors, implemented in pure Perl. Inspired by V8's Ignition bytecode
interpreter.

## What is a Register VM?

There are two main bytecode VM architectures:

**Stack-based** (JVM, CPython): instructions push/pop values on a stack.
Simple to implement; bytecode is compact; values move frequently.

**Register-based** (Lua 5, Dalvik): instructions name operands by register
number. Fewer instructions needed; less data movement; easier to optimise.

This VM is an **accumulator-register hybrid**:

- One implicit *accumulator* register (the usual source and destination).
- N explicit numbered registers per call frame (`r0` through `rN-1`).
- Most binary ops: `acc = acc OP reg[n]`.

This is the same design used by V8 Ignition for running JavaScript in Chrome
and Node.js.

## Where It Fits in the Stack

```
JavaScript / dynamic language source
          ↓  (parser + bytecode compiler)
      CodeObject (bytecode + metadata)
          ↓
  CodingAdventures::RegisterVM
          ↓
     Perl interpreter
          ↓
        OS / hardware
```

The VM sits between a bytecode compiler and the host runtime. A real pipeline
would feed it output from a language's compiler front-end.

## Usage

```perl
use CodingAdventures::RegisterVM;
use CodingAdventures::RegisterVM::Opcodes;

my $OPS = 'CodingAdventures::RegisterVM::Opcodes';

# Compute 3 + 4 = 7
my $code = {
    name                => 'add_example',
    instructions        => [
        { opcode => $OPS->LDA_SMI,  operands => [3], feedback_slot => -1 },
        { opcode => $OPS->STAR,     operands => [0], feedback_slot => -1 },
        { opcode => $OPS->LDA_SMI,  operands => [4], feedback_slot => -1 },
        { opcode => $OPS->ADD,      operands => [0], feedback_slot =>  0 },
        { opcode => $OPS->RETURN,   operands => [],  feedback_slot => -1 },
    ],
    constants           => [],
    names               => [],
    register_count      => 1,
    feedback_slot_count => 1,
    parameter_count     => 0,
};

my $result = CodingAdventures::RegisterVM->run($code, {});
print $result->{value};    # 7
```

## Core Data Structures

### CodeObject

```perl
{
    name                => 'my_function',
    instructions        => [ ... ],     # arrayref of instruction hashrefs
    constants           => [ ... ],     # literals: numbers, strings, nested CodeObjects
    names               => [ ... ],     # variable/property name strings
    register_count      => N,           # number of explicit registers
    feedback_slot_count => M,           # number of type-profiling slots
    parameter_count     => P,           # number of function parameters
}
```

### Instruction

```perl
{
    opcode        => 0x30,    # integer opcode constant
    operands      => [0],     # arrayref of integer operands
    feedback_slot => 0,       # feedback vector index, or -1 for none
}
```

### CallFrame (internal)

```perl
{
    code            => $code_obj,
    ip              => 0,            # instruction pointer
    accumulator     => undef,
    registers       => [...],        # register file
    feedback_vector => [...],        # FeedbackSlot hashrefs
    context         => $scope,       # lexical scope chain
    caller_frame    => $parent,      # previous frame or undef
}
```

### FeedbackSlot States

| State          | Meaning                                  |
|----------------|------------------------------------------|
| uninitialized  | No operation has run through this site   |
| monomorphic    | One distinct type-pair observed          |
| polymorphic    | 2–4 distinct type-pairs                  |
| megamorphic    | 5+ distinct type-pairs; give up on IC    |

## Opcodes

The VM implements ~70 opcodes grouped by category:

| Range     | Category                              |
|-----------|---------------------------------------|
| 0x00–0x06 | Load accumulator (immediate/literals) |
| 0x10–0x12 | Register ↔ accumulator moves         |
| 0x20–0x25 | Global and context variable access    |
| 0x30–0x3F | Arithmetic and bitwise operations     |
| 0x40–0x4C | Comparison and logical tests          |
| 0x50–0x58 | Jumps and branches                    |
| 0x60–0x66 | Calls, returns, generators            |
| 0x70–0x77 | Property load / store                 |
| 0x80–0x85 | Object/array/closure creation         |
| 0x90–0x93 | Iterator protocol                     |
| 0xA0–0xA1 | Exception handling                    |
| 0xB0–0xB3 | Context and module variables          |
| 0xF0–0xFF | Meta (STACK_CHECK, DEBUGGER, HALT)    |

See `lib/CodingAdventures/RegisterVM/Opcodes.pm` for the full table with
comments explaining each opcode's purpose.

## Running Tests

```sh
cpanm --installdeps .
prove -l -v t/
```

## Key Concepts for Learners

### Accumulator Model

Instead of naming both operands in every instruction, one side is always the
implicit accumulator. This halves the operand count for most instructions:

```
# Stack VM: two pops, one push
PUSH A
PUSH B
ADD        ← pops A and B, pushes A+B

# Accumulator VM: one register operand, result in accumulator
LDA A      ← acc = A
ADD B      ← acc = acc + B
```

### Feedback Vectors and Inline Caches

JavaScript engines don't know the type of `x + y` until runtime. Feedback
vectors record what types actually appear. After a few executions, a JIT
compiler can specialise:

```
# Generic: works for any types
add_generic(acc, reg[0])

# Specialised after profiling "int:int" always:
add_int_int(acc, reg[0])   ← fast! No type checks needed
```

### Hidden Classes

Objects with the same property layout share a hidden class ID. Property access
`obj.x` can be optimised to a fixed offset lookup if the JIT knows `obj`
always has hidden class ID 5 (which always has `x` at offset 2).

## License

MIT
