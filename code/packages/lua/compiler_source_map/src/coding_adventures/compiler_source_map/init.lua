-- ============================================================================
-- compiler_source_map — Source map chain for the AOT compiler pipeline
-- ============================================================================
--
-- This module provides the source-mapping sidecar that flows through every
-- stage of the AOT compiler pipeline. It connects source text positions to
-- machine code byte offsets through a chain of intermediate mappings.
--
-- ## Why a "chain" instead of a flat table?
--
-- A flat table (machine-code offset → source position) works for the final
-- consumer — a debugger, profiler, or error reporter. But it doesn't help
-- when you're debugging the *compiler itself*:
--
--   - "Why did the optimizer delete instruction #42?"
--     → Look at the IrToIr segment for that pass.
--
--   - "Which AST node produced this IR instruction?"
--     → Look at AstToIr.
--
--   - "The machine code for this instruction seems wrong — what IR produced it?"
--     → Look at IrToMachineCode in reverse.
--
-- The chain makes the compiler pipeline **transparent and debuggable at every
-- stage**. The flat composite mapping is just the composition of all segments.
--
-- ## Segment Overview
--
--   Segment 1: SourceToAst       source text position  → AST node ID
--   Segment 2: AstToIr           AST node ID           → IR instruction IDs
--   Segment 3: IrToIr            IR instruction ID     → optimized IR instruction IDs
--                                (one segment per optimizer pass)
--   Segment 4: IrToMachineCode   IR instruction ID     → machine code byte offset + length
--
--   Composite forward:  source position → machine code offset
--   Composite reverse:  machine code offset → source position
--
-- ## Usage
--
--   local sm = require("coding_adventures.compiler_source_map")
--
--   local chain = sm.new_source_map_chain()
--
--   -- Frontend fills SourceToAst and AstToIr
--   chain.source_to_ast:add(sm.new_source_position("main.bf", 1, 3, 1), 42)
--   chain.ast_to_ir:add(42, {7, 8, 9, 10})
--
--   -- Backend fills IrToMachineCode
--   chain.ir_to_machine_code:add(7, 0x14, 4)
--
--   -- Composite queries
--   local mc = chain:source_to_mc(sm.new_source_position("main.bf", 1, 3, 1))
--   local src = chain:mc_to_source(0x14)
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- SourcePosition — A Span of Characters in a Source File
-- ============================================================================
--
-- Think of SourcePosition as a "highlighter pen" marking a region of source
-- code. The (line, column) pair marks the start; length tells you how many
-- characters are highlighted.
--
-- For Brainfuck, every command is exactly one character (length = 1).
-- For BASIC, a keyword like "PRINT" would have length = 5.
--
-- Fields:
--   file:   source file path (e.g., "hello.bf")
--   line:   1-based line number
--   column: 1-based column number
--   length: character span in source

--- Create a new SourcePosition.
--
-- @param file   string  Source file path (e.g., "hello.bf").
-- @param line   number  1-based line number.
-- @param column number  1-based column number.
-- @param length number  Character span in source.
-- @return table         { file, line, column, length }
function M.new_source_position(file, line, column, length)
    return {
        file   = file,
        line   = line,
        column = column,
        length = length,
    }
end

--- Convert a SourcePosition to a human-readable string.
-- Example: "hello.bf:1:3 (len=1)"
--
-- @param sp table  A SourcePosition table.
-- @return string   Human-readable representation.
function M.source_position_tostring(sp)
    return string.format("%s:%d:%d (len=%d)",
        sp.file, sp.line, sp.column, sp.length)
end

-- ============================================================================
-- SourceToAst — Segment 1: Source Text Positions → AST Node IDs
-- ============================================================================
--
-- This segment is produced by the parser or language-specific frontend.
-- It maps every meaningful source position to the AST node that represents it.
--
-- Example:
--   The "+" character at line 1, column 3 of "hello.bf" maps to AST node #42
--   (which is a command(INC) node in the parse tree).
--
-- Internal layout:
--   entries: array of { pos = SourcePosition, ast_node_id = number }

--- Create a new SourceToAst segment.
--
-- @return table  A SourceToAst object with :add() and :lookup_by_node_id().
function M.new_source_to_ast()
    local obj = {
        entries = {},
    }

    --- Record a mapping from a source position to an AST node ID.
    --
    -- @param pos         table   A SourcePosition table.
    -- @param ast_node_id number  The AST node ID at this position.
    function obj:add(pos, ast_node_id)
        self.entries[#self.entries + 1] = {
            pos        = pos,
            ast_node_id = ast_node_id,
        }
    end

    --- Return the source position for the given AST node ID, or nil if not found.
    -- Used for reverse lookups (AST node → source position).
    --
    -- @param ast_node_id number  The AST node ID to look up.
    -- @return table|nil          A SourcePosition, or nil if not found.
    function obj:lookup_by_node_id(ast_node_id)
        for _, entry in ipairs(self.entries) do
            if entry.ast_node_id == ast_node_id then
                return entry.pos
            end
        end
        return nil
    end

    return obj
end

-- ============================================================================
-- AstToIr — Segment 2: AST Node IDs → IR Instruction IDs
-- ============================================================================
--
-- A single AST node often produces multiple IR instructions. For example,
-- a Brainfuck "+" command produces four instructions:
--   LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE
--
-- So the mapping is one-to-many:
--   ast_node_42 → [ir_7, ir_8, ir_9, ir_10]
--
-- Internal layout:
--   entries: array of { ast_node_id = number, ir_ids = {number, ...} }

--- Create a new AstToIr segment.
--
-- @return table  An AstToIr object with :add(), :lookup_by_ast_node_id(),
--                and :lookup_by_ir_id().
function M.new_ast_to_ir()
    local obj = {
        entries = {},
    }

    --- Record that the given AST node produced the given IR instruction IDs.
    --
    -- @param ast_node_id number        The AST node ID.
    -- @param ir_ids      table         Array of IR instruction IDs.
    function obj:add(ast_node_id, ir_ids)
        self.entries[#self.entries + 1] = {
            ast_node_id = ast_node_id,
            ir_ids      = ir_ids,
        }
    end

    --- Return the IR instruction IDs for the given AST node, or nil if not found.
    --
    -- @param ast_node_id number  The AST node ID to look up.
    -- @return table|nil          Array of IR IDs, or nil if not found.
    function obj:lookup_by_ast_node_id(ast_node_id)
        for _, entry in ipairs(self.entries) do
            if entry.ast_node_id == ast_node_id then
                return entry.ir_ids
            end
        end
        return nil
    end

    --- Return the AST node ID that produced the given IR instruction, or -1 if not found.
    -- Used for reverse lookups (IR instruction → AST node).
    -- When multiple AST nodes produced the same IR ID (rare), returns the first match.
    --
    -- @param ir_id number  The IR instruction ID to look up.
    -- @return number       The AST node ID, or -1 if not found.
    function obj:lookup_by_ir_id(ir_id)
        for _, entry in ipairs(self.entries) do
            for _, id in ipairs(entry.ir_ids) do
                if id == ir_id then
                    return entry.ast_node_id
                end
            end
        end
        return -1
    end

    return obj
end

-- ============================================================================
-- IrToIr — Segment 3: IR Instruction IDs → Optimized IR Instruction IDs
-- ============================================================================
--
-- One segment is produced per optimizer pass. The pass_name field identifies
-- which pass produced this mapping (e.g., "identity", "contraction",
-- "clear_loop", "dead_store").
--
-- Three cases for each original ID:
--   1. Preserved:  original_id → [same_id]        (instruction unchanged)
--   2. Replaced:   original_id → [new_id_1, ...]  (instruction split/transformed)
--   3. Deleted:    original_id is in deleted set   (instruction optimized away)
--
-- Example: A contraction pass folds three ADD_IMM 1 instructions
-- (IDs 7, 8, 9) into one ADD_IMM 3 (ID 100):
--   7 → [100], 8 → [100], 9 → [100]

--- Create a new IrToIr segment for the named optimizer pass.
--
-- @param pass_name string  The optimizer pass name (e.g., "contraction").
-- @return table  An IrToIr object with :add_mapping(), :add_deletion(),
--               :lookup_by_original_id(), and :lookup_by_new_id().
function M.new_ir_to_ir(pass_name)
    local obj = {
        entries   = {},
        deleted   = {},   -- set of deleted original IDs (id → true)
        pass_name = pass_name,
    }

    --- Record that the original instruction was replaced by the new ones.
    -- The new_ids array may have one entry (simple replacement), multiple
    -- entries (expansion), or zero entries (deletion — use add_deletion instead).
    --
    -- @param original_id number  The original IR instruction ID.
    -- @param new_ids     table   Array of new IR instruction IDs.
    function obj:add_mapping(original_id, new_ids)
        self.entries[#self.entries + 1] = {
            original_id = original_id,
            new_ids     = new_ids,
        }
    end

    --- Record that the original instruction was deleted by this pass.
    -- Deleted instructions do not appear in the optimized IR.
    --
    -- @param original_id number  The IR instruction ID that was deleted.
    function obj:add_deletion(original_id)
        self.deleted[original_id] = true
        self.entries[#self.entries + 1] = {
            original_id = original_id,
            new_ids     = nil,
        }
    end

    --- Return the new IDs for the given original ID, or nil if deleted or not found.
    --
    -- @param original_id number  The original IR instruction ID.
    -- @return table|nil          Array of new IR IDs, or nil.
    function obj:lookup_by_original_id(original_id)
        if self.deleted[original_id] then
            return nil
        end
        for _, entry in ipairs(self.entries) do
            if entry.original_id == original_id then
                return entry.new_ids
            end
        end
        return nil
    end

    --- Return the original ID that produced the given new ID, or -1 if not found.
    -- When multiple originals map to the same new ID (e.g., contraction),
    -- this returns the first one found.
    --
    -- @param new_id number  The new IR instruction ID.
    -- @return number        The original ID, or -1 if not found.
    function obj:lookup_by_new_id(new_id)
        for _, entry in ipairs(self.entries) do
            if entry.new_ids then
                for _, id in ipairs(entry.new_ids) do
                    if id == new_id then
                        return entry.original_id
                    end
                end
            end
        end
        return -1
    end

    return obj
end

-- ============================================================================
-- IrToMachineCode — Segment 4: IR Instruction IDs → Machine Code Offsets
-- ============================================================================
--
-- Each entry is a triple: (ir_id, mc_offset, mc_length).
-- For example, a LOAD_BYTE IR instruction might produce 8 bytes of RISC-V
-- machine code starting at offset 0x14 in the .text section.
--
-- Lookups:
--   By IR ID:     which machine code bytes does this instruction produce?
--   By MC offset: which IR instruction produced the code at this offset?
--
-- The "by MC offset" lookup checks the range [mc_offset, mc_offset + mc_length),
-- so a lookup of 0x16 finds the entry that covers [0x14, 0x1C).

--- Create a new IrToMachineCode segment.
--
-- @return table  An IrToMachineCode object with :add(), :lookup_by_ir_id(),
--               and :lookup_by_mc_offset().
function M.new_ir_to_machine_code()
    local obj = {
        entries = {},
    }

    --- Record that the given IR instruction produced machine code at the given offset.
    --
    -- @param ir_id     number  IR instruction ID.
    -- @param mc_offset number  Byte offset in the .text section.
    -- @param mc_length number  Number of bytes of machine code.
    function obj:add(ir_id, mc_offset, mc_length)
        self.entries[#self.entries + 1] = {
            ir_id     = ir_id,
            mc_offset = mc_offset,
            mc_length = mc_length,
        }
    end

    --- Return the machine code offset and length for the given IR instruction ID.
    -- Returns (-1, 0) if not found.
    --
    -- @param ir_id  number  IR instruction ID.
    -- @return number        mc_offset (-1 if not found)
    -- @return number        mc_length (0 if not found)
    function obj:lookup_by_ir_id(ir_id)
        for _, entry in ipairs(self.entries) do
            if entry.ir_id == ir_id then
                return entry.mc_offset, entry.mc_length
            end
        end
        return -1, 0
    end

    --- Return the IR instruction ID whose machine code contains the given offset.
    -- An instruction "contains" an offset if:
    --   entry.mc_offset <= offset < entry.mc_offset + entry.mc_length
    -- Returns -1 if not found.
    --
    -- @param offset number  The machine code byte offset to look up.
    -- @return number        The IR instruction ID, or -1 if not found.
    function obj:lookup_by_mc_offset(offset)
        for _, entry in ipairs(self.entries) do
            if offset >= entry.mc_offset and
               offset < entry.mc_offset + entry.mc_length then
                return entry.ir_id
            end
        end
        return -1
    end

    return obj
end

-- ============================================================================
-- SourceMapChain — The Full Pipeline Sidecar
-- ============================================================================
--
-- The SourceMapChain is the central data structure that flows through every
-- stage of the compiler pipeline. Each stage reads the existing segments and
-- appends its own:
--
--   1. Frontend (brainfuck-ir-compiler) → fills source_to_ast + ast_to_ir
--   2. Optimizer (compiler-ir-optimizer) → appends ir_to_ir segments
--   3. Backend (codegen-riscv) → fills ir_to_machine_code
--
-- ## Composite Queries
--
-- source_to_mc(pos):  source position → machine code entries
--   1. source_to_ast: source position → AST node ID
--   2. ast_to_ir: AST node ID → IR instruction IDs
--   3. ir_to_ir (each pass): follow IR IDs through each optimizer pass
--   4. ir_to_machine_code: final IR IDs → machine code offsets
--
-- mc_to_source(offset):  machine code offset → source position (reverse)
--   1. ir_to_machine_code: MC offset → IR instruction ID
--   2. ir_to_ir (each pass, in reverse): follow IR ID back through passes
--   3. ast_to_ir: IR ID → AST node ID
--   4. source_to_ast: AST node ID → source position

--- Create a new, empty SourceMapChain.
-- The ir_to_machine_code field starts as nil — it is filled by the backend.
-- The ir_to_ir list starts empty — optimizer passes append to it.
--
-- @return table  A SourceMapChain object with :add_optimizer_pass(),
--               :source_to_mc(), and :mc_to_source().
function M.new_source_map_chain()
    local obj = {
        source_to_ast      = M.new_source_to_ast(),
        ast_to_ir          = M.new_ast_to_ir(),
        ir_to_ir           = {},    -- one entry per optimizer pass
        ir_to_machine_code = nil,   -- filled by backend (nil until then)
    }

    --- Append an IrToIr segment from an optimizer pass.
    --
    -- @param segment table  An IrToIr segment (from new_ir_to_ir).
    function obj:add_optimizer_pass(segment)
        self.ir_to_ir[#self.ir_to_ir + 1] = segment
    end

    --- Compose all segments to look up the machine code offset(s) for a source position.
    -- Returns nil if the chain is incomplete or no mapping exists.
    --
    -- @param pos table  A SourcePosition table.
    -- @return table|nil Array of { ir_id, mc_offset, mc_length } entries, or nil.
    function obj:source_to_mc(pos)
        if not self.ir_to_machine_code then
            return nil
        end

        -- Step 1: source position → AST node ID.
        local ast_node_id = -1
        for _, entry in ipairs(self.source_to_ast.entries) do
            if entry.pos.file   == pos.file and
               entry.pos.line   == pos.line and
               entry.pos.column == pos.column then
                ast_node_id = entry.ast_node_id
                break
            end
        end
        if ast_node_id == -1 then
            return nil
        end

        -- Step 2: AST node ID → IR instruction IDs.
        local ir_ids = self.ast_to_ir:lookup_by_ast_node_id(ast_node_id)
        if not ir_ids then
            return nil
        end

        -- Step 3: follow through optimizer passes (in forward order).
        local current_ids = ir_ids
        for _, pass in ipairs(self.ir_to_ir) do
            local next_ids = {}
            for _, id in ipairs(current_ids) do
                if not pass.deleted[id] then
                    local new_ids = pass:lookup_by_original_id(id)
                    if new_ids then
                        for _, nid in ipairs(new_ids) do
                            next_ids[#next_ids + 1] = nid
                        end
                    end
                end
            end
            current_ids = next_ids
        end

        if #current_ids == 0 then
            return nil
        end

        -- Step 4: final IR IDs → machine code entries.
        local results = {}
        for _, id in ipairs(current_ids) do
            local offset, length = self.ir_to_machine_code:lookup_by_ir_id(id)
            if offset >= 0 then
                results[#results + 1] = {
                    ir_id     = id,
                    mc_offset = offset,
                    mc_length = length,
                }
            end
        end
        return results
    end

    --- Compose all segments in reverse to look up the source position for a MC offset.
    -- Returns nil if the chain is incomplete or no mapping exists.
    --
    -- @param mc_offset number  The machine code byte offset.
    -- @return table|nil        A SourcePosition table, or nil if not found.
    function obj:mc_to_source(mc_offset)
        if not self.ir_to_machine_code then
            return nil
        end

        -- Step 1: MC offset → IR instruction ID.
        local ir_id = self.ir_to_machine_code:lookup_by_mc_offset(mc_offset)
        if ir_id == -1 then
            return nil
        end

        -- Step 2: follow back through optimizer passes (in reverse order).
        local current_id = ir_id
        for i = #self.ir_to_ir, 1, -1 do
            local pass = self.ir_to_ir[i]
            local original_id = pass:lookup_by_new_id(current_id)
            if original_id == -1 then
                return nil  -- can't trace back through this pass
            end
            current_id = original_id
        end

        -- Step 3: IR instruction ID → AST node ID.
        local ast_node_id = self.ast_to_ir:lookup_by_ir_id(current_id)
        if ast_node_id == -1 then
            return nil
        end

        -- Step 4: AST node ID → source position.
        return self.source_to_ast:lookup_by_node_id(ast_node_id)
    end

    return obj
end

return M
