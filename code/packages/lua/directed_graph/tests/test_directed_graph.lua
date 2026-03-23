-- Tests for directed-graph — comprehensive busted test suite
-- ============================================================
--
-- These tests cover:
--   1. DirectedGraph: empty graph, single node, edges, self-loops,
--      predecessors/successors, topological sort, cycle detection,
--      transitive closure, independent groups, affected nodes.
--   2. LabeledGraph: all labeled edge operations, label filtering,
--      algorithm delegation, complex scenarios.
--   3. Visualization: DOT, Mermaid, and ASCII table formats for
--      both unlabeled and labeled graphs.
--
-- Test organization mirrors the Go test files to ensure full parity.

-- Add src/ to the module search path so we can require the package.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local dg = require("coding_adventures.directed_graph")
local DirectedGraph = dg.DirectedGraph
local LabeledGraph = dg.LabeledGraph
local viz = dg.visualization

--- Helper: plain substring check (avoids Lua pattern magic characters).
-- Lua's string.find treats characters like -, +, ., etc. as pattern
-- metacharacters. For checking whether a string contains a literal
-- substring, we use the plain flag (4th argument = true).
local function contains(str, substr)
    return string.find(str, substr, 1, true) ~= nil
end

-- =========================================================================
-- Version
-- =========================================================================

describe("directed-graph", function()
    it("has a version", function()
        assert.are.equal("0.1.0", dg.VERSION)
    end)
end)

-- =========================================================================
-- DirectedGraph — Empty graph tests
-- =========================================================================

describe("DirectedGraph - empty graph", function()
    it("has no nodes", function()
        local g = DirectedGraph.new()
        assert.are.equal(0, #g:nodes())
    end)

    it("has no edges", function()
        local g = DirectedGraph.new()
        assert.are.equal(0, #g:edges())
    end)

    it("returns empty topological sort", function()
        local g = DirectedGraph.new()
        local result, err = g:topological_sort()
        assert.is_nil(err)
        assert.are.equal(0, #result)
    end)

    it("has size 0", function()
        local g = DirectedGraph.new()
        assert.are.equal(0, g:size())
    end)

    it("has no cycle", function()
        local g = DirectedGraph.new()
        assert.is_false(g:has_cycle())
    end)

    it("returns empty independent groups", function()
        local g = DirectedGraph.new()
        local groups, err = g:independent_groups()
        assert.is_nil(err)
        assert.are.equal(0, #groups)
    end)
end)

-- =========================================================================
-- DirectedGraph — Single node tests
-- =========================================================================

describe("DirectedGraph - single node", function()
    it("adds and finds a node", function()
        local g = DirectedGraph.new()
        g:add_node("A")
        assert.is_true(g:has_node("A"))
        assert.are.equal(1, g:size())
    end)

    it("add_node is idempotent", function()
        local g = DirectedGraph.new()
        g:add_node("A")
        g:add_node("A")
        assert.are.equal(1, g:size())
    end)

    it("removes a node", function()
        local g = DirectedGraph.new()
        g:add_node("A")
        local ok, err = g:remove_node("A")
        assert.is_true(ok)
        assert.is_nil(err)
        assert.is_false(g:has_node("A"))
    end)

    it("returns error when removing nonexistent node", function()
        local g = DirectedGraph.new()
        local ok, err = g:remove_node("X")
        assert.is_nil(ok)
        assert.are.equal("node_not_found", err.type)
        assert.are.equal("X", err.node)
    end)

    it("has_node returns false for nonexistent node", function()
        local g = DirectedGraph.new()
        assert.is_false(g:has_node("Z"))
    end)
end)

-- =========================================================================
-- DirectedGraph — Edge tests
-- =========================================================================

describe("DirectedGraph - edges", function()
    it("adds an edge", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        assert.is_true(g:has_edge("A", "B"))
        assert.is_false(g:has_edge("B", "A"))  -- directed!
    end)

    it("add_edge implicitly adds nodes", function()
        local g = DirectedGraph.new()
        g:add_edge("X", "Y")
        assert.is_true(g:has_node("X"))
        assert.is_true(g:has_node("Y"))
    end)

    it("panics on self-loop", function()
        local g = DirectedGraph.new()
        assert.has_error(function()
            g:add_edge("A", "A")
        end)
    end)

    it("removes an edge", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        local ok, err = g:remove_edge("A", "B")
        assert.is_true(ok)
        assert.is_nil(err)
        assert.is_false(g:has_edge("A", "B"))
        -- Nodes should still exist
        assert.is_true(g:has_node("A"))
        assert.is_true(g:has_node("B"))
    end)

    it("returns error when removing nonexistent edge", function()
        local g = DirectedGraph.new()
        g:add_node("A")
        local ok, err = g:remove_edge("A", "B")
        assert.is_nil(ok)
        assert.are.equal("edge_not_found", err.type)
    end)

    it("remove_node cleans up incident edges", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        g:add_edge("B", "C")
        g:remove_node("B")
        assert.is_false(g:has_edge("A", "B"))
        assert.is_false(g:has_edge("B", "C"))
    end)

    it("edges returns sorted pairs", function()
        local g = DirectedGraph.new()
        g:add_edge("C", "D")
        g:add_edge("A", "B")
        local edges = g:edges()
        assert.are.same({"A", "B"}, edges[1])
        assert.are.same({"C", "D"}, edges[2])
    end)

    it("has_edge returns false for nonexistent nodes", function()
        local g = DirectedGraph.new()
        assert.is_false(g:has_edge("X", "Y"))
    end)

    it("add_edge is idempotent", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        g:add_edge("A", "B")
        local edges = g:edges()
        assert.are.equal(1, #edges)
    end)
end)

-- =========================================================================
-- DirectedGraph — Self-loop (allowed) tests
-- =========================================================================

describe("DirectedGraph - self-loops allowed", function()
    it("allows self-loop", function()
        local g = DirectedGraph.new_allow_self_loops()
        g:add_edge("A", "A")
        assert.is_true(g:has_edge("A", "A"))
    end)

    it("has node after self-loop", function()
        local g = DirectedGraph.new_allow_self_loops()
        g:add_edge("A", "A")
        assert.is_true(g:has_node("A"))
        assert.are.equal(1, g:size())
    end)

    it("self-loop appears in successors", function()
        local g = DirectedGraph.new_allow_self_loops()
        g:add_edge("A", "A")
        local succs = g:successors("A")
        assert.are.equal(1, #succs)
        assert.are.equal("A", succs[1])
    end)

    it("self-loop appears in predecessors", function()
        local g = DirectedGraph.new_allow_self_loops()
        g:add_edge("A", "A")
        local preds = g:predecessors("A")
        assert.are.equal(1, #preds)
        assert.are.equal("A", preds[1])
    end)

    it("self-loop is a cycle", function()
        local g = DirectedGraph.new_allow_self_loops()
        g:add_edge("A", "A")
        assert.is_true(g:has_cycle())
    end)

    it("topological sort fails with self-loop", function()
        local g = DirectedGraph.new_allow_self_loops()
        g:add_edge("A", "A")
        local result, err = g:topological_sort()
        assert.is_nil(result)
        assert.are.equal("cycle", err.type)
    end)

    it("allows mixed self-loop and normal edges", function()
        local g = DirectedGraph.new_allow_self_loops()
        g:add_edge("A", "A")
        g:add_edge("A", "B")
        g:add_edge("B", "C")
        assert.is_true(g:has_edge("A", "A"))
        assert.is_true(g:has_edge("A", "B"))
        assert.are.equal(3, g:size())
    end)

    it("removes self-loop edge", function()
        local g = DirectedGraph.new_allow_self_loops()
        g:add_edge("A", "A")
        local ok = g:remove_edge("A", "A")
        assert.is_true(ok)
        assert.is_false(g:has_edge("A", "A"))
        -- Node should still exist
        assert.is_true(g:has_node("A"))
    end)

    it("removes node with self-loop", function()
        local g = DirectedGraph.new_allow_self_loops()
        g:add_edge("A", "A")
        g:add_edge("A", "B")
        g:remove_node("A")
        assert.is_false(g:has_node("A"))
        assert.is_false(g:has_edge("A", "A"))
        assert.is_false(g:has_edge("A", "B"))
    end)

    it("edges output includes self-loop sorted correctly", function()
        local g = DirectedGraph.new_allow_self_loops()
        g:add_edge("A", "A")
        g:add_edge("A", "B")
        local edges = g:edges()
        assert.are.equal(2, #edges)
        assert.are.same({"A", "A"}, edges[1])
        assert.are.same({"A", "B"}, edges[2])
    end)

    it("default graph still rejects self-loops", function()
        local g = DirectedGraph.new()
        assert.has_error(function()
            g:add_edge("X", "X")
        end)
    end)

    it("normal edges work in self-loop graph", function()
        local g = DirectedGraph.new_allow_self_loops()
        g:add_edge("X", "Y")
        assert.is_true(g:has_edge("X", "Y"))
    end)

    it("transitive closure includes self-loop node", function()
        local g = DirectedGraph.new_allow_self_loops()
        g:add_edge("A", "A")
        g:add_edge("A", "B")
        local closure = g:transitive_closure("A")
        assert.is_true(closure["A"])
        assert.is_true(closure["B"])
    end)
end)

-- =========================================================================
-- DirectedGraph — Predecessors and Successors
-- =========================================================================

describe("DirectedGraph - predecessors and successors", function()
    it("returns predecessors sorted", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        g:add_edge("C", "B")
        local preds = g:predecessors("B")
        assert.are.equal(2, #preds)
        assert.are.equal("A", preds[1])
        assert.are.equal("C", preds[2])
    end)

    it("returns successors sorted", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        g:add_edge("A", "C")
        local succs = g:successors("A")
        assert.are.equal(2, #succs)
        assert.are.equal("B", succs[1])
        assert.are.equal("C", succs[2])
    end)

    it("predecessors returns error for nonexistent node", function()
        local g = DirectedGraph.new()
        local preds, err = g:predecessors("X")
        assert.is_nil(preds)
        assert.are.equal("node_not_found", err.type)
    end)

    it("successors returns error for nonexistent node", function()
        local g = DirectedGraph.new()
        local succs, err = g:successors("X")
        assert.is_nil(succs)
        assert.are.equal("node_not_found", err.type)
    end)

    it("returns empty predecessors for root node", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        local preds = g:predecessors("A")
        assert.are.equal(0, #preds)
    end)

    it("returns empty successors for leaf node", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        local succs = g:successors("B")
        assert.are.equal(0, #succs)
    end)
end)

-- =========================================================================
-- DirectedGraph — Linear chain: A -> B -> C -> D
-- =========================================================================

describe("DirectedGraph - linear chain", function()
    local function build_linear_chain()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        g:add_edge("B", "C")
        g:add_edge("C", "D")
        return g
    end

    it("topological sort returns correct order", function()
        local g = build_linear_chain()
        local result = g:topological_sort()
        assert.are.same({"A", "B", "C", "D"}, result)
    end)

    it("independent groups has 4 levels", function()
        local g = build_linear_chain()
        local groups = g:independent_groups()
        assert.are.equal(4, #groups)
        assert.are.same({"A"}, groups[1])
        assert.are.same({"B"}, groups[2])
        assert.are.same({"C"}, groups[3])
        assert.are.same({"D"}, groups[4])
    end)

    it("has no cycle", function()
        local g = build_linear_chain()
        assert.is_false(g:has_cycle())
    end)
end)

-- =========================================================================
-- DirectedGraph — Diamond: A->B, A->C, B->D, C->D
-- =========================================================================

describe("DirectedGraph - diamond", function()
    local function build_diamond()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        g:add_edge("A", "C")
        g:add_edge("B", "D")
        g:add_edge("C", "D")
        return g
    end

    it("independent groups has 3 levels with B,C parallel", function()
        local g = build_diamond()
        local groups = g:independent_groups()
        assert.are.equal(3, #groups)
        assert.are.same({"A"}, groups[1])
        assert.are.same({"B", "C"}, groups[2])
        assert.are.same({"D"}, groups[3])
    end)

    it("transitive closure of A reaches B, C, D", function()
        local g = build_diamond()
        local closure = g:transitive_closure("A")
        assert.is_true(closure["B"])
        assert.is_true(closure["C"])
        assert.is_true(closure["D"])
    end)

    it("transitive dependents of D is empty", function()
        local g = build_diamond()
        local deps = g:transitive_dependents("D")
        local count = 0
        for _ in pairs(deps) do count = count + 1 end
        assert.are.equal(0, count)
    end)

    it("transitive dependents of A includes B, C, D", function()
        local g = build_diamond()
        local deps = g:transitive_dependents("A")
        assert.is_true(deps["B"])
        assert.is_true(deps["C"])
        assert.is_true(deps["D"])
    end)

    it("has no cycle", function()
        local g = build_diamond()
        assert.is_false(g:has_cycle())
    end)
end)

-- =========================================================================
-- DirectedGraph — Cycle detection
-- =========================================================================

describe("DirectedGraph - cycle detection", function()
    it("detects a 3-node cycle", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        g:add_edge("B", "C")
        g:add_edge("C", "A")
        assert.is_true(g:has_cycle())
    end)

    it("topological sort fails on cycle", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        g:add_edge("B", "C")
        g:add_edge("C", "A")
        local result, err = g:topological_sort()
        assert.is_nil(result)
        assert.are.equal("cycle", err.type)
    end)

    it("diamond has no cycle", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        g:add_edge("A", "C")
        g:add_edge("B", "D")
        g:add_edge("C", "D")
        assert.is_false(g:has_cycle())
    end)

    it("independent_groups fails on cycle", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        g:add_edge("B", "A")
        local groups, err = g:independent_groups()
        assert.is_nil(groups)
        assert.are.equal("cycle", err.type)
    end)
end)

-- =========================================================================
-- DirectedGraph — Affected nodes
-- =========================================================================

describe("DirectedGraph - affected nodes", function()
    local function build_diamond()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        g:add_edge("A", "C")
        g:add_edge("B", "D")
        g:add_edge("C", "D")
        return g
    end

    it("leaf node only affects itself", function()
        local g = build_diamond()
        local affected = g:affected_nodes({D = true})
        assert.is_true(affected["D"])
        local count = 0
        for _ in pairs(affected) do count = count + 1 end
        assert.are.equal(1, count)
    end)

    it("root node affects everything", function()
        local g = build_diamond()
        local affected = g:affected_nodes({A = true})
        assert.is_true(affected["A"])
        assert.is_true(affected["B"])
        assert.is_true(affected["C"])
        assert.is_true(affected["D"])
    end)

    it("middle node affects downstream", function()
        local g = build_diamond()
        local affected = g:affected_nodes({B = true})
        assert.is_true(affected["B"])
        assert.is_true(affected["D"])
        local count = 0
        for _ in pairs(affected) do count = count + 1 end
        assert.are.equal(2, count)
    end)

    it("nonexistent node produces empty affected set", function()
        local g = build_diamond()
        local affected = g:affected_nodes({X = true})
        local count = 0
        for _ in pairs(affected) do count = count + 1 end
        assert.are.equal(0, count)
    end)

    it("affected_nodes_list returns sorted array", function()
        local g = build_diamond()
        local list = g:affected_nodes_list({A = true})
        assert.are.same({"A", "B", "C", "D"}, list)
    end)

    it("affected_nodes_list for leaf", function()
        local g = build_diamond()
        local list = g:affected_nodes_list({D = true})
        assert.are.same({"D"}, list)
    end)
end)

-- =========================================================================
-- DirectedGraph — Transitive closure errors
-- =========================================================================

describe("DirectedGraph - transitive closure errors", function()
    it("transitive_closure returns error for nonexistent node", function()
        local g = DirectedGraph.new()
        local result, err = g:transitive_closure("X")
        assert.is_nil(result)
        assert.are.equal("node_not_found", err.type)
    end)

    it("transitive_dependents returns error for nonexistent node", function()
        local g = DirectedGraph.new()
        local result, err = g:transitive_dependents("X")
        assert.is_nil(result)
        assert.are.equal("node_not_found", err.type)
    end)
end)

-- =========================================================================
-- DirectedGraph — Nodes returns sorted
-- =========================================================================

describe("DirectedGraph - nodes ordering", function()
    it("nodes are returned in sorted order", function()
        local g = DirectedGraph.new()
        g:add_node("C")
        g:add_node("A")
        g:add_node("B")
        assert.are.same({"A", "B", "C"}, g:nodes())
    end)
end)

-- =========================================================================
-- DirectedGraph — Real repo dependency graph
-- =========================================================================

describe("DirectedGraph - repo graph", function()
    local function build_repo_graph()
        local g = DirectedGraph.new()
        -- Independent roots
        for _, pkg in ipairs({
            "logic-gates", "grammar-tools", "virtual-machine",
            "jvm-simulator", "clr-simulator", "wasm-simulator",
            "intel4004-simulator", "html-renderer",
        }) do
            g:add_node(pkg)
        end
        -- Dependency edges
        g:add_edge("logic-gates", "arithmetic")
        g:add_edge("arithmetic", "cpu-simulator")
        g:add_edge("cpu-simulator", "arm-simulator")
        g:add_edge("cpu-simulator", "riscv-simulator")
        g:add_edge("grammar-tools", "lexer")
        g:add_edge("lexer", "parser")
        g:add_edge("grammar-tools", "parser")
        g:add_edge("lexer", "bytecode-compiler")
        g:add_edge("parser", "bytecode-compiler")
        g:add_edge("virtual-machine", "bytecode-compiler")
        g:add_edge("lexer", "pipeline")
        g:add_edge("parser", "pipeline")
        g:add_edge("bytecode-compiler", "pipeline")
        g:add_edge("virtual-machine", "pipeline")
        g:add_edge("arm-simulator", "assembler")
        g:add_edge("virtual-machine", "jit-compiler")
        g:add_edge("assembler", "jit-compiler")
        return g
    end

    it("has no cycle", function()
        local g = build_repo_graph()
        assert.is_false(g:has_cycle())
    end)

    it("topological sort includes all nodes", function()
        local g = build_repo_graph()
        local order = g:topological_sort()
        assert.are.equal(g:size(), #order)
    end)

    it("topological sort respects all edges", function()
        local g = build_repo_graph()
        local order = g:topological_sort()
        local pos = {}
        for i, n in ipairs(order) do
            pos[n] = i
        end
        for _, edge in ipairs(g:edges()) do
            assert.is_true(pos[edge[1]] < pos[edge[2]],
                edge[1] .. " should come before " .. edge[2])
        end
    end)

    it("independent groups level 0 contains all roots", function()
        local g = build_repo_graph()
        local groups = g:independent_groups()
        local level0 = {}
        for _, n in ipairs(groups[1]) do
            level0[n] = true
        end
        for _, root in ipairs({"logic-gates", "grammar-tools", "virtual-machine",
                                "jvm-simulator", "clr-simulator"}) do
            assert.is_true(level0[root], root .. " should be in level 0")
        end
    end)
end)

-- =========================================================================
-- Error constructors
-- =========================================================================

describe("Error constructors", function()
    it("CycleError has correct type and message", function()
        local err = dg.CycleError()
        assert.are.equal("cycle", err.type)
        assert.are.equal("graph contains a cycle", err.message)
    end)

    it("NodeNotFoundError has correct fields", function()
        local err = dg.NodeNotFoundError("X")
        assert.are.equal("node_not_found", err.type)
        assert.are.equal("X", err.node)
        assert.truthy(err.message:find("X"))
    end)

    it("EdgeNotFoundError has correct fields", function()
        local err = dg.EdgeNotFoundError("A", "B")
        assert.are.equal("edge_not_found", err.type)
        assert.are.equal("A", err.from)
        assert.are.equal("B", err.to)
    end)

    it("LabelNotFoundError has correct fields", function()
        local err = dg.LabelNotFoundError("A", "B", "compile")
        assert.are.equal("label_not_found", err.type)
        assert.are.equal("A", err.from)
        assert.are.equal("B", err.to)
        assert.are.equal("compile", err.label)
    end)
end)

-- =========================================================================
-- LabeledGraph — Empty graph tests
-- =========================================================================

describe("LabeledGraph - empty graph", function()
    it("has no nodes", function()
        local lg = LabeledGraph.new()
        assert.are.equal(0, #lg:nodes())
    end)

    it("has no edges", function()
        local lg = LabeledGraph.new()
        assert.are.equal(0, #lg:edges())
    end)

    it("has size 0", function()
        local lg = LabeledGraph.new()
        assert.are.equal(0, lg:size())
    end)

    it("returns empty topological sort", function()
        local lg = LabeledGraph.new()
        local result, err = lg:topological_sort()
        assert.is_nil(err)
        assert.are.equal(0, #result)
    end)

    it("has no cycle", function()
        local lg = LabeledGraph.new()
        assert.is_false(lg:has_cycle())
    end)
end)

-- =========================================================================
-- LabeledGraph — Node operations
-- =========================================================================

describe("LabeledGraph - node operations", function()
    it("adds a node", function()
        local lg = LabeledGraph.new()
        lg:add_node("A")
        assert.is_true(lg:has_node("A"))
        assert.are.equal(1, lg:size())
    end)

    it("add_node is idempotent", function()
        local lg = LabeledGraph.new()
        lg:add_node("A")
        lg:add_node("A")
        assert.are.equal(1, lg:size())
    end)

    it("removes a node", function()
        local lg = LabeledGraph.new()
        lg:add_node("A")
        local ok = lg:remove_node("A")
        assert.is_true(ok)
        assert.is_false(lg:has_node("A"))
    end)

    it("returns error when removing nonexistent node", function()
        local lg = LabeledGraph.new()
        local ok, err = lg:remove_node("X")
        assert.is_nil(ok)
        assert.are.equal("node_not_found", err.type)
    end)

    it("remove_node cleans up labels", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("B", "C", "test")
        lg:remove_node("B")
        assert.is_false(lg:has_edge("A", "B"))
        assert.is_false(lg:has_edge("B", "C"))
        local labels = lg:labels("A", "B")
        local count = 0
        for _ in pairs(labels) do count = count + 1 end
        assert.are.equal(0, count)
    end)

    it("nodes returns sorted", function()
        local lg = LabeledGraph.new()
        lg:add_node("C")
        lg:add_node("A")
        lg:add_node("B")
        assert.are.same({"A", "B", "C"}, lg:nodes())
    end)
end)

-- =========================================================================
-- LabeledGraph — Edge operations (single label)
-- =========================================================================

describe("LabeledGraph - edge operations", function()
    it("adds an edge with label", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        assert.is_true(lg:has_edge("A", "B"))
        assert.is_true(lg:has_edge_with_label("A", "B", "compile"))
    end)

    it("add_edge implicitly adds nodes", function()
        local lg = LabeledGraph.new()
        lg:add_edge("X", "Y", "dep")
        assert.is_true(lg:has_node("X"))
        assert.is_true(lg:has_node("Y"))
    end)

    it("edges are directed", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        assert.is_false(lg:has_edge("B", "A"))
    end)

    it("self-loop panics in default labeled graph", function()
        local lg = LabeledGraph.new()
        assert.has_error(function()
            lg:add_edge("A", "A", "loop")
        end)
    end)

    it("self-loop allowed in self-loop labeled graph", function()
        local lg = LabeledGraph.new_allow_self_loops()
        lg:add_edge("A", "A", "retry")
        assert.is_true(lg:has_edge("A", "A"))
        assert.is_true(lg:has_edge_with_label("A", "A", "retry"))
    end)
end)

-- =========================================================================
-- LabeledGraph — Multiple labels on same edge
-- =========================================================================

describe("LabeledGraph - multiple labels", function()
    it("supports multiple labels on same edge", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("A", "B", "test")
        assert.is_true(lg:has_edge_with_label("A", "B", "compile"))
        assert.is_true(lg:has_edge_with_label("A", "B", "test"))
    end)

    it("edges output has one triple per label", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("A", "B", "test")
        local edges = lg:edges()
        assert.are.equal(2, #edges)
        assert.are.same({"A", "B", "compile"}, edges[1])
        assert.are.same({"A", "B", "test"}, edges[2])
    end)

    it("labels method returns all labels", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("A", "B", "test")
        lg:add_edge("A", "B", "runtime")
        local labels = lg:labels("A", "B")
        assert.is_true(labels["compile"])
        assert.is_true(labels["test"])
        assert.is_true(labels["runtime"])
    end)

    it("labels returns empty for nonexistent edge", function()
        local lg = LabeledGraph.new()
        local labels = lg:labels("X", "Y")
        local count = 0
        for _ in pairs(labels) do count = count + 1 end
        assert.are.equal(0, count)
    end)

    it("duplicate label is no-op", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("A", "B", "compile")
        local labels = lg:labels("A", "B")
        local count = 0
        for _ in pairs(labels) do count = count + 1 end
        assert.are.equal(1, count)
    end)
end)

-- =========================================================================
-- LabeledGraph — Remove edge (with labels)
-- =========================================================================

describe("LabeledGraph - remove edge", function()
    it("removes single-label edge completely", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        local ok = lg:remove_edge("A", "B", "compile")
        assert.is_true(ok)
        assert.is_false(lg:has_edge("A", "B"))
        assert.is_false(lg:has_edge_with_label("A", "B", "compile"))
    end)

    it("removes one label, edge persists with remaining", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("A", "B", "test")
        lg:remove_edge("A", "B", "compile")
        assert.is_true(lg:has_edge("A", "B"))
        assert.is_false(lg:has_edge_with_label("A", "B", "compile"))
        assert.is_true(lg:has_edge_with_label("A", "B", "test"))
    end)

    it("returns error for nonexistent edge", function()
        local lg = LabeledGraph.new()
        local ok, err = lg:remove_edge("A", "B", "x")
        assert.is_nil(ok)
        assert.are.equal("edge_not_found", err.type)
    end)

    it("returns error for nonexistent label", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        local ok, err = lg:remove_edge("A", "B", "nonexistent")
        assert.is_nil(ok)
        assert.are.equal("label_not_found", err.type)
    end)

    it("removing all labels removes the edge", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("A", "B", "test")
        lg:remove_edge("A", "B", "compile")
        lg:remove_edge("A", "B", "test")
        assert.is_false(lg:has_edge("A", "B"))
        -- Nodes should still exist
        assert.is_true(lg:has_node("A"))
        assert.is_true(lg:has_node("B"))
    end)
end)

-- =========================================================================
-- LabeledGraph — HasEdge / HasEdgeWithLabel
-- =========================================================================

describe("LabeledGraph - has_edge queries", function()
    it("has_edge false for empty graph", function()
        local lg = LabeledGraph.new()
        assert.is_false(lg:has_edge("X", "Y"))
    end)

    it("has_edge_with_label false for empty graph", function()
        local lg = LabeledGraph.new()
        assert.is_false(lg:has_edge_with_label("X", "Y", "z"))
    end)

    it("has_edge_with_label false for wrong label", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        assert.is_false(lg:has_edge_with_label("A", "B", "wrong"))
    end)
end)

-- =========================================================================
-- LabeledGraph — Successors / Predecessors
-- =========================================================================

describe("LabeledGraph - successors and predecessors", function()
    it("successors returns all successors", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("A", "C", "test")
        local succs = lg:successors("A")
        assert.are.same({"B", "C"}, succs)
    end)

    it("successors returns error for nonexistent node", function()
        local lg = LabeledGraph.new()
        local succs, err = lg:successors("X")
        assert.is_nil(succs)
        assert.are.equal("node_not_found", err.type)
    end)

    it("successors_with_label filters by label", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("A", "C", "test")
        lg:add_edge("A", "D", "compile")
        local succs = lg:successors_with_label("A", "compile")
        assert.are.same({"B", "D"}, succs)
    end)

    it("successors_with_label returns error for nonexistent node", function()
        local lg = LabeledGraph.new()
        local succs, err = lg:successors_with_label("X", "compile")
        assert.is_nil(succs)
        assert.are.equal("node_not_found", err.type)
    end)

    it("successors_with_label returns empty for nonexistent label", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        local succs = lg:successors_with_label("A", "nonexistent")
        assert.are.equal(0, #succs)
    end)

    it("predecessors returns all predecessors", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "C", "compile")
        lg:add_edge("B", "C", "test")
        local preds = lg:predecessors("C")
        assert.are.same({"A", "B"}, preds)
    end)

    it("predecessors returns error for nonexistent node", function()
        local lg = LabeledGraph.new()
        local preds, err = lg:predecessors("X")
        assert.is_nil(preds)
        assert.are.equal("node_not_found", err.type)
    end)

    it("predecessors_with_label filters by label", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "C", "compile")
        lg:add_edge("B", "C", "test")
        lg:add_edge("D", "C", "compile")
        local preds = lg:predecessors_with_label("C", "compile")
        assert.are.same({"A", "D"}, preds)
    end)

    it("predecessors_with_label returns error for nonexistent node", function()
        local lg = LabeledGraph.new()
        local preds, err = lg:predecessors_with_label("X", "compile")
        assert.is_nil(preds)
        assert.are.equal("node_not_found", err.type)
    end)

    it("predecessors_with_label returns empty for nonexistent label", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        local preds = lg:predecessors_with_label("B", "nonexistent")
        assert.are.equal(0, #preds)
    end)
end)

-- =========================================================================
-- LabeledGraph — Algorithm delegation
-- =========================================================================

describe("LabeledGraph - algorithm delegation", function()
    it("topological sort works through labels", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("B", "C", "compile")
        local result = lg:topological_sort()
        assert.are.same({"A", "B", "C"}, result)
    end)

    it("has_cycle false for DAG", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "dep")
        lg:add_edge("B", "C", "dep")
        assert.is_false(lg:has_cycle())
    end)

    it("has_cycle true for cyclic graph", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "dep")
        lg:add_edge("B", "C", "dep")
        lg:add_edge("C", "A", "dep")
        assert.is_true(lg:has_cycle())
    end)

    it("transitive_closure works through labels", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("B", "C", "compile")
        lg:add_edge("A", "D", "test")
        local closure = lg:transitive_closure("A")
        assert.is_true(closure["B"])
        assert.is_true(closure["C"])
        assert.is_true(closure["D"])
    end)

    it("transitive_closure returns error for nonexistent node", function()
        local lg = LabeledGraph.new()
        local result, err = lg:transitive_closure("X")
        assert.is_nil(result)
        assert.are.equal("node_not_found", err.type)
    end)
end)

-- =========================================================================
-- LabeledGraph — graph() accessor
-- =========================================================================

describe("LabeledGraph - graph accessor", function()
    it("returns the underlying DirectedGraph", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        local g = lg:graph()
        assert.is_not_nil(g)
        assert.is_true(g:has_edge("A", "B"))
    end)

    it("independent_groups via graph accessor", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("A", "C", "compile")
        lg:add_edge("B", "D", "test")
        lg:add_edge("C", "D", "test")
        local groups = lg:graph():independent_groups()
        assert.are.equal(3, #groups)
    end)

    it("affected_nodes via graph accessor", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("B", "C", "compile")
        local affected = lg:graph():affected_nodes({A = true})
        local count = 0
        for _ in pairs(affected) do count = count + 1 end
        assert.are.equal(3, count)
    end)
end)

-- =========================================================================
-- LabeledGraph — Complex scenarios
-- =========================================================================

describe("LabeledGraph - complex scenarios", function()
    it("diamond with mixed labels", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("A", "C", "test")
        lg:add_edge("B", "D", "compile")
        lg:add_edge("C", "D", "runtime")

        local order = lg:topological_sort()
        assert.are.equal(4, #order)

        local compile_succs = lg:successors_with_label("A", "compile")
        assert.are.same({"B"}, compile_succs)

        local test_succs = lg:successors_with_label("A", "test")
        assert.are.same({"C"}, test_succs)
    end)

    it("build system scenario", function()
        local lg = LabeledGraph.new()
        lg:add_node("logic-gates")
        lg:add_edge("logic-gates", "arithmetic", "compile")
        lg:add_edge("arithmetic", "cpu-simulator", "compile")
        lg:add_edge("logic-gates", "test-harness", "test")

        local compile_succs = lg:successors_with_label("logic-gates", "compile")
        assert.are.same({"arithmetic"}, compile_succs)

        local test_succs = lg:successors_with_label("logic-gates", "test")
        assert.are.same({"test-harness"}, test_succs)

        local all_succs = lg:successors("logic-gates")
        assert.are.equal(2, #all_succs)
    end)

    it("edges returns sorted triples", function()
        local lg = LabeledGraph.new()
        lg:add_edge("C", "D", "z")
        lg:add_edge("A", "B", "a")
        lg:add_edge("A", "B", "b")
        local edges = lg:edges()
        assert.are.equal(3, #edges)
        assert.are.same({"A", "B", "a"}, edges[1])
        assert.are.same({"A", "B", "b"}, edges[2])
        assert.are.same({"C", "D", "z"}, edges[3])
    end)

    it("labels returns a copy", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        local labels = lg:labels("A", "B")
        labels["hacked"] = true
        local internal_labels = lg:labels("A", "B")
        assert.is_nil(internal_labels["hacked"])
    end)

    it("remove_node with multiple labels", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("A", "B", "test")
        lg:add_edge("B", "C", "runtime")
        lg:remove_node("B")
        assert.is_false(lg:has_edge("A", "B"))
        assert.is_false(lg:has_edge("B", "C"))
        local labels = lg:labels("A", "B")
        local count = 0
        for _ in pairs(labels) do count = count + 1 end
        assert.are.equal(0, count)
    end)

    it("self-loop with multiple labels", function()
        local lg = LabeledGraph.new_allow_self_loops()
        lg:add_edge("A", "A", "retry")
        lg:add_edge("A", "A", "refresh")
        local labels = lg:labels("A", "A")
        assert.is_true(labels["retry"])
        assert.is_true(labels["refresh"])
    end)

    it("remove self-loop labels one at a time", function()
        local lg = LabeledGraph.new_allow_self_loops()
        lg:add_edge("A", "A", "retry")
        lg:add_edge("A", "A", "refresh")
        lg:remove_edge("A", "A", "retry")
        assert.is_true(lg:has_edge("A", "A"))
        lg:remove_edge("A", "A", "refresh")
        assert.is_false(lg:has_edge("A", "A"))
    end)

    it("isolated node in topological sort", function()
        local lg = LabeledGraph.new()
        lg:add_node("isolated")
        lg:add_edge("A", "B", "dep")
        local order = lg:topological_sort()
        assert.are.equal(3, #order)
    end)

    it("edges after removal", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("A", "B", "test")
        lg:remove_edge("A", "B", "compile")
        local edges = lg:edges()
        assert.are.equal(1, #edges)
        assert.are.same({"A", "B", "test"}, edges[1])
    end)
end)

-- =========================================================================
-- Visualization — DOT format
-- =========================================================================

describe("Visualization - DOT format", function()
    it("renders simple graph", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        local dot = viz.to_dot(g)
        assert.truthy(contains(dot, "digraph G"))
        assert.truthy(contains(dot, "A -> B"))
        assert.truthy(contains(dot, "rankdir=LR"))
    end)

    it("renders with custom options", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        local dot = viz.to_dot(g, {name = "MyGraph", rankdir = "TB"})
        assert.truthy(contains(dot, "digraph MyGraph"))
        assert.truthy(contains(dot, "rankdir=TB"))
    end)

    it("renders initial state marker", function()
        local g = DirectedGraph.new()
        g:add_node("start")
        local dot = viz.to_dot(g, {initial = "start"})
        assert.truthy(contains(dot, "[shape=none]"))
        assert.truthy(contains(dot, "-> start"))
    end)

    it("renders node attributes", function()
        local g = DirectedGraph.new()
        g:add_node("A")
        local dot = viz.to_dot(g, {
            node_attrs = {A = {shape = "circle"}}
        })
        assert.truthy(contains(dot, "shape=circle"))
    end)

    it("renders isolated nodes", function()
        local g = DirectedGraph.new()
        g:add_node("isolated")
        local dot = viz.to_dot(g)
        assert.truthy(contains(dot, "isolated"))
    end)
end)

describe("Visualization - labeled DOT format", function()
    it("renders labeled edges", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        local dot = viz.labeled_to_dot(lg)
        assert.truthy(contains(dot, 'label="compile"'))
    end)

    it("combines multiple labels on same edge", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("A", "B", "test")
        local dot = viz.labeled_to_dot(lg)
        assert.truthy(contains(dot, 'label="compile, test"'))
    end)

    it("renders with initial state", function()
        local lg = LabeledGraph.new()
        lg:add_node("start")
        local dot = viz.labeled_to_dot(lg, {initial = "start"})
        assert.truthy(contains(dot, "-> start"))
    end)
end)

-- =========================================================================
-- Visualization — Mermaid format
-- =========================================================================

describe("Visualization - Mermaid format", function()
    it("renders simple graph", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        local mermaid = viz.to_mermaid(g)
        assert.truthy(contains(mermaid, "graph LR"))
        assert.truthy(contains(mermaid, "A --> B"))
    end)

    it("renders with custom direction", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        local mermaid = viz.to_mermaid(g, "TD")
        assert.truthy(contains(mermaid, "graph TD"))
    end)

    it("defaults to LR direction", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        local mermaid = viz.to_mermaid(g)
        assert.truthy(contains(mermaid, "graph LR"))
    end)
end)

describe("Visualization - labeled Mermaid format", function()
    it("renders labeled edges with |label| syntax", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        local mermaid = viz.labeled_to_mermaid(lg)
        assert.truthy(contains(mermaid, "A -->|compile| B"))
    end)

    it("combines multiple labels on same edge", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "compile")
        lg:add_edge("A", "B", "test")
        local mermaid = viz.labeled_to_mermaid(lg)
        assert.truthy(contains(mermaid, "|compile, test|"))
    end)
end)

-- =========================================================================
-- Visualization — ASCII table format
-- =========================================================================

describe("Visualization - ASCII table", function()
    it("renders adjacency table", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        g:add_edge("A", "C")
        local tbl = viz.to_ascii_table(g)
        assert.truthy(contains(tbl, "Node"))
        assert.truthy(contains(tbl, "Successors"))
        assert.truthy(contains(tbl, "B, C"))
    end)

    it("shows dash for leaf nodes", function()
        local g = DirectedGraph.new()
        g:add_edge("A", "B")
        local tbl = viz.to_ascii_table(g)
        -- B has no successors, should show "-"
        assert.truthy(contains(tbl, "| -"))
    end)

    it("renders empty graph", function()
        local g = DirectedGraph.new()
        local tbl = viz.to_ascii_table(g)
        assert.truthy(contains(tbl, "Node"))
        assert.truthy(contains(tbl, "Successors"))
    end)
end)

describe("Visualization - labeled ASCII table", function()
    it("renders transition table", function()
        local lg = LabeledGraph.new()
        lg:add_edge("locked", "unlocked", "coin")
        lg:add_edge("unlocked", "locked", "push")
        lg:add_edge("locked", "locked", "push")
        lg:add_edge("unlocked", "unlocked", "coin")
        local tbl = viz.labeled_to_ascii_table(lg)
        assert.truthy(contains(tbl, "State"))
        assert.truthy(contains(tbl, "coin"))
        assert.truthy(contains(tbl, "push"))
    end)

    it("handles no-label graph", function()
        local lg = LabeledGraph.new()
        lg:add_node("A")
        lg:add_node("B")
        local tbl = viz.labeled_to_ascii_table(lg)
        assert.truthy(contains(tbl, "State"))
        assert.truthy(contains(tbl, "A"))
        assert.truthy(contains(tbl, "B"))
    end)

    it("shows dash for missing transitions", function()
        local lg = LabeledGraph.new()
        lg:add_edge("A", "B", "x")
        lg:add_node("C")
        local tbl = viz.labeled_to_ascii_table(lg)
        -- C has no transitions, should show "-"
        assert.truthy(contains(tbl, "-"))
    end)
end)
