"""Source Map Chain — the debugging sidecar for the AOT compiler pipeline.

Overview
--------

The source map chain flows through every stage of the AOT compiler pipeline
as a sidecar data structure. Each stage reads what came before it and appends
its own segment. The result is a chain that answers two key questions:

  "Which machine code bytes correspond to this source position?"
      → forward lookup: SourceToMC()

  "Which source position produced this machine code byte?"
      → reverse lookup: MCToSource()

Why a Chain and Not a Flat Table?
----------------------------------

A flat table (machine-code offset → source position) works for the *final
consumer* — a debugger, profiler, or error reporter. But it doesn't help
when you're debugging the *compiler itself*:

  - "Why did the optimiser delete instruction #42?"
    → Look at the IrToIr segment for that pass.

  - "Which AST node produced this IR instruction?"
    → Look at AstToIr.

  - "The machine code for this instruction seems wrong — what IR produced it?"
    → Look at IrToMachineCode in reverse.

The chain makes the compiler pipeline **transparent and debuggable at every
stage**. The flat composite mapping is just the composition of all segments.

Segment Overview
-----------------

::

  Segment 1: SourceToAst      — source text position   → AST node ID
  Segment 2: AstToIr          — AST node ID            → IR instruction IDs
  Segment 3: IrToIr           — IR instruction ID      → optimised IR IDs
                                (one segment per optimiser pass)
  Segment 4: IrToMachineCode  — IR instruction ID      → machine code offset + length

  Composite: source position  → machine code offset  (forward)
             machine code offset → source position   (reverse)

Each of these is a separate Python class. The ``SourceMapChain`` collects
them all and provides the forward and reverse composite lookups.

Source Positions
-----------------

A ``SourcePosition`` is a "highlighter pen" marking a region of source code.
The ``(line, column)`` pair marks the start; ``length`` tells you how many
characters are highlighted. For Brainfuck, every command is exactly one
character (``length=1``). For BASIC, a keyword like ``PRINT`` has
``length=5``.
"""

from __future__ import annotations

from dataclasses import dataclass, field

# ---------------------------------------------------------------------------
# SourcePosition — a span of characters in a source file
# ---------------------------------------------------------------------------
#
# Think of this as a "highlighter pen" marking a region of source code.
# The (line, column) pair marks the start; length tells you how many
# characters are highlighted. For Brainfuck, every command is exactly one
# character (length=1). For BASIC, a keyword like "PRINT" would have length=5.
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class SourcePosition:
    """A span of characters in a source file.

    Attributes:
        file:   Source file path (e.g., ``"hello.bf"``).
        line:   1-based line number.
        column: 1-based column number.
        length: Character span in source (1 for Brainfuck commands).

    Example::

        pos = SourcePosition(file="hello.bf", line=1, column=3, length=1)
        str(pos)  # "hello.bf:1:3 (len=1)"
    """

    file: str
    line: int
    column: int
    length: int

    def __str__(self) -> str:
        """Return a human-readable representation like ``"hello.bf:1:3 (len=1)"``."""
        return f"{self.file}:{self.line}:{self.column} (len={self.length})"


# ---------------------------------------------------------------------------
# SourceToAst — Segment 1: source text positions → AST node IDs
# ---------------------------------------------------------------------------
#
# This segment is produced by the language-specific frontend (e.g.,
# brainfuck-ir-compiler). It maps every meaningful source position to the
# AST node that represents it.
#
# Example:
#   The "+" character at line 1, column 3 of "hello.bf" maps to AST node
#   #42 (which is a command(INC) node in the parse tree).
# ---------------------------------------------------------------------------


@dataclass
class SourceToAstEntry:
    """One mapping from a source position to an AST node ID.

    Attributes:
        pos:         The source position.
        ast_node_id: The ID of the AST node at that position.
    """

    pos: SourcePosition
    ast_node_id: int


@dataclass
class SourceToAst:
    """Segment 1: source text positions → AST node IDs.

    This segment is produced by the parser or by the language-specific
    frontend. It maps every meaningful source position to the AST node
    that represents it.

    Attributes:
        entries: The list of (SourcePosition, ast_node_id) pairs.

    Example::

        s = SourceToAst()
        s.add(SourcePosition("hello.bf", 1, 1, 1), ast_node_id=0)
        s.add(SourcePosition("hello.bf", 1, 2, 1), ast_node_id=1)
        s.lookup_by_node_id(1)  # SourcePosition("hello.bf", 1, 2, 1)
    """

    entries: list[SourceToAstEntry] = field(default_factory=list)

    def add(self, pos: SourcePosition, ast_node_id: int) -> None:
        """Record a mapping from a source position to an AST node ID.

        Args:
            pos:         The source position.
            ast_node_id: The AST node that represents this position.
        """
        self.entries.append(SourceToAstEntry(pos=pos, ast_node_id=ast_node_id))

    def lookup_by_node_id(self, ast_node_id: int) -> SourcePosition | None:
        """Return the source position for the given AST node ID.

        Returns the first match, or ``None`` if not found.

        Args:
            ast_node_id: The AST node ID to look up.

        Returns:
            The ``SourcePosition`` if found, else ``None``.

        Example::

            pos = s.lookup_by_node_id(42)  # SourcePosition for node 42
        """
        for entry in self.entries:
            if entry.ast_node_id == ast_node_id:
                return entry.pos
        return None


# ---------------------------------------------------------------------------
# AstToIr — Segment 2: AST node IDs → IR instruction IDs
# ---------------------------------------------------------------------------
#
# A single AST node often produces multiple IR instructions. For example,
# a Brainfuck "+" command produces four instructions:
#   LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE
# So the mapping is one-to-many: ast_node_42 → [ir_7, ir_8, ir_9, ir_10].
# ---------------------------------------------------------------------------


@dataclass
class AstToIrEntry:
    """One mapping from an AST node to the IR instructions it produced.

    Attributes:
        ast_node_id: The ID of the AST node.
        ir_ids:      The IR instruction IDs this AST node produced.
    """

    ast_node_id: int
    ir_ids: list[int]


@dataclass
class AstToIr:
    """Segment 2: AST node IDs → IR instruction IDs.

    A single AST node often produces multiple IR instructions. For example,
    a Brainfuck "+" command produces four instructions:
    ``LOAD_BYTE``, ``ADD_IMM``, ``AND_IMM``, ``STORE_BYTE``.

    Attributes:
        entries: The list of (ast_node_id, ir_ids) pairs.

    Example::

        a = AstToIr()
        a.add(ast_node_id=0, ir_ids=[2, 3, 4, 5])  # "+" → 4 instructions
        a.add(ast_node_id=1, ir_ids=[6, 7])         # "." → 2 instructions
    """

    entries: list[AstToIrEntry] = field(default_factory=list)

    def add(self, ast_node_id: int, ir_ids: list[int]) -> None:
        """Record that the given AST node produced the given IR instruction IDs.

        Args:
            ast_node_id: The AST node that emitted these instructions.
            ir_ids:      The IR instruction IDs the node produced.
        """
        self.entries.append(AstToIrEntry(ast_node_id=ast_node_id, ir_ids=ir_ids))

    def lookup_by_ast_node_id(self, ast_node_id: int) -> list[int] | None:
        """Return the IR instruction IDs for the given AST node.

        Args:
            ast_node_id: The AST node ID to look up.

        Returns:
            The list of IR IDs, or ``None`` if not found.
        """
        for entry in self.entries:
            if entry.ast_node_id == ast_node_id:
                return entry.ir_ids
        return None

    def lookup_by_ir_id(self, ir_id: int) -> int:
        """Return the AST node ID that produced the given IR instruction.

        Searches through all entries for an entry whose ``ir_ids`` list
        contains ``ir_id``. Returns -1 if not found.

        Args:
            ir_id: The IR instruction ID to look up.

        Returns:
            The AST node ID, or -1 if not found.

        Example::

            a.lookup_by_ir_id(4)  # 0  (node 0 produced IR 4)
        """
        for entry in self.entries:
            if ir_id in entry.ir_ids:
                return entry.ast_node_id
        return -1


# ---------------------------------------------------------------------------
# IrToIr — Segment 3: IR instruction IDs → optimised IR instruction IDs
# ---------------------------------------------------------------------------
#
# One segment is produced per optimiser pass. The pass_name field identifies
# which pass produced this mapping (e.g., "identity", "contraction",
# "clear_loop", "dead_store").
#
# Three cases:
#   1. Preserved:  original_id → [same_id]        (instruction unchanged)
#   2. Replaced:   original_id → [new_id_1, ...]  (instruction split/transformed)
#   3. Deleted:    original_id is in deleted set   (instruction optimised away)
#
# Example: A contraction pass folds three ADD_IMM 1 instructions
# (IDs 7, 8, 9) into one ADD_IMM 3 (ID 100):
#   7 → [100], 8 → [100], 9 → [100]
# ---------------------------------------------------------------------------


@dataclass
class IrToIrEntry:
    """One mapping from an original IR instruction to its replacement(s).

    Attributes:
        original_id: The IR instruction ID before the optimiser pass.
        new_ids:     The IR instruction IDs after the pass (empty if deleted).
    """

    original_id: int
    new_ids: list[int]


@dataclass
class IrToIr:
    """Segment 3: IR instruction IDs → optimised IR instruction IDs.

    One segment is produced per optimiser pass. The ``pass_name`` field
    identifies which pass produced this segment (e.g., ``"identity"``,
    ``"contraction"``, ``"dead_store"``).

    Three cases for each original instruction:

    1. **Preserved**: ``original_id → [same_id]`` — instruction unchanged.
    2. **Replaced**: ``original_id → [new_id_1, ...]`` — split or transformed.
    3. **Deleted**: original_id is in ``deleted`` — optimised away.

    Attributes:
        entries:   The list of (original_id, new_ids) pairs.
        deleted:   Set of original IDs that were optimised away.
        pass_name: Which optimiser pass produced this segment.

    Example::

        m = IrToIr(pass_name="identity")
        for i in range(8):
            m.add_mapping(i, [i])  # identity: each ID maps to itself
    """

    entries: list[IrToIrEntry] = field(default_factory=list)
    deleted: set[int] = field(default_factory=set)
    pass_name: str = ""

    def add_mapping(self, original_id: int, new_ids: list[int]) -> None:
        """Record that the original instruction was replaced by the new ones.

        Args:
            original_id: The original IR instruction ID.
            new_ids:     The new IR instruction IDs (may be the same ID for
                         identity passes, or different IDs for transforms).
        """
        self.entries.append(IrToIrEntry(original_id=original_id, new_ids=new_ids))

    def add_deletion(self, original_id: int) -> None:
        """Record that the original instruction was deleted by this pass.

        Deleted instructions have no corresponding new instruction. The
        ``deleted`` set tracks which IDs were removed.

        Args:
            original_id: The original IR instruction ID that was deleted.
        """
        self.deleted.add(original_id)
        self.entries.append(IrToIrEntry(original_id=original_id, new_ids=[]))

    def lookup_by_original_id(self, original_id: int) -> list[int] | None:
        """Return the new IDs for the given original ID.

        Returns ``None`` if the instruction was deleted or not found.

        Args:
            original_id: The original IR instruction ID.

        Returns:
            The list of new IR IDs, or ``None`` if deleted or not found.
        """
        if original_id in self.deleted:
            return None
        for entry in self.entries:
            if entry.original_id == original_id:
                return entry.new_ids
        return None

    def lookup_by_new_id(self, new_id: int) -> int:
        """Return the original ID that produced the given new ID.

        When multiple originals map to the same new ID (e.g., contraction),
        this returns the first one found. Returns -1 if not found.

        Args:
            new_id: The new IR instruction ID to reverse-look up.

        Returns:
            The original ID, or -1 if not found.
        """
        for entry in self.entries:
            if new_id in entry.new_ids:
                return entry.original_id
        return -1


# ---------------------------------------------------------------------------
# IrToMachineCode — Segment 4: IR instruction IDs → machine code byte offsets
# ---------------------------------------------------------------------------
#
# Each entry is a triple: (ir_id, mc_offset, mc_length).
# For example, a LOAD_BYTE IR instruction might produce 8 bytes of RISC-V
# machine code starting at offset 0x14 in the .text section.
# ---------------------------------------------------------------------------


@dataclass
class IrToMachineCodeEntry:
    """One mapping from an IR instruction to the machine code bytes it produced.

    Attributes:
        ir_id:     IR instruction ID.
        mc_offset: Byte offset in the .text section.
        mc_length: Number of bytes of machine code.

    Example::

        e = IrToMachineCodeEntry(ir_id=3, mc_offset=0x14, mc_length=4)
        # IR instruction 3 → 4 bytes of machine code starting at offset 0x14
    """

    ir_id: int
    mc_offset: int
    mc_length: int


@dataclass
class IrToMachineCode:
    """Segment 4: IR instruction IDs → machine code byte offsets.

    This segment is produced by the code generation backend. It maps each
    IR instruction to the region of machine code bytes it produced.

    Attributes:
        entries: The list of (ir_id, mc_offset, mc_length) triples.

    Example::

        mc = IrToMachineCode()
        mc.add(ir_id=0, mc_offset=0x00, mc_length=8)  # LOAD_ADDR → 8 bytes
        mc.add(ir_id=1, mc_offset=0x08, mc_length=4)  # LOAD_IMM → 4 bytes
    """

    entries: list[IrToMachineCodeEntry] = field(default_factory=list)

    def add(self, ir_id: int, mc_offset: int, mc_length: int) -> None:
        """Record that the given IR instruction produced machine code.

        Args:
            ir_id:     The IR instruction ID.
            mc_offset: The byte offset in the .text section.
            mc_length: The number of bytes produced.
        """
        self.entries.append(
            IrToMachineCodeEntry(ir_id=ir_id, mc_offset=mc_offset, mc_length=mc_length)
        )

    def lookup_by_ir_id(self, ir_id: int) -> tuple[int, int]:
        """Return the machine code offset and length for the given IR instruction.

        Args:
            ir_id: The IR instruction ID.

        Returns:
            A ``(offset, length)`` tuple. Returns ``(-1, 0)`` if not found.

        Example::

            offset, length = mc.lookup_by_ir_id(3)  # (0x14, 4)
        """
        for entry in self.entries:
            if entry.ir_id == ir_id:
                return entry.mc_offset, entry.mc_length
        return -1, 0

    def lookup_by_mc_offset(self, offset: int) -> int:
        """Return the IR instruction ID whose machine code contains this offset.

        An offset ``o`` is "contained by" an entry if::

            entry.mc_offset <= o < entry.mc_offset + entry.mc_length

        Args:
            offset: A byte offset in the .text section.

        Returns:
            The IR instruction ID, or -1 if no entry contains this offset.

        Example::

            # mc entry: ir_id=5, mc_offset=0x20, mc_length=4
            mc.lookup_by_mc_offset(0x22)  # 5 (inside the range)
            mc.lookup_by_mc_offset(0x24)  # -1 (past the end)
        """
        for entry in self.entries:
            if entry.mc_offset <= offset < entry.mc_offset + entry.mc_length:
                return entry.ir_id
        return -1


# ---------------------------------------------------------------------------
# SourceMapChain — the full pipeline sidecar
# ---------------------------------------------------------------------------
#
# This is the central data structure that flows through every stage of the
# compiler pipeline. Each stage reads the existing segments and appends
# its own:
#
#   1. Frontend (brainfuck-ir-compiler) → fills SourceToAst + AstToIr
#   2. Optimiser (compiler-ir-optimizer) → appends IrToIr segments
#   3. Backend (codegen-riscv) → fills IrToMachineCode
# ---------------------------------------------------------------------------


@dataclass
class SourceMapChain:
    """The full pipeline source map sidecar.

    This data structure flows through every stage of the compiler pipeline.
    Each stage reads the existing segments and appends its own:

      1. Frontend (brainfuck-ir-compiler) → fills ``source_to_ast`` and ``ast_to_ir``
      2. Optimiser (compiler-ir-optimizer) → appends ``ir_to_ir`` segments
      3. Backend (codegen-riscv) → fills ``ir_to_machine_code``

    Attributes:
        source_to_ast:     Segment 1: source positions → AST node IDs.
        ast_to_ir:         Segment 2: AST node IDs → IR instruction IDs.
        ir_to_ir:          Segment 3: one entry per optimiser pass.
        ir_to_machine_code: Segment 4: IR IDs → machine code offsets. ``None``
                            until the backend fills it.

    Example::

        chain = SourceMapChain.new()
        chain.source_to_ast.add(SourcePosition("a.bf", 1, 1, 1), 0)
        chain.ast_to_ir.add(0, [2, 3, 4, 5])
    """

    source_to_ast: SourceToAst
    ast_to_ir: AstToIr
    ir_to_ir: list[IrToIr]
    ir_to_machine_code: IrToMachineCode | None

    @classmethod
    def new(cls) -> SourceMapChain:
        """Create an empty source map chain ready for use.

        The ``ir_to_machine_code`` field starts as ``None`` because the
        backend fills it last. The ``ir_to_ir`` list starts empty.

        Returns:
            A fresh ``SourceMapChain`` with empty segments.
        """
        return cls(
            source_to_ast=SourceToAst(),
            ast_to_ir=AstToIr(),
            ir_to_ir=[],
            ir_to_machine_code=None,
        )

    def add_optimizer_pass(self, segment: IrToIr) -> None:
        """Append an IrToIr segment from an optimiser pass.

        Each optimiser pass appends one segment. The order matters for
        the composite lookup: passes are applied in the order they were
        appended.

        Args:
            segment: The ``IrToIr`` segment to append.
        """
        self.ir_to_ir.append(segment)

    # -----------------------------------------------------------------------
    # Composite queries — compose all segments for end-to-end lookups
    # -----------------------------------------------------------------------

    def source_to_mc(self, pos: SourcePosition) -> list[IrToMachineCodeEntry] | None:
        """Compose all segments to look up machine code for a source position.

        The algorithm is:

          1. ``SourceToAst``: source position → AST node ID
          2. ``AstToIr``: AST node ID → IR instruction IDs
          3. ``IrToIr`` (each pass): follow IR IDs through each optimiser pass
          4. ``IrToMachineCode``: final IR IDs → machine code offsets

        Returns ``None`` if:
        - The chain has no ``ir_to_machine_code`` segment yet
        - The source position is not found in segment 1
        - No IR IDs survive optimiser passes (all deleted)

        Args:
            pos: The source position to look up.

        Returns:
            A list of ``IrToMachineCodeEntry`` objects, or ``None``.

        Example::

            entries = chain.source_to_mc(
                SourcePosition("hello.bf", 1, 1, 1)
            )
            # entries[0].mc_offset  # byte offset of first machine code byte
        """
        if self.ir_to_machine_code is None:
            return None

        # Step 1: source → AST node ID
        ast_node_id = -1
        for entry in self.source_to_ast.entries:
            if (
                entry.pos.file == pos.file
                and entry.pos.line == pos.line
                and entry.pos.column == pos.column
            ):
                ast_node_id = entry.ast_node_id
                break
        if ast_node_id == -1:
            return None

        # Step 2: AST node → IR IDs
        ir_ids = self.ast_to_ir.lookup_by_ast_node_id(ast_node_id)
        if ir_ids is None:
            return None

        # Step 3: follow through optimiser passes
        # Each pass may replace, preserve, or delete IR IDs.
        current_ids = list(ir_ids)
        for pass_seg in self.ir_to_ir:
            next_ids: list[int] = []
            for ir_id in current_ids:
                if ir_id in pass_seg.deleted:
                    continue  # instruction was optimised away
                new_ids = pass_seg.lookup_by_original_id(ir_id)
                if new_ids is not None:
                    next_ids.extend(new_ids)
            current_ids = next_ids

        if not current_ids:
            return None

        # Step 4: IR IDs → machine code
        results: list[IrToMachineCodeEntry] = []
        for ir_id in current_ids:
            offset, length = self.ir_to_machine_code.lookup_by_ir_id(ir_id)
            if offset >= 0:
                results.append(
                    IrToMachineCodeEntry(
                        ir_id=ir_id, mc_offset=offset, mc_length=length
                    )
                )
        return results if results else None

    def mc_to_source(self, mc_offset: int) -> SourcePosition | None:
        """Compose all segments in reverse to look up the source for machine code.

        The algorithm is the reverse of ``source_to_mc``:

          1. ``IrToMachineCode``: MC offset → IR instruction ID
          2. ``IrToIr`` (each pass, **in reverse order**): trace IR ID back
          3. ``AstToIr``: IR ID → AST node ID
          4. ``SourceToAst``: AST node ID → source position

        Returns ``None`` if:
        - The chain has no ``ir_to_machine_code`` segment yet
        - The MC offset is not covered by any IR instruction
        - The trace fails at any step (e.g., prologue instructions with no
          AST mapping)

        Args:
            mc_offset: A byte offset in the .text section.

        Returns:
            A ``SourcePosition``, or ``None``.

        Example::

            pos = chain.mc_to_source(0x14)
            # SourcePosition(file="hello.bf", line=1, column=1, length=1)
        """
        if self.ir_to_machine_code is None:
            return None

        # Step 1: MC offset → IR ID
        ir_id = self.ir_to_machine_code.lookup_by_mc_offset(mc_offset)
        if ir_id == -1:
            return None

        # Step 2: follow back through optimiser passes (in REVERSE order)
        # Each pass maps new IDs back to original IDs.
        current_id = ir_id
        for pass_seg in reversed(self.ir_to_ir):
            original_id = pass_seg.lookup_by_new_id(current_id)
            if original_id == -1:
                return None  # can't trace back through this pass
            current_id = original_id

        # Step 3: IR ID → AST node ID
        ast_node_id = self.ast_to_ir.lookup_by_ir_id(current_id)
        if ast_node_id == -1:
            return None

        # Step 4: AST node ID → source position
        return self.source_to_ast.lookup_by_node_id(ast_node_id)
