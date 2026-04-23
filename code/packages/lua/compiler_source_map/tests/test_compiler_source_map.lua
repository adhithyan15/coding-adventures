-- ============================================================================
-- Tests for compiler_source_map — Source map chain
-- ============================================================================
--
-- ## Testing Strategy
--
-- 1. SourcePosition — constructor and tostring.
-- 2. SourceToAst — add, lookup_by_node_id.
-- 3. AstToIr — add, lookup_by_ast_node_id, lookup_by_ir_id.
-- 4. IrToIr — add_mapping, add_deletion, lookup_by_original_id, lookup_by_new_id.
-- 5. IrToMachineCode — add, lookup_by_ir_id, lookup_by_mc_offset.
-- 6. SourceMapChain — new_source_map_chain, add_optimizer_pass.
-- 7. Composite source_to_mc — end-to-end forward query.
-- 8. Composite mc_to_source — end-to-end reverse query.
-- 9. Edge cases — nil ir_to_machine_code, missing IDs, deleted instructions.
-- ============================================================================

-- IMPORTANT: Set package.path before any require calls.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local sm = require("coding_adventures.compiler_source_map")

describe("compiler_source_map", function()

    -- -----------------------------------------------------------------------
    -- SourcePosition
    -- -----------------------------------------------------------------------
    describe("new_source_position", function()

        it("stores file, line, column, and length", function()
            local sp = sm.new_source_position("hello.bf", 1, 3, 1)
            assert.equals("hello.bf", sp.file)
            assert.equals(1, sp.line)
            assert.equals(3, sp.column)
            assert.equals(1, sp.length)
        end)

        it("source_position_tostring formats correctly", function()
            local sp = sm.new_source_position("hello.bf", 1, 3, 1)
            local s = sm.source_position_tostring(sp)
            assert.equals("hello.bf:1:3 (len=1)", s)
        end)

        it("handles multi-character spans", function()
            local sp = sm.new_source_position("test.basic", 5, 10, 5)
            assert.equals(5, sp.length)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- SourceToAst
    -- -----------------------------------------------------------------------
    describe("new_source_to_ast", function()

        it("starts with empty entries", function()
            local s2a = sm.new_source_to_ast()
            assert.equals(0, #s2a.entries)
        end)

        it("add stores an entry", function()
            local s2a = sm.new_source_to_ast()
            s2a:add(sm.new_source_position("test.bf", 1, 1, 1), 42)
            assert.equals(1, #s2a.entries)
        end)

        it("lookup_by_node_id finds the entry", function()
            local s2a = sm.new_source_to_ast()
            local pos = sm.new_source_position("test.bf", 1, 3, 1)
            s2a:add(pos, 42)
            local found = s2a:lookup_by_node_id(42)
            assert.is_not_nil(found)
            assert.equals(3, found.column)
        end)

        it("lookup_by_node_id returns nil for unknown ID", function()
            local s2a = sm.new_source_to_ast()
            assert.is_nil(s2a:lookup_by_node_id(999))
        end)

        it("stores multiple entries", function()
            local s2a = sm.new_source_to_ast()
            s2a:add(sm.new_source_position("test.bf", 1, 1, 1), 0)
            s2a:add(sm.new_source_position("test.bf", 1, 2, 1), 1)
            assert.equals(2, #s2a.entries)
        end)

        it("lookup finds correct entry among multiple", function()
            local s2a = sm.new_source_to_ast()
            s2a:add(sm.new_source_position("test.bf", 1, 1, 1), 0)
            s2a:add(sm.new_source_position("test.bf", 1, 2, 1), 1)
            local pos = s2a:lookup_by_node_id(1)
            assert.is_not_nil(pos)
            assert.equals(2, pos.column)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- AstToIr
    -- -----------------------------------------------------------------------
    describe("new_ast_to_ir", function()

        it("starts with empty entries", function()
            local a2i = sm.new_ast_to_ir()
            assert.equals(0, #a2i.entries)
        end)

        it("add stores an entry", function()
            local a2i = sm.new_ast_to_ir()
            a2i:add(42, {7, 8, 9, 10})
            assert.equals(1, #a2i.entries)
        end)

        it("lookup_by_ast_node_id returns ir_ids", function()
            local a2i = sm.new_ast_to_ir()
            a2i:add(42, {7, 8, 9, 10})
            local ids = a2i:lookup_by_ast_node_id(42)
            assert.is_not_nil(ids)
            assert.equals(4, #ids)
            assert.equals(7, ids[1])
        end)

        it("lookup_by_ast_node_id returns nil for unknown id", function()
            local a2i = sm.new_ast_to_ir()
            assert.is_nil(a2i:lookup_by_ast_node_id(999))
        end)

        it("lookup_by_ir_id finds the ast node for a given ir id", function()
            local a2i = sm.new_ast_to_ir()
            a2i:add(42, {7, 8, 9, 10})
            assert.equals(42, a2i:lookup_by_ir_id(9))
        end)

        it("lookup_by_ir_id returns -1 for unknown ir id", function()
            local a2i = sm.new_ast_to_ir()
            assert.equals(-1, a2i:lookup_by_ir_id(999))
        end)

        it("lookup_by_ir_id checks all entries", function()
            local a2i = sm.new_ast_to_ir()
            a2i:add(0, {1, 2})
            a2i:add(1, {3, 4})
            assert.equals(1, a2i:lookup_by_ir_id(3))
        end)

    end)

    -- -----------------------------------------------------------------------
    -- IrToIr
    -- -----------------------------------------------------------------------
    describe("new_ir_to_ir", function()

        it("stores pass_name", function()
            local m = sm.new_ir_to_ir("contraction")
            assert.equals("contraction", m.pass_name)
        end)

        it("add_mapping stores a mapping", function()
            local m = sm.new_ir_to_ir("identity")
            m:add_mapping(7, {100})
            local ids = m:lookup_by_original_id(7)
            assert.is_not_nil(ids)
            assert.equals(100, ids[1])
        end)

        it("add_deletion marks id as deleted", function()
            local m = sm.new_ir_to_ir("dead_store")
            m:add_deletion(5)
            assert.is_true(m.deleted[5])
        end)

        it("lookup_by_original_id returns nil for deleted id", function()
            local m = sm.new_ir_to_ir("dead_store")
            m:add_deletion(5)
            assert.is_nil(m:lookup_by_original_id(5))
        end)

        it("lookup_by_original_id returns nil for unknown id", function()
            local m = sm.new_ir_to_ir("identity")
            assert.is_nil(m:lookup_by_original_id(999))
        end)

        it("lookup_by_new_id finds original for a given new id", function()
            local m = sm.new_ir_to_ir("contraction")
            m:add_mapping(7, {100})
            m:add_mapping(8, {100})
            m:add_mapping(9, {100})
            -- First one to add id 100 should be found
            local orig = m:lookup_by_new_id(100)
            assert.equals(7, orig)
        end)

        it("lookup_by_new_id returns -1 for unknown new id", function()
            local m = sm.new_ir_to_ir("identity")
            assert.equals(-1, m:lookup_by_new_id(999))
        end)

        it("many-to-one mapping: multiple originals map to same new id", function()
            local m = sm.new_ir_to_ir("contraction")
            m:add_mapping(10, {200})
            m:add_mapping(11, {200})
            -- Both should be traceable to 200
            assert.is_not_nil(m:lookup_by_original_id(10))
            assert.is_not_nil(m:lookup_by_original_id(11))
        end)

    end)

    -- -----------------------------------------------------------------------
    -- IrToMachineCode
    -- -----------------------------------------------------------------------
    describe("new_ir_to_machine_code", function()

        it("starts with empty entries", function()
            local m2mc = sm.new_ir_to_machine_code()
            assert.equals(0, #m2mc.entries)
        end)

        it("add stores an entry", function()
            local m2mc = sm.new_ir_to_machine_code()
            m2mc:add(7, 0x14, 4)
            assert.equals(1, #m2mc.entries)
        end)

        it("lookup_by_ir_id returns offset and length", function()
            local m2mc = sm.new_ir_to_machine_code()
            m2mc:add(7, 0x14, 4)
            local offset, length = m2mc:lookup_by_ir_id(7)
            assert.equals(0x14, offset)
            assert.equals(4, length)
        end)

        it("lookup_by_ir_id returns -1, 0 for unknown id", function()
            local m2mc = sm.new_ir_to_machine_code()
            local offset, length = m2mc:lookup_by_ir_id(999)
            assert.equals(-1, offset)
            assert.equals(0, length)
        end)

        it("lookup_by_mc_offset finds instruction by offset", function()
            local m2mc = sm.new_ir_to_machine_code()
            m2mc:add(7, 0x14, 8)  -- covers 0x14..0x1B
            assert.equals(7, m2mc:lookup_by_mc_offset(0x14))  -- start
            assert.equals(7, m2mc:lookup_by_mc_offset(0x1A))  -- middle
        end)

        it("lookup_by_mc_offset returns -1 for out-of-range offset", function()
            local m2mc = sm.new_ir_to_machine_code()
            m2mc:add(7, 0x14, 4)  -- covers 0x14..0x17
            assert.equals(-1, m2mc:lookup_by_mc_offset(0x18))  -- just past end
        end)

        it("lookup_by_mc_offset returns -1 for offset before range", function()
            local m2mc = sm.new_ir_to_machine_code()
            m2mc:add(7, 0x14, 4)
            assert.equals(-1, m2mc:lookup_by_mc_offset(0x13))  -- just before start
        end)

        it("the end of range is exclusive", function()
            -- [0x14, 0x18) means 0x17 is included, 0x18 is NOT
            local m2mc = sm.new_ir_to_machine_code()
            m2mc:add(7, 0x14, 4)
            assert.equals(7,  m2mc:lookup_by_mc_offset(0x17))  -- last valid
            assert.equals(-1, m2mc:lookup_by_mc_offset(0x18))  -- first invalid
        end)

    end)

    -- -----------------------------------------------------------------------
    -- SourceMapChain
    -- -----------------------------------------------------------------------
    describe("new_source_map_chain", function()

        it("creates a chain with source_to_ast, ast_to_ir, ir_to_ir fields", function()
            local chain = sm.new_source_map_chain()
            assert.is_not_nil(chain.source_to_ast)
            assert.is_not_nil(chain.ast_to_ir)
            assert.is_table(chain.ir_to_ir)
        end)

        it("ir_to_machine_code starts nil", function()
            local chain = sm.new_source_map_chain()
            assert.is_nil(chain.ir_to_machine_code)
        end)

        it("add_optimizer_pass appends to ir_to_ir", function()
            local chain = sm.new_source_map_chain()
            local pass = sm.new_ir_to_ir("test_pass")
            chain:add_optimizer_pass(pass)
            assert.equals(1, #chain.ir_to_ir)
            assert.equals("test_pass", chain.ir_to_ir[1].pass_name)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Composite: source_to_mc
    -- -----------------------------------------------------------------------
    describe("source_to_mc", function()

        local function make_full_chain()
            local chain = sm.new_source_map_chain()

            -- Frontend: "+" at line 1, col 1 → AST node 0 → IR IDs {2, 3, 4, 5}
            local pos = sm.new_source_position("test.bf", 1, 1, 1)
            chain.source_to_ast:add(pos, 0)
            chain.ast_to_ir:add(0, {2, 3, 4, 5})

            -- Backend: IR instruction 2 → MC offset 0, length 4
            chain.ir_to_machine_code = sm.new_ir_to_machine_code()
            chain.ir_to_machine_code:add(2, 0, 4)
            chain.ir_to_machine_code:add(3, 4, 4)
            chain.ir_to_machine_code:add(4, 8, 4)
            chain.ir_to_machine_code:add(5, 12, 4)

            return chain, pos
        end

        it("returns nil when ir_to_machine_code is nil", function()
            local chain = sm.new_source_map_chain()
            local pos = sm.new_source_position("test.bf", 1, 1, 1)
            assert.is_nil(chain:source_to_mc(pos))
        end)

        it("returns mc entries for a known source position", function()
            local chain, pos = make_full_chain()
            local results = chain:source_to_mc(pos)
            assert.is_not_nil(results)
            assert.is_true(#results > 0)
        end)

        it("returns nil for unknown source position", function()
            local chain = make_full_chain()
            local unknown = sm.new_source_position("other.bf", 99, 99, 1)
            assert.is_nil(chain:source_to_mc(unknown))
        end)

        it("returns correct mc_offset in results", function()
            local chain, pos = make_full_chain()
            local results = chain:source_to_mc(pos)
            -- All 4 IR instructions should map to MC entries
            assert.equals(4, #results)
            -- First IR instruction maps to offset 0
            local first = nil
            for _, r in ipairs(results) do
                if r.ir_id == 2 then first = r end
            end
            assert.is_not_nil(first)
            assert.equals(0, first.mc_offset)
        end)

        it("passes through ir_to_ir optimizer segments", function()
            local chain = sm.new_source_map_chain()
            local pos = sm.new_source_position("test.bf", 1, 1, 1)
            chain.source_to_ast:add(pos, 0)
            chain.ast_to_ir:add(0, {10, 11, 12})

            -- Optimizer pass: replaces {10, 11, 12} with {100, 101, 102}
            local pass = sm.new_ir_to_ir("contraction")
            pass:add_mapping(10, {100})
            pass:add_mapping(11, {101})
            pass:add_mapping(12, {102})
            chain:add_optimizer_pass(pass)

            chain.ir_to_machine_code = sm.new_ir_to_machine_code()
            chain.ir_to_machine_code:add(100, 0, 4)
            chain.ir_to_machine_code:add(101, 4, 4)
            chain.ir_to_machine_code:add(102, 8, 4)

            local results = chain:source_to_mc(pos)
            assert.is_not_nil(results)
            assert.equals(3, #results)
        end)

        it("handles deletion in optimizer pass — deleted instructions not in results", function()
            local chain = sm.new_source_map_chain()
            local pos = sm.new_source_position("test.bf", 1, 1, 1)
            chain.source_to_ast:add(pos, 0)
            chain.ast_to_ir:add(0, {10, 11})

            -- Optimizer deletes 11 (dead code)
            local pass = sm.new_ir_to_ir("dead_store")
            pass:add_mapping(10, {10})
            pass:add_deletion(11)
            chain:add_optimizer_pass(pass)

            chain.ir_to_machine_code = sm.new_ir_to_machine_code()
            chain.ir_to_machine_code:add(10, 0, 4)

            local results = chain:source_to_mc(pos)
            assert.is_not_nil(results)
            assert.equals(1, #results)  -- only ir_id=10 survives
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Composite: mc_to_source
    -- -----------------------------------------------------------------------
    describe("mc_to_source", function()

        local function make_reverse_chain()
            local chain = sm.new_source_map_chain()
            local pos = sm.new_source_position("test.bf", 1, 3, 1)
            chain.source_to_ast:add(pos, 0)
            chain.ast_to_ir:add(0, {5})
            chain.ir_to_machine_code = sm.new_ir_to_machine_code()
            chain.ir_to_machine_code:add(5, 0x20, 4)
            return chain, pos
        end

        it("returns nil when ir_to_machine_code is nil", function()
            local chain = sm.new_source_map_chain()
            assert.is_nil(chain:mc_to_source(0))
        end)

        it("returns nil for unknown mc offset", function()
            local chain = make_reverse_chain()
            assert.is_nil(chain:mc_to_source(0x100))
        end)

        it("finds source position for a known mc offset", function()
            local chain, expected_pos = make_reverse_chain()
            local found = chain:mc_to_source(0x20)
            assert.is_not_nil(found)
            assert.equals(expected_pos.file,   found.file)
            assert.equals(expected_pos.line,   found.line)
            assert.equals(expected_pos.column, found.column)
        end)

        it("works for offset in middle of mc range", function()
            local chain, expected_pos = make_reverse_chain()
            local found = chain:mc_to_source(0x22)  -- offset 0x22 is in [0x20, 0x24)
            assert.is_not_nil(found)
            assert.equals(expected_pos.column, found.column)
        end)

        it("traces back through optimizer passes in reverse", function()
            local chain = sm.new_source_map_chain()
            local pos = sm.new_source_position("test.bf", 1, 1, 1)
            chain.source_to_ast:add(pos, 0)
            chain.ast_to_ir:add(0, {7})

            -- Optimizer renames 7 → 100
            local pass = sm.new_ir_to_ir("rename")
            pass:add_mapping(7, {100})
            chain:add_optimizer_pass(pass)

            chain.ir_to_machine_code = sm.new_ir_to_machine_code()
            chain.ir_to_machine_code:add(100, 0x30, 4)

            local found = chain:mc_to_source(0x30)
            assert.is_not_nil(found)
            assert.equals(1, found.column)
        end)

    end)

end)
