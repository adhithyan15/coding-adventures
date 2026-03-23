-- Tests for the directed graph module.
--
-- The directed graph is the core data structure for dependency resolution.
-- These tests verify node/edge operations, topological sort (Kahn's
-- algorithm), transitive closure, and cycle detection.

-- Add lib/ to the module search path.
package.path = "../lib/?.lua;" .. "../lib/?/init.lua;" .. package.path

local DirectedGraph = require("build_tool.directed_graph")

describe("DirectedGraph", function()

    describe("add_node", function()
        it("adds a node to the graph", function()
            local g = DirectedGraph.new()
            g:add_node("a")
            assert.is_true(g:has_node("a"))
        end)

        it("is idempotent", function()
            local g = DirectedGraph.new()
            g:add_node("a")
            g:add_node("a")
            assert.is_true(g:has_node("a"))
        end)
    end)

    describe("add_edge", function()
        it("creates both nodes and the edge", function()
            local g = DirectedGraph.new()
            g:add_edge("a", "b")
            assert.is_true(g:has_node("a"))
            assert.is_true(g:has_node("b"))
        end)

        it("records successors and predecessors", function()
            local g = DirectedGraph.new()
            g:add_edge("a", "b")
            local succs = g:successors("a")
            local preds = g:predecessors("b")
            assert.are.equal(1, #succs)
            assert.are.equal("b", succs[1])
            assert.are.equal(1, #preds)
            assert.are.equal("a", preds[1])
        end)
    end)

    describe("has_node", function()
        it("returns false for missing nodes", function()
            local g = DirectedGraph.new()
            assert.is_false(g:has_node("missing"))
        end)
    end)

    describe("nodes", function()
        it("returns all nodes sorted", function()
            local g = DirectedGraph.new()
            g:add_edge("b", "a")
            g:add_node("c")
            local nodes = g:nodes()
            assert.are.same({"a", "b", "c"}, nodes)
        end)
    end)

    describe("transitive_closure", function()
        it("finds all reachable nodes", function()
            local g = DirectedGraph.new()
            g:add_edge("a", "b")
            g:add_edge("b", "c")
            local closure = g:transitive_closure("a")
            assert.is_true(closure["b"])
            assert.is_true(closure["c"])
        end)

        it("returns empty for nodes with no outgoing edges", function()
            local g = DirectedGraph.new()
            g:add_node("a")
            local closure = g:transitive_closure("a")
            local count = 0
            for _ in pairs(closure) do count = count + 1 end
            assert.are.equal(0, count)
        end)

        it("returns empty for missing nodes", function()
            local g = DirectedGraph.new()
            local closure = g:transitive_closure("missing")
            local count = 0
            for _ in pairs(closure) do count = count + 1 end
            assert.are.equal(0, count)
        end)
    end)

    describe("transitive_dependents", function()
        it("finds all nodes that depend on the given node", function()
            local g = DirectedGraph.new()
            g:add_edge("a", "b")
            g:add_edge("b", "c")
            local deps = g:transitive_dependents("c")
            assert.is_true(deps["a"])
            assert.is_true(deps["b"])
        end)

        it("returns empty for missing nodes", function()
            local g = DirectedGraph.new()
            local deps = g:transitive_dependents("missing")
            local count = 0
            for _ in pairs(deps) do count = count + 1 end
            assert.are.equal(0, count)
        end)
    end)

    describe("independent_groups", function()
        it("handles a linear chain", function()
            local g = DirectedGraph.new()
            g:add_edge("a", "b")
            g:add_edge("b", "c")
            local groups = g:independent_groups()
            assert.are.same({"a"}, groups[1])
            assert.are.same({"b"}, groups[2])
            assert.are.same({"c"}, groups[3])
        end)

        it("handles a diamond dependency", function()
            local g = DirectedGraph.new()
            g:add_edge("a", "b")
            g:add_edge("a", "c")
            g:add_edge("b", "d")
            g:add_edge("c", "d")
            local groups = g:independent_groups()
            assert.are.same({"a"}, groups[1])
            assert.are.same({"b", "c"}, groups[2])
            assert.are.same({"d"}, groups[3])
        end)

        it("returns empty for an empty graph", function()
            local g = DirectedGraph.new()
            local groups = g:independent_groups()
            assert.are.equal(0, #groups)
        end)

        it("groups isolated nodes together", function()
            local g = DirectedGraph.new()
            g:add_node("a")
            g:add_node("b")
            g:add_node("c")
            local groups = g:independent_groups()
            assert.are.equal(1, #groups)
            assert.are.same({"a", "b", "c"}, groups[1])
        end)

        it("detects cycles", function()
            local g = DirectedGraph.new()
            g:add_edge("a", "b")
            g:add_edge("b", "a")
            assert.has_error(function()
                g:independent_groups()
            end, "Dependency graph contains a cycle")
        end)
    end)
end)
