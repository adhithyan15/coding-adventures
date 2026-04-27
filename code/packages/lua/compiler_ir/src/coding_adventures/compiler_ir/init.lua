-- ============================================================================
-- compiler_ir — General-purpose IR type library for the AOT compiler pipeline
-- ============================================================================
--
-- This module defines the intermediate representation (IR) used by the
-- coding-adventures AOT compiler pipeline. The IR is general-purpose:
-- it can serve as the compilation target for any language (Brainfuck, BASIC,
-- Lua, etc.), not just Brainfuck.
--
-- ## Design Philosophy
--
-- The IR is:
--   - Linear: no basic blocks, no SSA, no phi nodes. Instructions execute
--     from top to bottom, with jumps/branches altering the flow.
--   - Register-based: infinite virtual registers (v0, v1, v2, ...). The
--     backend's register allocator maps these to physical registers.
--   - Target-independent: backends map IR to physical ISA instructions.
--   - Versioned: a .version directive in the text format identifies the
--     instruction set version (v1 = Brainfuck subset).
--
-- ## Module Structure
--
-- | Section          | Contents                                              |
-- |------------------|-------------------------------------------------------|
-- | IrOp             | Opcode enumeration (25 opcodes)                       |
-- | IrOpName         | Reverse map: opcode number → name string              |
-- | Operand helpers  | new_register, new_immediate, new_label, operand_tostring |
-- | IrInstruction    | new_instruction constructor                           |
-- | IrDataDecl       | new_data_decl constructor                             |
-- | IrProgram        | new_program, add_instruction, add_data                |
-- | IDGenerator      | new_id_generator, next_id, current_id                 |
-- | Printer          | print_ir(program) → canonical text                    |
-- | Parser           | parse_ir(text) → IrProgram                            |
--
-- ## Usage
--
--   local ir = require("coding_adventures.compiler_ir")
--
--   local prog = ir.new_program("_start")
--   local gen  = ir.new_id_generator()
--   ir.add_instruction(prog, ir.new_instruction(
--       ir.IrOp.LOAD_ADDR,
--       { ir.new_register(0), ir.new_label("tape") },
--       ir.next_id(gen)
--   ))
--   local text = ir.print_ir(prog)
--   local prog2 = ir.parse_ir(text)   -- roundtrip
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- IrOp — Opcode Enumeration
-- ============================================================================
--
-- Each opcode represents a single operation. Opcodes are grouped by category:
--
--   Constants:    LOAD_IMM, LOAD_ADDR
--   Memory:       LOAD_BYTE, STORE_BYTE, LOAD_WORD, STORE_WORD
--   Arithmetic:   ADD, ADD_IMM, SUB, AND, AND_IMM
--   Comparison:   CMP_EQ, CMP_NE, CMP_LT, CMP_GT
--   Control Flow: LABEL, JUMP, BRANCH_Z, BRANCH_NZ, CALL, RET
--   System:       SYSCALL, HALT
--   Meta:         NOP, COMMENT
--
-- The numeric values are fixed and must never change. New opcodes are only
-- ever appended at the end (before the max sentinel). This ensures forward
-- compatibility: programs compiled with an older IR version still have
-- the same opcode semantics in a newer runtime.

M.IrOp = {
    -- ── Constants ──────────────────────────────────────────────────────────
    -- Load an immediate integer value into a register.
    --   LOAD_IMM  v0, 42    →  v0 = 42
    LOAD_IMM   = 0,

    -- Load the address of a data label into a register.
    --   LOAD_ADDR v0, tape  →  v0 = &tape
    LOAD_ADDR  = 1,

    -- ── Memory ────────────────────────────────────────────────────────────
    -- Load a byte from memory (zero-extended).
    --   LOAD_BYTE v2, v0, v1  →  v2 = mem[v0 + v1] & 0xFF
    LOAD_BYTE  = 2,

    -- Store a byte to memory.
    --   STORE_BYTE v2, v0, v1  →  mem[v0 + v1] = v2 & 0xFF
    STORE_BYTE = 3,

    -- Load a machine word from memory.
    --   LOAD_WORD v2, v0, v1  →  v2 = *(int*)(v0 + v1)
    LOAD_WORD  = 4,

    -- Store a machine word to memory.
    --   STORE_WORD v2, v0, v1  →  *(int*)(v0 + v1) = v2
    STORE_WORD = 5,

    -- ── Arithmetic ────────────────────────────────────────────────────────
    -- Register-register addition: dst = lhs + rhs.
    --   ADD v3, v1, v2  →  v3 = v1 + v2
    ADD        = 6,

    -- Register-immediate addition: dst = src + immediate.
    --   ADD_IMM v1, v1, 1  →  v1 = v1 + 1
    ADD_IMM    = 7,

    -- Register-register subtraction: dst = lhs - rhs.
    --   SUB v3, v1, v2  →  v3 = v1 - v2
    SUB        = 8,

    -- Register-register bitwise AND: dst = lhs & rhs.
    --   AND v3, v1, v2  →  v3 = v1 & v2
    AND        = 9,

    -- Register-immediate bitwise AND: dst = src & immediate.
    --   AND_IMM v2, v2, 255  →  v2 = v2 & 0xFF
    AND_IMM    = 10,

    -- ── Comparison ────────────────────────────────────────────────────────
    -- Set dst = 1 if lhs == rhs, else 0.
    --   CMP_EQ v4, v1, v2  →  v4 = (v1 == v2) ? 1 : 0
    CMP_EQ     = 11,

    -- Set dst = 1 if lhs != rhs, else 0.
    --   CMP_NE v4, v1, v2  →  v4 = (v1 != v2) ? 1 : 0
    CMP_NE     = 12,

    -- Set dst = 1 if lhs < rhs (signed), else 0.
    --   CMP_LT v4, v1, v2  →  v4 = (v1 < v2) ? 1 : 0
    CMP_LT     = 13,

    -- Set dst = 1 if lhs > rhs (signed), else 0.
    --   CMP_GT v4, v1, v2  →  v4 = (v1 > v2) ? 1 : 0
    CMP_GT     = 14,

    -- ── Control Flow ──────────────────────────────────────────────────────
    -- Define a label at this point. Produces no machine code.
    --   LABEL loop_start
    LABEL      = 15,

    -- Unconditional jump to a label.
    --   JUMP loop_start  →  PC = &loop_start
    JUMP       = 16,

    -- Conditional branch: jump to label if register == 0.
    --   BRANCH_Z v2, loop_end  →  if v2 == 0 then PC = &loop_end
    BRANCH_Z   = 17,

    -- Conditional branch: jump to label if register != 0.
    --   BRANCH_NZ v2, loop_end  →  if v2 != 0 then PC = &loop_end
    BRANCH_NZ  = 18,

    -- Call a subroutine at the given label. Pushes return address.
    --   CALL my_func
    CALL       = 19,

    -- Return from a subroutine. Pops return address.
    --   RET
    RET        = 20,

    -- ── System ────────────────────────────────────────────────────────────
    -- Invoke a system call. The syscall number is an immediate operand.
    --   SYSCALL 1  →  ecall with a7=1 (write)
    SYSCALL    = 21,

    -- Halt execution. The program terminates.
    --   HALT  →  ecall with a7=10 (exit)
    HALT       = 22,

    -- ── Meta ──────────────────────────────────────────────────────────────
    -- No operation. Produces a single NOP instruction in the backend.
    --   NOP
    NOP        = 23,

    -- A human-readable comment. Produces no machine code.
    --   COMMENT "load tape base address"
    COMMENT    = 24,
}

-- ============================================================================
-- IrOpName — Reverse Map: Numeric Opcode → Name String
-- ============================================================================
--
-- This table is the inverse of IrOp. It lets the printer and tests convert
-- an opcode number back to its canonical text name without a linear scan.
--
-- Example:
--   M.IrOpName[0]   →  "LOAD_IMM"
--   M.IrOpName[22]  →  "HALT"

M.IrOpName = {}
for name, val in pairs(M.IrOp) do
    M.IrOpName[val] = name
end

-- ============================================================================
-- Operand Constructors and Helpers
-- ============================================================================
--
-- Operands are the arguments to IR instructions. There are three kinds:
--
--   register:  a virtual register (v0, v1, ...)
--   immediate: a literal integer value (42, -1, 255, ...)
--   label:     a named target (_start, loop_0_end, tape, ...)
--
-- Each operand is a plain Lua table with a `kind` discriminator field,
-- plus one data field (`index`, `value`, or `name`).
--
-- Operands are immutable by convention: create them with the constructors
-- below and do not modify them afterward.

--- Create a virtual register operand.
-- Virtual registers are named v0, v1, v2, ... (the `index` field).
-- There are infinitely many — the backend's register allocator maps them
-- to physical registers.
--
-- @param index number  The zero-based register index (0, 1, 2, ...).
-- @return table        { kind = "register", index = index }
--
-- Example:
--   new_register(0)  →  prints as "v0"
--   new_register(5)  →  prints as "v5"
function M.new_register(index)
    return { kind = "register", index = index }
end

--- Create an immediate (literal integer) operand.
-- Immediates are signed integers that appear directly in instructions.
--
-- @param value number  The integer value (may be negative).
-- @return table        { kind = "immediate", value = value }
--
-- Example:
--   new_immediate(42)   →  prints as "42"
--   new_immediate(-1)   →  prints as "-1"
--   new_immediate(255)  →  prints as "255"
function M.new_immediate(value)
    return { kind = "immediate", value = value }
end

--- Create a label operand.
-- Labels are strings like "_start", "loop_0_end", "tape", "__trap_oob".
-- They resolve to addresses during code generation.
--
-- @param name string  The label name.
-- @return table       { kind = "label", name = name }
--
-- Example:
--   new_label("_start")      →  prints as "_start"
--   new_label("loop_0_end")  →  prints as "loop_0_end"
function M.new_label(name)
    return { kind = "label", name = name }
end

--- Convert an operand to its canonical string representation.
-- Used by the printer and tests.
--
-- @param op table  An operand table (register, immediate, or label).
-- @return string   Human-readable text like "v0", "42", or "_start".
function M.operand_tostring(op)
    if op.kind == "register" then
        return "v" .. op.index
    elseif op.kind == "immediate" then
        return tostring(op.value)
    elseif op.kind == "label" then
        return op.name
    else
        return "???"
    end
end

-- ============================================================================
-- IrInstruction Constructor
-- ============================================================================
--
-- An IrInstruction represents a single IR operation. It has three fields:
--
--   opcode:   which operation to perform (a value from IrOp)
--   operands: array of operand tables (registers, immediates, labels)
--   id:       a unique monotonic integer used for source mapping.
--             Labels use id = -1 (they produce no machine code).
--
-- The `id` field is the key that connects an instruction to the source
-- map chain. Each instruction gets a unique ID assigned by the IDGenerator,
-- and that ID flows through all pipeline stages (IR → optimizer → backend).

--- Create a new IR instruction.
--
-- @param opcode   number  An IrOp value (e.g., M.IrOp.ADD_IMM).
-- @param operands table   Array of operand tables (may be empty or nil).
-- @param id       number  Unique instruction ID (use -1 for labels).
-- @return table           { opcode, operands, id }
function M.new_instruction(opcode, operands, id)
    return {
        opcode   = opcode,
        operands = operands or {},
        id       = id,
    }
end

-- ============================================================================
-- IrDataDecl Constructor
-- ============================================================================
--
-- A data declaration reserves a named region of memory with a fixed size
-- and initial byte value. In the Brainfuck compiler, this is the tape:
--
--   tape: 30,000 bytes, all initialized to 0.
--
-- In the text format this appears as:
--   .data tape 30000 0
--
-- The `init` field is the byte value that every cell is initialized to.
-- init=0 means zero-initialized (equivalent to .bss in ELF).

--- Create a new data segment declaration.
--
-- @param label  string  The name of the data region (e.g., "tape").
-- @param size   number  Number of bytes to allocate.
-- @param init   number  Initial byte value (0-255, default 0).
-- @return table         { label, size, init }
function M.new_data_decl(label, size, init)
    return {
        label = label,
        size  = size,
        init  = init or 0,
    }
end

-- ============================================================================
-- IrProgram — A Complete IR Program
-- ============================================================================
--
-- An IrProgram is the top-level container for compiled code. It holds:
--
--   instructions: the linear sequence of IR instructions (LABEL, ADD_IMM, ...)
--   data:         data segment declarations (.data tape 30000 0)
--   entry_label:  the label where execution begins (usually "_start")
--   version:      IR version number (1 = Brainfuck subset)
--
-- Instructions are ordered — execution flows from index 1 to #instructions,
-- with jumps and branches altering the flow.

--- Create a new, empty IR program.
--
-- @param entry_label string  The label where execution begins (e.g., "_start").
-- @return table              { instructions={}, data={}, entry_label, version=1 }
function M.new_program(entry_label)
    return {
        instructions = {},
        data         = {},
        entry_label  = entry_label,
        version      = 1,
    }
end

--- Append an instruction to the program.
-- Instructions are stored in the order they are added. The order matters:
-- the backend emits machine code in this exact order.
--
-- @param program table  An IrProgram table.
-- @param instr   table  An IrInstruction table.
function M.add_instruction(program, instr)
    table.insert(program.instructions, instr)
end

--- Append a data declaration to the program.
-- Data declarations define the initial contents of the data segment.
-- They appear before .entry in the text format.
--
-- @param program table  An IrProgram table.
-- @param decl    table  An IrDataDecl table.
function M.add_data(program, decl)
    table.insert(program.data, decl)
end

-- ============================================================================
-- IDGenerator — Produces Unique Monotonic Instruction IDs
-- ============================================================================
--
-- Every IR instruction in the pipeline needs a unique ID for source mapping.
-- The IDGenerator ensures no two instructions ever share an ID within a
-- single compilation unit.
--
-- The generator is a simple counter starting at 0. Call next_id() to get
-- the next ID and advance the counter. Call current_id() to peek at the
-- counter without advancing it.
--
-- Usage:
--   local gen = new_id_generator()
--   local id0 = next_id(gen)   -- 0
--   local id1 = next_id(gen)   -- 1
--   local id2 = next_id(gen)   -- 2
--   -- current_id(gen) == 3 now

--- Create a new ID generator starting at 0.
--
-- @return table  { _next = 0 }
function M.new_id_generator()
    return { _next = 0 }
end

--- Create a new ID generator starting at `start`.
-- Use this when multiple compilers contribute instructions to the same
-- program and IDs must not collide.
--
-- @param start number  The first ID to return.
-- @return table        { _next = start }
function M.new_id_generator_from(start)
    return { _next = start }
end

--- Return the next unique ID and advance the counter.
-- Each call returns a different value. IDs are monotonically increasing
-- and never reused within the same generator.
--
-- @param gen table  An ID generator table.
-- @return number    The next ID.
function M.next_id(gen)
    local id = gen._next
    gen._next = gen._next + 1
    return id
end

--- Return the current counter value without advancing it.
-- This is the ID that will be returned by the next call to next_id().
-- Useful for recording "how many IDs have been issued so far".
--
-- @param gen table  An ID generator table.
-- @return number    The current counter value.
function M.current_id(gen)
    return gen._next
end

-- ============================================================================
-- Printer — IrProgram → Canonical Text Format
-- ============================================================================
--
-- The printer converts an IrProgram into its canonical text representation.
-- This format serves three purposes:
--
--   1. Debugging: humans can read the IR to understand what the compiler did
--   2. Golden-file tests: expected IR output is committed as .ir text files
--   3. Roundtrip: parse(print(program)) == program is a testable invariant
--
-- ## Text Format
--
--   .version 1
--
--   .data tape 30000 0
--
--   .entry _start
--
--   _start:
--     LOAD_ADDR   v0, tape          ; #0
--     LOAD_IMM    v1, 0             ; #1
--     HALT                          ; #2
--
-- ## Key Rules
--
--   - .version N is always the first line
--   - .data declarations come before .entry
--   - Labels are on their own line with a trailing colon
--   - Instructions are indented with two spaces
--   - ; #N comments show instruction IDs (informational, not required)
--   - COMMENT instructions emit as "  ; <text>" on their own line

--- Convert an IrProgram to its canonical text representation.
--
-- @param program table  An IrProgram table.
-- @return string        The canonical IR text.
function M.print_ir(program)
    local lines = {}

    -- Version directive (always first)
    lines[#lines + 1] = string.format(".version %d", program.version)

    -- Data declarations (one per .data line)
    for _, d in ipairs(program.data) do
        lines[#lines + 1] = ""
        lines[#lines + 1] = string.format(".data %s %d %d", d.label, d.size, d.init)
    end

    -- Entry point
    lines[#lines + 1] = ""
    lines[#lines + 1] = string.format(".entry %s", program.entry_label)

    -- Instructions
    for _, instr in ipairs(program.instructions) do
        local op = instr.opcode

        if op == M.IrOp.LABEL then
            -- Labels get their own unindented line with a trailing colon.
            -- They are preceded by a blank line for readability.
            local label_name = M.operand_tostring(instr.operands[1])
            lines[#lines + 1] = ""
            lines[#lines + 1] = label_name .. ":"

        elseif op == M.IrOp.COMMENT then
            -- Comments emit as "  ; <text>" — no ID suffix.
            local text = ""
            if instr.operands and instr.operands[1] then
                text = M.operand_tostring(instr.operands[1])
            end
            lines[#lines + 1] = "  ; " .. text

        else
            -- Regular instruction: "  OPCODE      op1, op2, ...  ; #ID"
            -- The opcode name is left-padded to 11 characters for alignment.
            local op_name = M.IrOpName[op] or "UNKNOWN"
            local padded  = string.format("%-11s", op_name)

            -- Operands joined by ", "
            local parts = {}
            for _, operand in ipairs(instr.operands) do
                parts[#parts + 1] = M.operand_tostring(operand)
            end
            local operand_str = table.concat(parts, ", ")

            -- Build the full line
            local line = "  " .. padded
            if #operand_str > 0 then
                line = line .. operand_str
            end
            line = line .. "  ; #" .. instr.id
            lines[#lines + 1] = line
        end
    end

    -- Join lines with newlines and add a trailing newline (matching Go printer)
    return table.concat(lines, "\n") .. "\n"
end

-- ============================================================================
-- Parser — Canonical Text → IrProgram
-- ============================================================================
--
-- The parser reads the canonical IR text format (produced by print_ir) and
-- reconstructs an IrProgram. This enables:
--
--   1. Golden-file testing: load an expected .ir file, parse it, compare.
--   2. Roundtrip verification: parse(print(prog)) == prog.
--   3. Manual IR authoring: write IR by hand for testing backends.
--
-- ## Parsing Strategy (line by line)
--
--   1. ".version N"    → set program.version = N
--   2. ".data L S I"   → add_data(new_data_decl(L, S, I))
--   3. ".entry L"      → set program.entry_label = L
--   4. "L:" (no ";")   → add_instruction(LABEL, [new_label(L)], -1)
--   5. "; text"        → add_instruction(COMMENT, [new_label(text)], -1)
--   6. whitespace-led  → parse_instruction_line(trimmed_line)
--   7. blank           → skip
--
-- Operands are parsed as:
--   - "vN"  where N is digits → register
--   - integer (parseable by tonumber) → immediate
--   - anything else → label

--- Parse an IR text string and return an IrProgram.
-- Returns the program on success, or nil and an error message on failure.
--
-- @param text string  The IR text to parse.
-- @return table|nil   An IrProgram on success, nil on failure.
-- @return string|nil  An error message on failure, nil on success.
function M.parse_ir(text)
    local program = {
        instructions = {},
        data         = {},
        entry_label  = "",
        version      = 1,
    }

    local lines = {}
    -- Split on both \n and \r\n to handle different line endings.
    for line in (text .. "\n"):gmatch("([^\n]*)\n") do
        lines[#lines + 1] = line
    end

    -- Safety limit: prevent excessive memory use on adversarial input.
    local MAX_LINES = 1000000
    if #lines > MAX_LINES then
        return nil, string.format(
            "input too large: %d lines (max %d)", #lines, MAX_LINES)
    end

    for line_num, line in ipairs(lines) do
        local trimmed = line:match("^%s*(.-)%s*$")  -- trim whitespace

        -- Blank lines are always skipped.
        if trimmed == "" then
            goto continue
        end

        -- Version directive: ".version N"
        if trimmed:sub(1, 8) == ".version" then
            local n = trimmed:match("^%.version%s+(%S+)$")
            if not n then
                return nil, string.format(
                    "line %d: invalid .version directive: %q", line_num, line)
            end
            local v = tonumber(n)
            if not v or math.type(v) ~= "integer" or v ~= math.floor(v) then
                -- Accept floats that are whole numbers (e.g., 1.0)
                v = math.floor(tonumber(n) or 0)
                if v == 0 and n ~= "0" then
                    return nil, string.format(
                        "line %d: invalid version number: %q", line_num, n)
                end
            end
            program.version = math.floor(tonumber(n))
            goto continue
        end

        -- Data declaration: ".data label size init"
        if trimmed:sub(1, 5) == ".data" then
            local label, size_s, init_s =
                trimmed:match("^%.data%s+(%S+)%s+(%S+)%s+(%S+)$")
            if not label then
                return nil, string.format(
                    "line %d: invalid .data directive: %q", line_num, line)
            end
            local size_n = tonumber(size_s)
            local init_n = tonumber(init_s)
            if not size_n then
                return nil, string.format(
                    "line %d: invalid data size: %q", line_num, size_s)
            end
            if not init_n then
                return nil, string.format(
                    "line %d: invalid data init: %q", line_num, init_s)
            end
            table.insert(program.data, M.new_data_decl(
                label, math.floor(size_n), math.floor(init_n)))
            goto continue
        end

        -- Entry point: ".entry label"
        if trimmed:sub(1, 6) == ".entry" then
            local lbl = trimmed:match("^%.entry%s+(%S+)$")
            if not lbl then
                return nil, string.format(
                    "line %d: invalid .entry directive: %q", line_num, line)
            end
            program.entry_label = lbl
            goto continue
        end

        -- Label definition: "name:" (does not start with ";")
        if trimmed:sub(-1) == ":" and trimmed:sub(1, 1) ~= ";" then
            local label_name = trimmed:sub(1, -2)
            table.insert(program.instructions, M.new_instruction(
                M.IrOp.LABEL,
                { M.new_label(label_name) },
                -1
            ))
            goto continue
        end

        -- Standalone comment: "; text" (but not "; #N" which is an ID comment)
        if trimmed:sub(1, 1) == ";" then
            local comment_text = trimmed:sub(2):match("^%s*(.-)%s*$")
            -- Skip pure ID comments like "; #42" — those are attached to
            -- instructions, not standalone comment instructions.
            if comment_text:sub(1, 1) ~= "#" then
                table.insert(program.instructions, M.new_instruction(
                    M.IrOp.COMMENT,
                    { M.new_label(comment_text) },
                    -1
                ))
            end
            goto continue
        end

        -- Instruction line (starts with whitespace or is otherwise non-directive)
        do
            local instr, err = M._parse_instruction_line(trimmed, line_num)
            if not instr then
                return nil, err
            end
            table.insert(program.instructions, instr)
        end

        ::continue::
    end

    return program, nil
end

--- Parse a single instruction line like "LOAD_IMM   v0, 42  ; #3".
-- This is an internal helper used by parse_ir.
--
-- Parsing steps:
--   1. Strip the "; #N" ID comment from the end (if present).
--   2. Split on whitespace to get the opcode name.
--   3. Look up the opcode in the name table.
--   4. Parse each operand (comma-separated).
--
-- @param line     string  The trimmed instruction line (no leading/trailing space).
-- @param line_num number  The 1-based line number (for error messages).
-- @return table|nil  An IrInstruction on success, nil on failure.
-- @return string|nil An error message on failure.
function M._parse_instruction_line(line, line_num)
    -- Extract the "; #N" ID comment if present.
    local id = -1
    local instruction_part = line
    local id_start = line:find("; #")
    if id_start then
        local id_str = line:sub(id_start + 3):match("^%s*(%d+)%s*$")
        if id_str then
            id = tonumber(id_str)
        end
        instruction_part = line:sub(1, id_start - 1):match("^(.-)%s*$")
    end

    -- Split into fields (opcode + operands).
    local fields = {}
    for f in instruction_part:gmatch("%S+") do
        fields[#fields + 1] = f
    end
    if #fields == 0 then
        return nil, string.format("line %d: empty instruction", line_num)
    end

    -- Look up the opcode.
    local opcode_name = fields[1]
    local opcode = nil
    for name, val in pairs(M.IrOp) do
        if name == opcode_name then
            opcode = val
            break
        end
    end
    if opcode == nil then
        return nil, string.format(
            "line %d: unknown opcode %q", line_num, opcode_name)
    end

    -- Parse operands: everything after the opcode, comma-separated.
    -- We rejoin fields[2..] to handle "v0, v1, 42" where commas may be
    -- attached to tokens or separate.
    local operands = {}
    if #fields > 1 then
        local operand_str = table.concat(fields, " ", 2)
        -- Split by comma, handling optional spaces around commas.
        for raw_part in operand_str:gmatch("[^,]+") do
            local part = raw_part:match("^%s*(.-)%s*$")  -- trim
            if part ~= "" then
                local op, err = M._parse_operand(part, line_num)
                if not op then
                    return nil, err
                end
                operands[#operands + 1] = op
            end
        end
    end

    return M.new_instruction(opcode, operands, id), nil
end

--- Parse a single operand string into an operand table.
--
-- Parsing rules (in order):
--   1. Starts with "v" followed by digits → register (e.g., "v0" → register{0})
--   2. Parseable as an integer → immediate (e.g., "42" → immediate{42})
--   3. Anything else → label (e.g., "_start" → label{"_start"})
--
-- @param s        string  The operand string.
-- @param line_num number  Line number (for error messages).
-- @return table|nil  An operand table on success, nil on failure.
-- @return string|nil An error message on failure.
function M._parse_operand(s, line_num)
    -- Register: "v0", "v1", "v12", ...
    if #s > 1 and s:sub(1, 1) == "v" then
        local digits = s:sub(2)
        local idx = tonumber(digits)
        -- Check it's a valid non-negative integer with only digit characters
        if idx and digits:match("^%d+$") then
            if idx < 0 or idx > 65535 then
                return nil, string.format(
                    "line %d: register index %d out of range (max 65535)",
                    line_num, idx)
            end
            return M.new_register(idx), nil
        end
        -- Not a valid register number — fall through to label.
    end

    -- Immediate: an integer (possibly negative, like "-1").
    local n = tonumber(s)
    if n and (s:match("^-?%d+$")) then
        return M.new_immediate(math.floor(n)), nil
    end

    -- Label: anything else (identifiers, names with underscores, etc.)
    return M.new_label(s), nil
end

return M
