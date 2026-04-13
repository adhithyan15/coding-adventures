"""compiler_source_map — Source map chain sidecar for the AOT compiler pipeline.

This package provides the source map chain data structure that flows through
every stage of the AOT compiler pipeline as a sidecar. Each stage appends
its own segment, enabling end-to-end traceability from source positions to
machine code bytes and back.

Quick Start
-----------

::

    from compiler_source_map import (
        SourceMapChain, SourcePosition,
        SourceToAst, AstToIr, IrToIr, IrToMachineCode,
    )

    # Create a fresh chain:
    chain = SourceMapChain.new()

    # Segment 1: frontend fills SourceToAst
    chain.source_to_ast.add(SourcePosition("a.bf", 1, 1, 1), ast_node_id=0)

    # Segment 2: frontend fills AstToIr
    chain.ast_to_ir.add(ast_node_id=0, ir_ids=[2, 3, 4, 5])

    # Segment 3: optimizer appends IrToIr
    pass1 = IrToIr(pass_name="identity")
    for i in range(6):
        pass1.add_mapping(i, [i])
    chain.add_optimizer_pass(pass1)

    # Segment 4: backend fills IrToMachineCode
    mc = IrToMachineCode()
    mc.add(ir_id=2, mc_offset=0x0C, mc_length=8)
    chain.ir_to_machine_code = mc

    # Forward lookup:
    entries = chain.source_to_mc(SourcePosition("a.bf", 1, 1, 1))

    # Reverse lookup:
    pos = chain.mc_to_source(0x0C)

Segments
--------

- ``SourcePosition``     — a span of characters in a source file
- ``SourceToAst``        — source text positions → AST node IDs
- ``AstToIr``            — AST node IDs → IR instruction IDs
- ``IrToIr``             — IR IDs → optimised IR IDs (one per optimizer pass)
- ``IrToMachineCode``    — IR IDs → machine code byte offsets
- ``SourceMapChain``     — the complete chain + composite lookups
"""

from compiler_source_map.source_map import (
    AstToIr,
    AstToIrEntry,
    IrToIr,
    IrToIrEntry,
    IrToMachineCode,
    IrToMachineCodeEntry,
    SourceMapChain,
    SourcePosition,
    SourceToAst,
    SourceToAstEntry,
)

__all__ = [
    "AstToIr",
    "AstToIrEntry",
    "IrToIr",
    "IrToIrEntry",
    "IrToMachineCode",
    "IrToMachineCodeEntry",
    "SourceMapChain",
    "SourcePosition",
    "SourceToAst",
    "SourceToAstEntry",
]
