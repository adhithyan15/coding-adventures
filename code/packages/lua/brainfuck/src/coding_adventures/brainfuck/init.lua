-- ============================================================================
-- brainfuck — Brainfuck interpreter and bytecode compiler
-- ============================================================================
--
-- Brainfuck is a Turing-complete programming language invented by Urban
-- Müller in 1993.  It has exactly 8 commands — one per printable ASCII
-- character — and is deliberately minimal to the point of absurdity.  Its
-- educational value is inversely proportional to its usability: by being so
-- stripped down, it forces you to understand the absolute fundamentals of
-- how a computer works:
--
--   - Memory is a flat tape of cells (like RAM).
--   - A pointer keeps track of which cell we're operating on.
--   - Programs are sequences of operations on that pointer and cell.
--   - Loops repeat until a condition is false.
--
-- ## The 8 Commands
--
--   Command  | Meaning                                      | C equivalent
--   ---------+----------------------------------------------+--------------
--   >        | Move data pointer right                      | ++ptr
--   <        | Move data pointer left                       | --ptr
--   +        | Increment byte at current cell               | (*ptr)++
--   -        | Decrement byte at current cell               | (*ptr)--
--   .        | Output current cell as ASCII character       | putchar(*ptr)
--   ,        | Read one byte of input into current cell     | *ptr = getchar()
--   [        | If current cell is 0, jump past matching ]   | while (*ptr) {
--   ]        | If current cell is nonzero, jump back to [   | }
--
-- Any character that is not one of these 8 is a *comment* and is ignored.
-- Brainfuck programs can contain arbitrary text as documentation!
--
-- ## The Tape
--
-- The tape is an array of 30,000 byte cells, all initialised to 0.  Cell
-- values are unsigned bytes: they wrap from 255 to 0 on increment and from
-- 0 to 255 on decrement.  The data pointer (DP) starts at cell 0.
--
-- ## Equivalent C Program for "Hello, World!"
--
-- The classic Brainfuck "Hello World" program looks like this in C:
--
--   char tape[30000] = {0};
--   char *ptr = tape;
--   // Brainfuck's [ ] → C while; + - → (*ptr)++ (*ptr)--; . → putchar
--
-- ## What This Module Provides
--
-- | Function                    | Purpose                                   |
-- |-----------------------------|-------------------------------------------|
-- | validate(program)           | Check for balanced brackets               |
-- | compile_to_opcodes(program) | Translate source to opcode list           |
-- | run_opcodes(opcodes, input) | Execute compiled opcodes                  |
-- | interpret(program, input)   | Validate + compile + run in one call      |
--
-- ## Two-Phase Execution
--
-- We separate compilation from execution for performance:
--
--   Phase 1 — compile_to_opcodes:  translate the 8 commands to integer opcodes
--             and pre-compute jump targets for [ and ].  This turns O(n)
--             bracket-matching scans at runtime into O(1) jumps.
--
--   Phase 2 — run_opcodes: execute the opcode list using a simple eval loop.
--
-- ## Usage
--
--   local bf = require("coding_adventures.brainfuck")
--
--   -- Simple: one call
--   local output = bf.interpret("+++.", "")   -- outputs char(3) = ETX
--
--   -- Compiled: reuse opcodes for multiple runs
--   local opcodes, err = bf.compile_to_opcodes(",[.,]")
--   if err then error(err) end
--   local out = bf.run_opcodes(opcodes, "hello")   -- "hello"
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"
M.lang_vm = require("coding_adventures.brainfuck.lang_vm")

-- ============================================================================
-- Opcode Constants
-- ============================================================================
--
-- We map each Brainfuck character to a numeric opcode.  This is exactly
-- what a real compiler does: source characters → instruction codes.
--
-- We mirror the Go implementation's opcode numbers so that bytecode is
-- conceptually compatible across language implementations.

M.OP_RIGHT      = 0x01   -- >  move data pointer right
M.OP_LEFT       = 0x02   -- <  move data pointer left
M.OP_INC        = 0x03   -- +  increment current cell
M.OP_DEC        = 0x04   -- -  decrement current cell
M.OP_OUTPUT     = 0x05   -- .  output cell as ASCII
M.OP_INPUT      = 0x06   -- ,  read input into cell
M.OP_LOOP_START = 0x07   -- [  jump forward if cell == 0
M.OP_LOOP_END   = 0x08   -- ]  jump backward if cell != 0
M.OP_HALT       = 0xFF   -- end of program

--- CHAR_TO_OP maps Brainfuck source characters to their opcodes.
-- Any character NOT in this table is a comment and is silently ignored.
local CHAR_TO_OP = {
    [">"] = M.OP_RIGHT,
    ["<"] = M.OP_LEFT,
    ["+"] = M.OP_INC,
    ["-"] = M.OP_DEC,
    ["."] = M.OP_OUTPUT,
    [","] = M.OP_INPUT,
    ["["] = M.OP_LOOP_START,
    ["]"] = M.OP_LOOP_END,
}

-- ============================================================================
-- Tape size
-- ============================================================================
--
-- The canonical Brainfuck tape has 30,000 cells.  This matches the
-- original specification and every mainstream implementation.

local TAPE_SIZE = 30000

-- ============================================================================
-- validate — Check for balanced brackets
-- ============================================================================

--- validate checks that all [ ] brackets in the program are balanced.
--
-- Balanced means:
--   - Every [ has a matching ] that comes after it.
--   - Every ] has a matching [ that comes before it.
--   - Brackets properly nest: [[][]] is valid; []] is not.
--
-- We use a counter (depth):
--   - [ increases depth by 1.
--   - ] decreases depth by 1.
--   - If depth goes negative, we found a ] before any [.
--   - If depth > 0 at the end, there are unclosed [s.
--
-- @param program  string  The Brainfuck source code.
-- @return         boolean true if valid, false if not.
-- @return         string  Error message if invalid, nil if valid.
function M.validate(program)
    local depth = 0
    for i = 1, #program do
        local ch = program:sub(i, i)
        if ch == "[" then
            depth = depth + 1
        elseif ch == "]" then
            depth = depth - 1
            if depth < 0 then
                return false, string.format("unmatched ']' at position %d", i)
            end
        end
    end
    if depth > 0 then
        return false, string.format("%d unclosed '[' bracket(s)", depth)
    end
    return true, nil
end

-- ============================================================================
-- compile_to_opcodes — Translate source to opcode list with jump targets
-- ============================================================================

--- compile_to_opcodes translates a Brainfuck source string into an opcode
-- list.  Each element of the list is a table:
--
--   { op = <opcode number>, operand = <jump target or nil> }
--
-- For OP_LOOP_START and OP_LOOP_END, the operand holds the instruction
-- index (1-based, Lua convention) of the matching bracket:
--
--   OP_LOOP_START.operand = index of instruction after matching ]
--   OP_LOOP_END.operand   = index of matching [
--
-- This pre-computation is the key optimisation: instead of scanning the
-- instruction list at runtime to find the matching bracket (O(n) per
-- iteration), we look it up in O(1).
--
-- ## Algorithm: Stack-Based Bracket Matching
--
--   For each instruction in the compiled list:
--     - On [: push its index onto a stack.
--     - On ]: pop the index of the matching [.
--              Patch both instructions with each other's targets.
--
-- @param program  string  Brainfuck source code.
-- @return         table   Array of opcode tables, or nil on error.
-- @return         string  Error message, or nil on success.
function M.compile_to_opcodes(program)
    -- Validate first: bail out on unbalanced brackets.
    local ok, err = M.validate(program)
    if not ok then return nil, err end

    -- First pass: build the raw opcode list (no jump targets yet).
    local opcodes = {}
    for i = 1, #program do
        local ch  = program:sub(i, i)
        local op  = CHAR_TO_OP[ch]
        if op then
            table.insert(opcodes, { op = op, operand = nil })
        end
    end
    -- Append HALT so the executor knows when to stop.
    table.insert(opcodes, { op = M.OP_HALT, operand = nil })

    -- Second pass: resolve [ ] jump targets using a stack.
    --
    -- The stack holds indices (into opcodes) of unmatched [ instructions.
    -- When we encounter ], we pop from the stack to find the matching [.
    local stack = {}
    for i = 1, #opcodes do
        local instr = opcodes[i]
        if instr.op == M.OP_LOOP_START then
            -- Remember this [ for when we find its matching ].
            table.insert(stack, i)
        elseif instr.op == M.OP_LOOP_END then
            -- Pop the matching [.
            local open_idx = table.remove(stack)
            -- [ jumps to instruction AFTER ] if cell is 0.
            opcodes[open_idx].operand = i + 1
            -- ] jumps back to the [ if cell is nonzero.
            instr.operand = open_idx
        end
    end

    return opcodes, nil
end

-- ============================================================================
-- run_opcodes — Execute a compiled opcode list
-- ============================================================================

--- run_opcodes executes a compiled opcode list and returns the output string.
--
-- ## State
--
--   tape[1..TAPE_SIZE]  — 30,000 byte cells, all 0 initially.
--   dp                  — data pointer, 1-based index into tape.
--   pc                  — program counter, 1-based index into opcodes.
--   input_pos           — current position in the input string (1-based).
--   output              — list of output characters, joined at the end.
--
-- ## Cell Wrapping
--
-- Cells are unsigned bytes (0–255):
--
--   INC: (cell + 1) % 256  →  255 + 1 = 0   (wraps around)
--   DEC: (cell - 1 + 256) % 256  →  0 - 1 = 255 (wraps around)
--
-- The + 256 before the modulus for DEC prevents negative results in Lua's %
-- operator (which, unlike Python's, can produce negative values when the
-- dividend is negative).  This matches the Go implementation's same fix.
--
-- ## EOF Handling
--
-- When the , command is executed and there is no more input, we set the cell
-- to 0.  This is the most common convention (used by gcc's brainfuck port and
-- most interpreters) and makes the cat program ,[.,] terminate naturally.
--
-- @param opcodes   table   Compiled opcode list from compile_to_opcodes().
-- @param input_str string  Input data (treated as a stream of bytes).
-- @return          string  Output produced by . commands.
function M.run_opcodes(opcodes, input_str)
    input_str = input_str or ""

    -- Initialise tape: 30,000 zero cells.
    local tape = {}
    for _ = 1, TAPE_SIZE do tape[#tape + 1] = 0 end

    local dp        = 1            -- data pointer (1-based)
    local pc        = 1            -- program counter (1-based)
    local input_pos = 1            -- next input byte position (1-based)
    local output    = {}           -- collected output characters

    while pc <= #opcodes do
        local instr = opcodes[pc]
        local op    = instr.op

        -- -------------------------------------------------------------------
        -- > — Move data pointer right
        -- -------------------------------------------------------------------
        if op == M.OP_RIGHT then
            dp = dp + 1
            if dp > TAPE_SIZE then
                error(string.format(
                    "BrainfuckError: data pointer moved past end of tape at instruction %d", pc))
            end
            pc = pc + 1

        -- -------------------------------------------------------------------
        -- < — Move data pointer left
        -- -------------------------------------------------------------------
        elseif op == M.OP_LEFT then
            dp = dp - 1
            if dp < 1 then
                error("BrainfuckError: data pointer moved before start of tape")
            end
            pc = pc + 1

        -- -------------------------------------------------------------------
        -- + — Increment current cell (wraps 255 → 0)
        -- -------------------------------------------------------------------
        elseif op == M.OP_INC then
            tape[dp] = (tape[dp] + 1) % 256
            pc = pc + 1

        -- -------------------------------------------------------------------
        -- - — Decrement current cell (wraps 0 → 255)
        -- -------------------------------------------------------------------
        elseif op == M.OP_DEC then
            tape[dp] = (tape[dp] - 1 + 256) % 256
            pc = pc + 1

        -- -------------------------------------------------------------------
        -- . — Output current cell as ASCII
        -- -------------------------------------------------------------------
        elseif op == M.OP_OUTPUT then
            output[#output + 1] = string.char(tape[dp])
            pc = pc + 1

        -- -------------------------------------------------------------------
        -- , — Read one byte of input into current cell
        --
        -- EOF → cell = 0  (clean convention; makes ,[.,] cat loop terminate)
        -- -------------------------------------------------------------------
        elseif op == M.OP_INPUT then
            if input_pos <= #input_str then
                tape[dp] = input_str:byte(input_pos)
                input_pos = input_pos + 1
            else
                tape[dp] = 0   -- EOF
            end
            pc = pc + 1

        -- -------------------------------------------------------------------
        -- [ — Jump forward past matching ] if cell is 0
        --
        -- If the current cell is zero, we skip the entire loop body.
        -- The operand holds the instruction index AFTER the matching ].
        -- -------------------------------------------------------------------
        elseif op == M.OP_LOOP_START then
            if tape[dp] == 0 then
                pc = instr.operand   -- jump past ]
            else
                pc = pc + 1          -- enter loop body
            end

        -- -------------------------------------------------------------------
        -- ] — Jump back to matching [ if cell is nonzero
        --
        -- If the current cell is nonzero, we loop back.
        -- The operand holds the index of the matching [.
        -- -------------------------------------------------------------------
        elseif op == M.OP_LOOP_END then
            if tape[dp] ~= 0 then
                pc = instr.operand   -- jump back to [
            else
                pc = pc + 1          -- exit loop
            end

        -- -------------------------------------------------------------------
        -- HALT — Stop execution
        -- -------------------------------------------------------------------
        elseif op == M.OP_HALT then
            break

        else
            error(string.format("BrainfuckError: unknown opcode 0x%02x at instruction %d", op, pc))
        end
    end

    return table.concat(output)
end

-- ============================================================================
-- interpret — High-level convenience function
-- ============================================================================

--- interpret validates, compiles, and executes a Brainfuck program in one call.
--
-- This is the "batteries-included" entry point: most users never need to
-- call compile_to_opcodes or run_opcodes directly.
--
-- @param program   string  Brainfuck source code.
-- @param input_str string  Input data (default "").
-- @return          string  Output produced by the program, or nil on error.
-- @return          string  Error message, or nil on success.
function M.interpret(program, input_str)
    input_str = input_str or ""

    local opcodes, err = M.compile_to_opcodes(program)
    if err then return nil, err end

    local ok, result = pcall(M.run_opcodes, opcodes, input_str)
    if not ok then
        return nil, result
    end
    return result, nil
end

return M
