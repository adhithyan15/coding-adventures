<!-- learning-concepts: interpreter-ir, semantic-ir, compiler-ir, bytecode-compiler, virtual-machine, jit-compiler -->

# Intermediate Representations

An intermediate representation, or IR, is the language a tool uses between the
source program and its execution target. It lets one stage make facts explicit
so later stages do not need to rediscover them.

## Why More Than One IR?

A syntax tree preserves source structure. A semantic IR can attach resolved
names and types. A compiler IR can make control flow and operations explicit.
Bytecode can provide a compact executable form for a virtual machine.

No single shape is ideal for all of these jobs. A useful IR has a clear owner,
invariants, and consumers.

## The Repository Pipeline

The broad flow is:

```text
source -> tokens -> syntax -> semantic IR -> compiler IR -> bytecode
                                                     -> native lowering
```

An interpreter may consume syntax or a higher-level IR directly. A bytecode
compiler lowers to instructions for a virtual machine. A JIT compiler observes
or receives executable structure and lowers hot work toward a native target.

## Lowering Is a Contract Boundary

Each lowering step should make some ambiguity disappear:

- name resolution replaces text names with known bindings
- type checking replaces uncertain operations with validated ones
- control-flow lowering turns nested syntax into explicit branches and blocks
- bytecode lowering chooses stack or register operations and concrete operands

After a lowering step, later phases should be able to rely on the new
invariants. If every consumer repeats semantic analysis, the IR boundary is not
doing enough work.

## Preserve Source Identity

Lowered code still needs to explain itself to a human. Source spans, symbol
names, and source maps connect an instruction or diagnostic back to the text
that produced it.

This metadata matters for errors, debuggers, stack traces, and tests. It should
be designed alongside the IR rather than patched in after the representation
has erased all source identity.

## Questions for Reading an IR

When exploring one of the repository's IR packages, ask:

1. What phase creates it?
2. What invariants are guaranteed?
3. Is control flow implicit or explicit?
4. Where do types and resolved symbols live?
5. How is source location preserved?
6. Which phases consume it?
7. Can it be serialized or inspected deterministically?

Those answers reveal why the representation exists and where a bug should be
fixed when adjacent phases disagree.
