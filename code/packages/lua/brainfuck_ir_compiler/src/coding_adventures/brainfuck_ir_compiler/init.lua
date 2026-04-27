-- ============================================================================
-- brainfuck_ir_compiler — Brainfuck AOT compiler: AST → IR
-- ============================================================================
--
-- This module is the Brainfuck-specific frontend of the AOT compiler
-- pipeline. It takes an AST produced by the Brainfuck parser and emits
-- general-purpose IR instructions from the compiler_ir package.
--
-- It also builds the first two segments of the source map chain:
--   Segment 1: SourceToAst  (source positions → AST node IDs)
--   Segment 2: AstToIr      (AST node IDs → IR instruction IDs)
--
-- This compiler knows Brainfuck semantics (tape, cells, pointer, loops,
-- I/O) and translates them into target-independent IR. It does NOT know
-- about RISC-V, ARM, ELF, or any specific machine target.
--
-- ## Register Allocation
--
-- Brainfuck needs very few virtual registers:
--
--   v0 = tape base address (pointer to start of tape)
--   v1 = tape pointer offset (current cell index, 0-based)
--   v2 = temporary (cell value for loads/stores)
--   v3 = temporary (for bounds checks)
--   v4 = temporary (for syscall arguments)
--   v5 = max pointer value (tape_size - 1, for bounds checks)
--   v6 = zero constant (0, for bounds checks)
--
-- ## Syscall Numbers
--
--   1 = write byte in a0 to stdout
--   2 = read byte from stdin into a0
--   10 = halt with exit code in a0
--
-- ## Usage
--
--   local bic = require("coding_adventures.brainfuck_ir_compiler")
--   local bf_parser = require("coding_adventures.brainfuck.parser")
--
--   local ast = bf_parser.parse("+.")
--   local config = bic.release_config()
--   local result, err = bic.compile(ast, "hello.bf", config)
--   if err then error(err) end
--
--   -- result.program  → IrProgram
--   -- result.source_map → SourceMapChain
--
--   local ir = require("coding_adventures.compiler_ir")
--   local text = ir.print_ir(result.program)
--   print(text)
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

local ir = require("coding_adventures.compiler_ir")
local sm = require("coding_adventures.compiler_source_map")

-- ============================================================================
-- Register Constants
-- ============================================================================
--
-- These fixed virtual register indices are used throughout the compiler.
-- Future languages (BASIC) that need more registers will use a register
-- allocator in the backend instead of fixed assignments.

local REG_TAPE_BASE = 0  -- v0: base address of the tape
local REG_TAPE_PTR  = 1  -- v1: current cell offset (0-based index)
local REG_TEMP      = 2  -- v2: temporary for cell values
local REG_TEMP2     = 3  -- v3: temporary for bounds checks
local REG_SYS_ARG   = 4  -- v4: syscall argument register
local REG_MAX_PTR   = 5  -- v5: tape_size - 1 (for bounds checks)
local REG_ZERO      = 6  -- v6: constant 0 (for bounds checks)

-- ============================================================================
-- Syscall Numbers
-- ============================================================================
--
-- These match the RISC-V simulator's ecall dispatch table.
-- They are fixed and must not change.

local SYSCALL_WRITE = 1   -- write byte in v4 to stdout
local SYSCALL_READ  = 2   -- read byte from stdin into v4
local SYSCALL_EXIT  = 10  -- halt with exit code in v4

-- ============================================================================
-- BuildConfig — Controls What the Compiler Emits
-- ============================================================================
--
-- Build modes are **composable flags**, not a fixed enum. A BuildConfig
-- table controls every aspect of compilation:
--
--   insert_bounds_checks: emit tape pointer range checks (debug builds)
--   insert_debug_locs:    emit source location markers
--   mask_byte_arithmetic: AND 0xFF after every cell mutation (correctness)
--   tape_size:            configurable tape length (default 30,000 cells)
--
-- Presets:
--   debug_config:   bounds checks ON, debug locs ON, masking ON
--   release_config: bounds checks OFF, debug locs OFF, masking ON

--- Create a BuildConfig with the given options.
--
-- @param opts table  Option table. Supported fields:
--   insert_bounds_checks (bool, default false)
--   insert_debug_locs    (bool, default false)
--   mask_byte_arithmetic (bool, default true)
--   tape_size            (number, default 30000)
-- @return table  A BuildConfig table.
function M.new_build_config(opts)
    opts = opts or {}
    return {
        insert_bounds_checks = opts.insert_bounds_checks or false,
        insert_debug_locs    = opts.insert_debug_locs or false,
        -- mask_byte_arithmetic defaults to true (safe default)
        mask_byte_arithmetic = (opts.mask_byte_arithmetic ~= false),
        tape_size            = opts.tape_size or 30000,
    }
end

--- Return a BuildConfig suitable for debug builds.
-- All safety checks are enabled.
--
-- @return table  A BuildConfig with:
--   insert_bounds_checks = true
--   insert_debug_locs    = true
--   mask_byte_arithmetic = true
--   tape_size            = 30000
function M.debug_config()
    return M.new_build_config({
        insert_bounds_checks = true,
        insert_debug_locs    = true,
        mask_byte_arithmetic = true,
        tape_size            = 30000,
    })
end

--- Return a BuildConfig suitable for release builds.
-- Safety checks are disabled for maximum performance.
--
-- @return table  A BuildConfig with:
--   insert_bounds_checks = false
--   insert_debug_locs    = false
--   mask_byte_arithmetic = true
--   tape_size            = 30000
function M.release_config()
    return M.new_build_config({
        insert_bounds_checks = false,
        insert_debug_locs    = false,
        mask_byte_arithmetic = true,
        tape_size            = 30000,
    })
end

-- ============================================================================
-- CompileResult
-- ============================================================================
--
-- compile() returns a CompileResult on success (or nil + error string on fail).
-- A CompileResult has two fields:
--   program:    an IrProgram (from compiler_ir)
--   source_map: a SourceMapChain (from compiler_source_map)

--- Create a new CompileResult.
-- (Internal helper; callers receive this from compile().)
local function new_compile_result(program, source_map)
    return {
        program    = program,
        source_map = source_map,
    }
end

-- ============================================================================
-- Internal Compiler Object
-- ============================================================================
--
-- The compiler is a table that holds all mutable state during a single
-- compilation. Creating a new compiler for each call to compile() ensures
-- that compiler state never leaks between invocations.

local Compiler = {}
Compiler.__index = Compiler

--- Create a new internal compiler instance.
local function new_compiler(config, filename)
    return setmetatable({
        config      = config,
        filename    = filename,
        id_gen      = ir.new_id_generator(),
        node_id_gen = 0,
        program     = ir.new_program("_start"),
        source_map  = sm.new_source_map_chain(),
        loop_count  = 0,
    }, Compiler)
end

--- Return the next unique AST node ID.
function Compiler:next_node_id()
    local id = self.node_id_gen
    self.node_id_gen = self.node_id_gen + 1
    return id
end

--- Emit an IR instruction with the given opcode and operands.
-- Returns the new instruction's unique ID.
--
-- @param opcode   number  An ir.IrOp value.
-- @param operands table   Array of operand tables.
-- @return number          The instruction's unique ID.
function Compiler:emit(opcode, operands)
    local id = ir.next_id(self.id_gen)
    ir.add_instruction(self.program,
        ir.new_instruction(opcode, operands, id))
    return id
end

--- Emit a LABEL instruction (labels have ID -1 — they produce no machine code).
--
-- @param name string  The label name (e.g., "_start", "loop_0_start").
function Compiler:emit_label(name)
    ir.add_instruction(self.program,
        ir.new_instruction(ir.IrOp.LABEL, { ir.new_label(name) }, -1))
end

-- ============================================================================
-- Prologue and Epilogue
-- ============================================================================
--
-- The prologue sets up the execution environment:
--   - Load the tape base address into v0
--   - Set the tape pointer to 0 in v1
--   - (debug) Set max pointer and zero constant
--
-- The epilogue terminates the program cleanly:
--   - HALT instruction
--   - (debug) __trap_oob handler

function Compiler:emit_prologue()
    self:emit_label("_start")

    -- v0 = &tape  (base address of the tape in memory)
    self:emit(ir.IrOp.LOAD_ADDR, {
        ir.new_register(REG_TAPE_BASE),
        ir.new_label("tape"),
    })

    -- v1 = 0  (tape pointer starts at cell 0)
    self:emit(ir.IrOp.LOAD_IMM, {
        ir.new_register(REG_TAPE_PTR),
        ir.new_immediate(0),
    })

    -- Debug mode: set up registers for bounds checks.
    if self.config.insert_bounds_checks then
        -- v5 = tape_size - 1  (max valid pointer index)
        self:emit(ir.IrOp.LOAD_IMM, {
            ir.new_register(REG_MAX_PTR),
            ir.new_immediate(self.config.tape_size - 1),
        })
        -- v6 = 0  (used as the lower bound in CMP_LT v1, v1, v6)
        self:emit(ir.IrOp.LOAD_IMM, {
            ir.new_register(REG_ZERO),
            ir.new_immediate(0),
        })
    end
end

function Compiler:emit_epilogue()
    -- Normal termination: HALT instruction.
    self:emit(ir.IrOp.HALT, {})

    -- Debug mode: emit the out-of-bounds trap handler.
    -- This is a subroutine that is only reached when a bounds check fails.
    if self.config.insert_bounds_checks then
        self:emit_label("__trap_oob")
        -- Load exit code 1 into the syscall argument register.
        self:emit(ir.IrOp.LOAD_IMM, {
            ir.new_register(REG_SYS_ARG),
            ir.new_immediate(1),
        })
        -- Call exit(1).
        self:emit(ir.IrOp.SYSCALL, { ir.new_immediate(SYSCALL_EXIT) })
    end
end

-- ============================================================================
-- Bounds Checking
-- ============================================================================
--
-- In debug builds, the compiler inserts range checks before every pointer
-- move. If the pointer goes out of bounds, the program jumps to __trap_oob.
--
-- RIGHT (>) check — is v1 >= tape_size?
--   CMP_GT  v3, v1, v5        ← v3 = (v1 > v5) ? 1 : 0
--   BRANCH_NZ v3, __trap_oob  ← if v3 != 0, trap
--
-- LEFT (<) check — is v1 < 0?
--   CMP_LT  v3, v1, v6        ← v3 = (v1 < 0) ? 1 : 0
--   BRANCH_NZ v3, __trap_oob  ← if v3 != 0, trap

function Compiler:emit_bounds_check_right()
    local ids = {}
    ids[#ids + 1] = self:emit(ir.IrOp.CMP_GT, {
        ir.new_register(REG_TEMP2),
        ir.new_register(REG_TAPE_PTR),
        ir.new_register(REG_MAX_PTR),
    })
    ids[#ids + 1] = self:emit(ir.IrOp.BRANCH_NZ, {
        ir.new_register(REG_TEMP2),
        ir.new_label("__trap_oob"),
    })
    return ids
end

function Compiler:emit_bounds_check_left()
    local ids = {}
    ids[#ids + 1] = self:emit(ir.IrOp.CMP_LT, {
        ir.new_register(REG_TAPE_PTR),
        ir.new_register(REG_TAPE_PTR),
        ir.new_register(REG_ZERO),
    })
    ids[#ids + 1] = self:emit(ir.IrOp.BRANCH_NZ, {
        ir.new_register(REG_TAPE_PTR),
        ir.new_label("__trap_oob"),
    })
    return ids
end

-- ============================================================================
-- Cell Mutation Helper
-- ============================================================================
--
-- Brainfuck "+" and "-" both read-modify-write the current cell.
-- The sequence is:
--   LOAD_BYTE  v2, v0, v1        ← load current cell
--   ADD_IMM    v2, v2, delta      ← increment (+1) or decrement (-1)
--   AND_IMM    v2, v2, 255        ← mask to byte range [0, 255] (if enabled)
--   STORE_BYTE v2, v0, v1        ← store back
--
-- The AND_IMM is the "mask_byte_arithmetic" guard. In hardware, byte-width
-- stores automatically discard the high bits. In simulation or on word-width
-- ISAs, we need the mask to ensure cells stay in 0-255.

function Compiler:emit_cell_mutation(delta)
    local ids = {}

    -- Load current cell value into v2.
    ids[#ids + 1] = self:emit(ir.IrOp.LOAD_BYTE, {
        ir.new_register(REG_TEMP),
        ir.new_register(REG_TAPE_BASE),
        ir.new_register(REG_TAPE_PTR),
    })

    -- Add delta to the cell value.
    ids[#ids + 1] = self:emit(ir.IrOp.ADD_IMM, {
        ir.new_register(REG_TEMP),
        ir.new_register(REG_TEMP),
        ir.new_immediate(delta),
    })

    -- Mask to byte range (0-255) if masking is enabled.
    if self.config.mask_byte_arithmetic then
        ids[#ids + 1] = self:emit(ir.IrOp.AND_IMM, {
            ir.new_register(REG_TEMP),
            ir.new_register(REG_TEMP),
            ir.new_immediate(255),
        })
    end

    -- Store back to the current cell.
    ids[#ids + 1] = self:emit(ir.IrOp.STORE_BYTE, {
        ir.new_register(REG_TEMP),
        ir.new_register(REG_TAPE_BASE),
        ir.new_register(REG_TAPE_PTR),
    })

    return ids
end

-- ============================================================================
-- Token Extraction
-- ============================================================================
--
-- The Brainfuck AST uses the grammar-driven parser. A "command" node is
-- an ASTNode with rule_name "command" that wraps exactly one token.
--
-- Token fields (from brainfuck.lexer.tokenize):
--   tok.value  — the character (">", "<", "+", "-", ".", ",", "[", "]")
--   tok.line   — 1-based line number
--   tok.col    — 1-based column number (NOTE: "col", not "column")
--
-- We dig through the children to find the first token (non-ASTNode table).

--- Find the first leaf token in an ASTNode tree.
-- Returns the token table, or nil if none found.
local function extract_token(node)
    -- A leaf ASTNode has exactly one child that is not an ASTNode.
    -- We check for presence of 'value' field to identify tokens.
    for _, child in ipairs(node.children) do
        if type(child) == "table" then
            if child.value ~= nil and child.rule_name == nil then
                -- This is a token table (has value but no rule_name)
                return child
            elseif child.rule_name ~= nil then
                -- Recurse into ASTNode children
                local tok = extract_token(child)
                if tok then return tok end
            end
        end
    end
    return nil
end

-- ============================================================================
-- AST Walking
-- ============================================================================
--
-- The Brainfuck AST structure (from brainfuck.grammar):
--
--   program     → { instruction }
--   instruction → loop | command
--   loop        → LOOP_START { instruction } LOOP_END
--   command     → RIGHT | LEFT | INC | DEC | OUTPUT | INPUT
--
-- The compiler walks this tree recursively, emitting IR for each leaf node.

function Compiler:compile_program(node)
    for _, child in ipairs(node.children) do
        if type(child) == "table" and child.rule_name ~= nil then
            local ok, err = self:compile_node(child)
            if not ok then
                return false, err
            end
        end
        -- Skip token children at program level (EOF, etc.)
    end
    return true, nil
end

function Compiler:compile_node(node)
    local rn = node.rule_name

    if rn == "instruction" then
        -- "instruction" is a wrapper node; descend into children.
        for _, child in ipairs(node.children) do
            if type(child) == "table" and child.rule_name ~= nil then
                local ok, err = self:compile_node(child)
                if not ok then
                    return false, err
                end
            end
        end
        return true, nil

    elseif rn == "command" then
        return self:compile_command(node)

    elseif rn == "loop" then
        return self:compile_loop(node)

    else
        return false, string.format(
            "unexpected AST node type: %q", rn)
    end
end

-- ============================================================================
-- Command Compilation
-- ============================================================================
--
-- Each Brainfuck command maps to a specific sequence of IR instructions.
-- The mapping is documented in the spec (BF03) and in the table below.
--
-- ┌──────────────────┬────────────────────────────────────────────────────────┐
-- │ Command          │ IR Output                                              │
-- ├──────────────────┼────────────────────────────────────────────────────────┤
-- │ > (RIGHT)        │ ADD_IMM v1, v1, 1                                      │
-- │ < (LEFT)         │ ADD_IMM v1, v1, -1                                     │
-- │ + (INC)          │ LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, 1;              │
-- │                  │ AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1            │
-- │ - (DEC)          │ LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, -1;             │
-- │                  │ AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1            │
-- │ . (OUTPUT)       │ LOAD_BYTE v2, v0, v1; ADD_IMM v4, v2, 0; SYSCALL 1    │
-- │ , (INPUT)        │ SYSCALL 2; STORE_BYTE v4, v0, v1                      │
-- └──────────────────┴────────────────────────────────────────────────────────┘

function Compiler:compile_command(node)
    local tok = extract_token(node)
    if not tok then
        return false, "command node has no token"
    end

    -- Assign a unique AST node ID and record the source position.
    local ast_node_id = self:next_node_id()
    self.source_map.source_to_ast:add(
        sm.new_source_position(self.filename, tok.line, tok.col, 1),
        ast_node_id
    )

    local ir_ids = {}
    local val = tok.value

    if val == ">" then
        -- RIGHT: move tape pointer right by 1.
        -- Optionally check that pointer is not already at the max.
        if self.config.insert_bounds_checks then
            local check_ids = self:emit_bounds_check_right()
            for _, id in ipairs(check_ids) do
                ir_ids[#ir_ids + 1] = id
            end
        end
        ir_ids[#ir_ids + 1] = self:emit(ir.IrOp.ADD_IMM, {
            ir.new_register(REG_TAPE_PTR),
            ir.new_register(REG_TAPE_PTR),
            ir.new_immediate(1),
        })

    elseif val == "<" then
        -- LEFT: move tape pointer left by 1.
        -- Optionally check that pointer is not already at 0.
        if self.config.insert_bounds_checks then
            local check_ids = self:emit_bounds_check_left()
            for _, id in ipairs(check_ids) do
                ir_ids[#ir_ids + 1] = id
            end
        end
        ir_ids[#ir_ids + 1] = self:emit(ir.IrOp.ADD_IMM, {
            ir.new_register(REG_TAPE_PTR),
            ir.new_register(REG_TAPE_PTR),
            ir.new_immediate(-1),
        })

    elseif val == "+" then
        -- INC: increment current cell by 1.
        local ids = self:emit_cell_mutation(1)
        for _, id in ipairs(ids) do
            ir_ids[#ir_ids + 1] = id
        end

    elseif val == "-" then
        -- DEC: decrement current cell by 1.
        local ids = self:emit_cell_mutation(-1)
        for _, id in ipairs(ids) do
            ir_ids[#ir_ids + 1] = id
        end

    elseif val == "." then
        -- OUTPUT: write current cell byte to stdout.
        -- Step 1: load current cell into v2.
        ir_ids[#ir_ids + 1] = self:emit(ir.IrOp.LOAD_BYTE, {
            ir.new_register(REG_TEMP),
            ir.new_register(REG_TAPE_BASE),
            ir.new_register(REG_TAPE_PTR),
        })
        -- Step 2: copy v2 → v4 (syscall arg register).
        -- Use ADD_IMM with 0 so release builds do not depend on the
        -- debug-only zero register v6 being initialized.
        ir_ids[#ir_ids + 1] = self:emit(ir.IrOp.ADD_IMM, {
            ir.new_register(REG_SYS_ARG),
            ir.new_register(REG_TEMP),
            ir.new_immediate(0),
        })
        -- Step 3: syscall 1 = write.
        ir_ids[#ir_ids + 1] = self:emit(ir.IrOp.SYSCALL, {
            ir.new_immediate(SYSCALL_WRITE),
        })

    elseif val == "," then
        -- INPUT: read one byte from stdin into current cell.
        -- Step 1: syscall 2 = read (result goes to v4).
        ir_ids[#ir_ids + 1] = self:emit(ir.IrOp.SYSCALL, {
            ir.new_immediate(SYSCALL_READ),
        })
        -- Step 2: store v4 (syscall result) to current cell.
        ir_ids[#ir_ids + 1] = self:emit(ir.IrOp.STORE_BYTE, {
            ir.new_register(REG_SYS_ARG),
            ir.new_register(REG_TAPE_BASE),
            ir.new_register(REG_TAPE_PTR),
        })

    else
        return false, string.format("unknown command token: %q", val)
    end

    -- Record the AST → IR mapping for this command.
    self.source_map.ast_to_ir:add(ast_node_id, ir_ids)
    return true, nil
end

-- ============================================================================
-- Loop Compilation
-- ============================================================================
--
-- A Brainfuck loop [body] compiles to:
--
--   LABEL      loop_N_start
--   LOAD_BYTE  v2, v0, v1          ← load current cell
--   BRANCH_Z   v2, loop_N_end      ← skip body if cell == 0
--   ...compile body...
--   JUMP       loop_N_start        ← repeat
--   LABEL      loop_N_end
--
-- Each loop gets a unique number N (from loop_count) to make labels unique.
-- Nested loops get distinct N values (loop_0_start, loop_1_start, etc.).

function Compiler:compile_loop(node)
    local loop_num   = self.loop_count
    self.loop_count  = self.loop_count + 1
    local start_label = string.format("loop_%d_start", loop_num)
    local end_label   = string.format("loop_%d_end", loop_num)

    -- Assign a source map entry for the loop construct itself.
    local ast_node_id = self:next_node_id()
    if node.start_line and node.start_line > 0 then
        self.source_map.source_to_ast:add(
            sm.new_source_position(
                self.filename,
                node.start_line,
                node.start_column,
                1
            ),
            ast_node_id
        )
    end

    local ir_ids = {}

    -- Emit loop start label.
    self:emit_label(start_label)

    -- Load current cell and branch to end if zero.
    ir_ids[#ir_ids + 1] = self:emit(ir.IrOp.LOAD_BYTE, {
        ir.new_register(REG_TEMP),
        ir.new_register(REG_TAPE_BASE),
        ir.new_register(REG_TAPE_PTR),
    })
    ir_ids[#ir_ids + 1] = self:emit(ir.IrOp.BRANCH_Z, {
        ir.new_register(REG_TEMP),
        ir.new_label(end_label),
    })

    -- Compile loop body (skip LOOP_START and LOOP_END bracket tokens).
    for _, child in ipairs(node.children) do
        if type(child) == "table" and child.rule_name ~= nil then
            local ok, err = self:compile_node(child)
            if not ok then
                return false, err
            end
        end
        -- Token children (brackets) are skipped automatically.
    end

    -- Jump back to loop start.
    ir_ids[#ir_ids + 1] = self:emit(ir.IrOp.JUMP, {
        ir.new_label(start_label),
    })

    -- Emit loop end label.
    self:emit_label(end_label)

    -- Record AST → IR mapping for the loop construct.
    self.source_map.ast_to_ir:add(ast_node_id, ir_ids)

    return true, nil
end

-- ============================================================================
-- Public compile() Entry Point
-- ============================================================================
--
-- compile() takes a Brainfuck AST (from brainfuck.parser.parse), a filename
-- string, and a BuildConfig. It returns a CompileResult on success, or
-- (nil, error_string) on failure.
--
-- The filename is used in source map entries to identify which file the
-- source positions refer to. Use a short basename like "hello.bf" or
-- "test.bf" for readability.

--- Compile a Brainfuck AST to IR.
--
-- @param ast      table   The root ASTNode from brainfuck.parser.parse().
--                         Must have rule_name == "program".
-- @param filename string  Source file name for source map entries.
-- @param config   table   A BuildConfig table (from debug_config or release_config).
-- @return table|nil       A CompileResult { program, source_map } on success.
-- @return string|nil      An error message on failure, nil on success.
function M.compile(ast, filename, config)
    -- Validate inputs.
    if ast.rule_name ~= "program" then
        return nil, string.format(
            "expected 'program' AST node, got %q", ast.rule_name)
    end
    if config.tape_size <= 0 then
        return nil, string.format(
            "invalid tape_size %d: must be positive", config.tape_size)
    end

    -- Create the internal compiler.
    local c = new_compiler(config, filename)

    -- Add tape data declaration.
    ir.add_data(c.program, ir.new_data_decl("tape", config.tape_size, 0))

    -- Emit prologue (sets up v0, v1, and debug registers).
    c:emit_prologue()

    -- Compile the program body.
    local ok, err = c:compile_program(ast)
    if not ok then
        return nil, err
    end

    -- Emit epilogue (HALT and optional trap handler).
    c:emit_epilogue()

    return new_compile_result(c.program, c.source_map), nil
end

return M
