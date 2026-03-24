-- test_tree.lua -- Comprehensive tests for the Tree package
-- ============================================================
--
-- Organized by category:
--
--  1. Construction -- creating trees, verifying initial state
--  2. add_child -- building trees, error cases
--  3. remove_subtree -- pruning branches, error cases
--  4. Queries -- parent, children, siblings, is_leaf, is_root, depth, height, etc.
--  5. Traversals -- preorder, postorder, level_order
--  6. path_to -- root-to-node paths
--  7. LCA -- lowest common ancestor
--  8. Subtree -- extracting subtrees
--  9. to_ascii -- ASCII visualization
-- 10. Edge cases -- single-node trees, deep chains, wide trees
-- 11. Graph -- accessing the underlying DirectedGraph

-- Add src/ and directed_graph to the module search path.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. "../../directed_graph/src/?.lua;" .. "../../directed_graph/src/?/init.lua;" .. package.path

local tree_mod = require("coding_adventures.tree")
local Tree = tree_mod.Tree

-- =========================================================================
-- Helper: Build a sample tree for many tests
-- =========================================================================
--
--         A
--        / \
--       B   C
--      / \   \
--     D   E   F
--     |
--     G
--
local function make_sample_tree()
    local t = Tree.new("A")
    t:add_child("A", "B")
    t:add_child("A", "C")
    t:add_child("B", "D")
    t:add_child("B", "E")
    t:add_child("C", "F")
    t:add_child("D", "G")
    return t
end

-- =========================================================================
-- 1. Construction
-- =========================================================================

describe("construction", function()
    it("creates a tree with the given root", function()
        local t = Tree.new("root")
        assert.are.equal("root", t:root())
    end)

    it("new tree has size one", function()
        local t = Tree.new("root")
        assert.are.equal(1, t:size())
    end)

    it("root is a leaf in a new tree", function()
        local t = Tree.new("root")
        assert.is_true(t:is_leaf("root"))
    end)

    it("root is_root in a new tree", function()
        local t = Tree.new("root")
        assert.is_true(t:is_root("root"))
    end)

    it("root has no parent", function()
        local t = Tree.new("root")
        assert.is_nil(t:parent("root"))
    end)

    it("root has no children", function()
        local t = Tree.new("root")
        local children = t:children("root")
        assert.are.same({}, children)
    end)

    it("root has depth zero", function()
        local t = Tree.new("root")
        assert.are.equal(0, t:depth("root"))
    end)

    it("new tree has height zero", function()
        local t = Tree.new("root")
        assert.are.equal(0, t:height())
    end)

    it("root is in the nodes list", function()
        local t = Tree.new("root")
        local ns = t:nodes()
        local found = false
        for _, n in ipairs(ns) do
            if n == "root" then found = true end
        end
        assert.is_true(found)
    end)

    it("tostring shows root and size", function()
        local t = Tree.new("root")
        assert.are.equal('Tree(root="root", size=1)', tostring(t))
    end)

    it("has a version", function()
        assert.are.equal("0.1.0", tree_mod.VERSION)
    end)
end)

-- =========================================================================
-- 2. add_child
-- =========================================================================

describe("add_child", function()
    it("adds one child", function()
        local t = Tree.new("root")
        t:add_child("root", "child")
        assert.are.equal(2, t:size())
    end)

    it("child has correct parent", function()
        local t = Tree.new("root")
        t:add_child("root", "child")
        assert.are.equal("root", t:parent("child"))
    end)

    it("parent has child in children list", function()
        local t = Tree.new("root")
        t:add_child("root", "child")
        local children = t:children("root")
        local found = false
        for _, c in ipairs(children) do
            if c == "child" then found = true end
        end
        assert.is_true(found)
    end)

    it("adds multiple children sorted", function()
        local t = Tree.new("root")
        t:add_child("root", "A")
        t:add_child("root", "B")
        t:add_child("root", "C")
        assert.are.same({"A", "B", "C"}, t:children("root"))
    end)

    it("adds child to non-root", function()
        local t = Tree.new("root")
        t:add_child("root", "mid")
        t:add_child("mid", "leaf")
        assert.are.equal("mid", t:parent("leaf"))
    end)

    it("builds a deep tree", function()
        local t = Tree.new("level0")
        for i = 1, 9 do
            t:add_child("level" .. (i - 1), "level" .. i)
        end
        assert.are.equal(10, t:size())
        assert.are.equal(9, t:depth("level9"))
    end)

    it("returns error for nonexistent parent", function()
        local t = Tree.new("root")
        local ok, err = t:add_child("nonexistent", "child")
        assert.is_nil(ok)
        assert.are.equal("node_not_found", err.type)
        assert.are.equal("nonexistent", err.node)
    end)

    it("returns error for duplicate child", function()
        local t = Tree.new("root")
        t:add_child("root", "child")
        local ok, err = t:add_child("root", "child")
        assert.is_nil(ok)
        assert.are.equal("duplicate_node", err.type)
        assert.are.equal("child", err.node)
    end)

    it("returns error when adding root as child", function()
        local t = Tree.new("root")
        local ok, err = t:add_child("root", "root")
        assert.is_nil(ok)
        assert.are.equal("duplicate_node", err.type)
    end)

    it("makes parent not a leaf", function()
        local t = Tree.new("root")
        assert.is_true(t:is_leaf("root"))
        t:add_child("root", "child")
        assert.is_false(t:is_leaf("root"))
    end)

    it("new child is a leaf", function()
        local t = Tree.new("root")
        t:add_child("root", "child")
        assert.is_true(t:is_leaf("child"))
    end)
end)

-- =========================================================================
-- 3. remove_subtree
-- =========================================================================

describe("remove_subtree", function()
    it("removes a leaf", function()
        local t = Tree.new("root")
        t:add_child("root", "leaf")
        t:remove_subtree("leaf")
        assert.are.equal(1, t:size())
        assert.is_false(t:has_node("leaf"))
    end)

    it("removes descendants", function()
        local t = make_sample_tree()
        t:remove_subtree("B")
        assert.are.equal(3, t:size())
        assert.is_false(t:has_node("B"))
        assert.is_false(t:has_node("D"))
        assert.is_false(t:has_node("E"))
        assert.is_false(t:has_node("G"))
    end)

    it("preserves siblings", function()
        local t = make_sample_tree()
        t:remove_subtree("B")
        assert.is_true(t:has_node("C"))
        assert.is_true(t:has_node("F"))
        assert.are.same({"C"}, t:children("A"))
    end)

    it("removes a deep subtree", function()
        local t = make_sample_tree()
        t:remove_subtree("D")
        assert.are.equal(5, t:size())
        assert.is_false(t:has_node("D"))
        assert.is_false(t:has_node("G"))
        assert.are.same({"E"}, t:children("B"))
    end)

    it("returns error for root removal", function()
        local t = Tree.new("root")
        local ok, err = t:remove_subtree("root")
        assert.is_nil(ok)
        assert.are.equal("root_removal", err.type)
    end)

    it("returns error for nonexistent node", function()
        local t = Tree.new("root")
        local ok, err = t:remove_subtree("nonexistent")
        assert.is_nil(ok)
        assert.are.equal("node_not_found", err.type)
    end)

    it("allows re-adding after removal", function()
        local t = Tree.new("root")
        t:add_child("root", "child")
        t:remove_subtree("child")
        t:add_child("root", "child")
        assert.is_true(t:has_node("child"))
    end)

    it("parent becomes leaf after removing only child", function()
        local t = Tree.new("root")
        t:add_child("root", "only_child")
        t:remove_subtree("only_child")
        assert.is_true(t:is_leaf("root"))
    end)
end)

-- =========================================================================
-- 4. Queries
-- =========================================================================

describe("parent", function()
    it("returns parent of a child", function()
        assert.are.equal("A", make_sample_tree():parent("B"))
    end)

    it("returns parent of grandchild", function()
        assert.are.equal("D", make_sample_tree():parent("G"))
    end)

    it("returns nil for root", function()
        assert.is_nil(make_sample_tree():parent("A"))
    end)

    it("returns error for nonexistent node", function()
        local _, err = make_sample_tree():parent("Z")
        assert.are.equal("node_not_found", err.type)
    end)
end)

describe("children", function()
    it("returns children of root", function()
        assert.are.same({"B", "C"}, make_sample_tree():children("A"))
    end)

    it("returns children of internal node", function()
        assert.are.same({"D", "E"}, make_sample_tree():children("B"))
    end)

    it("returns empty for leaf", function()
        assert.are.same({}, make_sample_tree():children("G"))
    end)

    it("returns error for nonexistent node", function()
        local _, err = make_sample_tree():children("Z")
        assert.are.equal("node_not_found", err.type)
    end)
end)

describe("siblings", function()
    it("returns sibling of node with sibling", function()
        assert.are.same({"C"}, make_sample_tree():siblings("B"))
    end)

    it("siblings are mutual", function()
        assert.are.same({"B"}, make_sample_tree():siblings("C"))
    end)

    it("returns empty for only child", function()
        assert.are.same({}, make_sample_tree():siblings("F"))
    end)

    it("returns empty for root", function()
        assert.are.same({}, make_sample_tree():siblings("A"))
    end)

    it("returns error for nonexistent node", function()
        local _, err = make_sample_tree():siblings("Z")
        assert.are.equal("node_not_found", err.type)
    end)

    it("returns multiple siblings sorted", function()
        local t = Tree.new("root")
        t:add_child("root", "A")
        t:add_child("root", "B")
        t:add_child("root", "C")
        t:add_child("root", "D")
        assert.are.same({"A", "C", "D"}, t:siblings("B"))
    end)
end)

describe("is_leaf", function()
    it("true for leaf nodes", function()
        local t = make_sample_tree()
        for _, node in ipairs({"G", "E", "F"}) do
            assert.is_true(t:is_leaf(node))
        end
    end)

    it("false for internal nodes", function()
        local t = make_sample_tree()
        for _, node in ipairs({"A", "B"}) do
            assert.is_false(t:is_leaf(node))
        end
    end)

    it("returns error for nonexistent node", function()
        local _, err = make_sample_tree():is_leaf("Z")
        assert.are.equal("node_not_found", err.type)
    end)
end)

describe("is_root", function()
    it("true for root", function()
        assert.is_true(make_sample_tree():is_root("A"))
    end)

    it("false for non-root", function()
        assert.is_false(make_sample_tree():is_root("B"))
    end)

    it("returns error for nonexistent node", function()
        local _, err = make_sample_tree():is_root("Z")
        assert.are.equal("node_not_found", err.type)
    end)
end)

describe("depth", function()
    it("root has depth 0", function()
        assert.are.equal(0, make_sample_tree():depth("A"))
    end)

    it("level-one nodes have depth 1", function()
        local t = make_sample_tree()
        assert.are.equal(1, t:depth("B"))
        assert.are.equal(1, t:depth("C"))
    end)

    it("level-two nodes have depth 2", function()
        local t = make_sample_tree()
        for _, n in ipairs({"D", "E", "F"}) do
            assert.are.equal(2, t:depth(n))
        end
    end)

    it("level-three node has depth 3", function()
        assert.are.equal(3, make_sample_tree():depth("G"))
    end)

    it("returns error for nonexistent node", function()
        local _, err = make_sample_tree():depth("Z")
        assert.are.equal("node_not_found", err.type)
    end)
end)

describe("height", function()
    it("sample tree has height 3", function()
        assert.are.equal(3, make_sample_tree():height())
    end)

    it("single node has height 0", function()
        assert.are.equal(0, Tree.new("root"):height())
    end)

    it("flat tree has height 1", function()
        local t = Tree.new("root")
        for i = 0, 4 do
            t:add_child("root", "child" .. i)
        end
        assert.are.equal(1, t:height())
    end)

    it("deep chain has correct height", function()
        local t = Tree.new("0")
        for i = 1, 19 do
            t:add_child(tostring(i - 1), tostring(i))
        end
        assert.are.equal(19, t:height())
    end)
end)

describe("size", function()
    it("sample tree has 7 nodes", function()
        assert.are.equal(7, make_sample_tree():size())
    end)

    it("grows after add", function()
        local t = Tree.new("root")
        assert.are.equal(1, t:size())
        t:add_child("root", "A")
        assert.are.equal(2, t:size())
    end)
end)

describe("nodes", function()
    it("returns all nodes sorted", function()
        assert.are.same(
            {"A", "B", "C", "D", "E", "F", "G"},
            make_sample_tree():nodes()
        )
    end)
end)

describe("leaves", function()
    it("returns leaves of sample tree sorted", function()
        assert.are.same({"E", "F", "G"}, make_sample_tree():leaves())
    end)

    it("single node is its own leaf", function()
        assert.are.same({"root"}, Tree.new("root"):leaves())
    end)

    it("flat tree leaves are all children", function()
        local t = Tree.new("root")
        t:add_child("root", "A")
        t:add_child("root", "B")
        t:add_child("root", "C")
        assert.are.same({"A", "B", "C"}, t:leaves())
    end)
end)

describe("has_node", function()
    it("true for existing node", function()
        assert.is_true(make_sample_tree():has_node("A"))
    end)

    it("false for nonexistent node", function()
        assert.is_false(make_sample_tree():has_node("Z"))
    end)
end)

-- =========================================================================
-- 5. Traversals
-- =========================================================================

describe("preorder", function()
    it("sample tree", function()
        assert.are.same(
            {"A", "B", "D", "G", "E", "C", "F"},
            make_sample_tree():preorder()
        )
    end)

    it("single node", function()
        assert.are.same({"root"}, Tree.new("root"):preorder())
    end)

    it("flat tree visits children in sorted order", function()
        local t = Tree.new("root")
        t:add_child("root", "C")
        t:add_child("root", "A")
        t:add_child("root", "B")
        assert.are.same({"root", "A", "B", "C"}, t:preorder())
    end)

    it("deep chain", function()
        local t = Tree.new("A")
        t:add_child("A", "B")
        t:add_child("B", "C")
        assert.are.same({"A", "B", "C"}, t:preorder())
    end)

    it("root is first", function()
        assert.are.equal("A", make_sample_tree():preorder()[1])
    end)
end)

describe("postorder", function()
    it("sample tree", function()
        assert.are.same(
            {"G", "D", "E", "B", "F", "C", "A"},
            make_sample_tree():postorder()
        )
    end)

    it("single node", function()
        assert.are.same({"root"}, Tree.new("root"):postorder())
    end)

    it("flat tree visits children in sorted order", function()
        local t = Tree.new("root")
        t:add_child("root", "C")
        t:add_child("root", "A")
        t:add_child("root", "B")
        assert.are.same({"A", "B", "C", "root"}, t:postorder())
    end)

    it("deep chain", function()
        local t = Tree.new("A")
        t:add_child("A", "B")
        t:add_child("B", "C")
        assert.are.same({"C", "B", "A"}, t:postorder())
    end)

    it("root is last", function()
        local po = make_sample_tree():postorder()
        assert.are.equal("A", po[#po])
    end)
end)

describe("level_order", function()
    it("sample tree", function()
        assert.are.same(
            {"A", "B", "C", "D", "E", "F", "G"},
            make_sample_tree():level_order()
        )
    end)

    it("single node", function()
        assert.are.same({"root"}, Tree.new("root"):level_order())
    end)

    it("flat tree visits children in sorted order", function()
        local t = Tree.new("root")
        t:add_child("root", "C")
        t:add_child("root", "A")
        t:add_child("root", "B")
        assert.are.same({"root", "A", "B", "C"}, t:level_order())
    end)

    it("deep chain", function()
        local t = Tree.new("A")
        t:add_child("A", "B")
        t:add_child("B", "C")
        assert.are.same({"A", "B", "C"}, t:level_order())
    end)

    it("root is first", function()
        assert.are.equal("A", make_sample_tree():level_order()[1])
    end)
end)

describe("traversals consistency", function()
    it("all traversals have same length", function()
        local t = make_sample_tree()
        assert.are.equal(7, #t:preorder())
        assert.are.equal(7, #t:postorder())
        assert.are.equal(7, #t:level_order())
    end)

    it("all traversals contain the same elements", function()
        local t = make_sample_tree()
        local pre = {table.unpack(t:preorder())}
        local post = {table.unpack(t:postorder())}
        local level = {table.unpack(t:level_order())}
        table.sort(pre)
        table.sort(post)
        table.sort(level)
        assert.are.same(pre, post)
        assert.are.same(pre, level)
    end)
end)

-- =========================================================================
-- 6. path_to
-- =========================================================================

describe("path_to", function()
    it("path to root", function()
        assert.are.same({"A"}, make_sample_tree():path_to("A"))
    end)

    it("path to child", function()
        assert.are.same({"A", "B"}, make_sample_tree():path_to("B"))
    end)

    it("path to grandchild", function()
        assert.are.same({"A", "B", "D"}, make_sample_tree():path_to("D"))
    end)

    it("path to deep node", function()
        assert.are.same({"A", "B", "D", "G"}, make_sample_tree():path_to("G"))
    end)

    it("path to right branch", function()
        assert.are.same({"A", "C", "F"}, make_sample_tree():path_to("F"))
    end)

    it("returns error for nonexistent node", function()
        local _, err = make_sample_tree():path_to("Z")
        assert.are.equal("node_not_found", err.type)
    end)

    it("path length equals depth plus one", function()
        local t = make_sample_tree()
        for _, node in ipairs(t:nodes()) do
            local path = t:path_to(node)
            local d = t:depth(node)
            assert.are.equal(d + 1, #path)
        end
    end)
end)

-- =========================================================================
-- 7. LCA
-- =========================================================================

describe("lca", function()
    it("same node", function()
        assert.are.equal("D", make_sample_tree():lca("D", "D"))
    end)

    it("siblings", function()
        assert.are.equal("B", make_sample_tree():lca("D", "E"))
    end)

    it("parent-child", function()
        assert.are.equal("B", make_sample_tree():lca("B", "D"))
    end)

    it("child-parent (symmetric)", function()
        assert.are.equal("B", make_sample_tree():lca("D", "B"))
    end)

    it("cousins", function()
        assert.are.equal("A", make_sample_tree():lca("D", "F"))
    end)

    it("root and leaf", function()
        assert.are.equal("A", make_sample_tree():lca("A", "G"))
    end)

    it("deep nodes in same subtree", function()
        assert.are.equal("B", make_sample_tree():lca("G", "E"))
    end)

    it("both leaves different subtrees", function()
        assert.are.equal("A", make_sample_tree():lca("G", "F"))
    end)

    it("returns error for nonexistent first node", function()
        local _, err = make_sample_tree():lca("Z", "A")
        assert.are.equal("node_not_found", err.type)
    end)

    it("returns error for nonexistent second node", function()
        local _, err = make_sample_tree():lca("A", "Z")
        assert.are.equal("node_not_found", err.type)
    end)

    it("root with root", function()
        assert.are.equal("A", make_sample_tree():lca("A", "A"))
    end)
end)

-- =========================================================================
-- 8. Subtree
-- =========================================================================

describe("subtree", function()
    it("leaf subtree", function()
        local sub = make_sample_tree():subtree("G")
        assert.are.equal("G", sub:root())
        assert.are.equal(1, sub:size())
    end)

    it("internal node subtree", function()
        local sub = make_sample_tree():subtree("B")
        assert.are.equal("B", sub:root())
        assert.are.equal(4, sub:size())
        assert.is_true(sub:has_node("D"))
        assert.is_true(sub:has_node("E"))
        assert.is_true(sub:has_node("G"))
    end)

    it("preserves structure", function()
        local sub = make_sample_tree():subtree("B")
        assert.are.same({"D", "E"}, sub:children("B"))
        assert.are.same({"G"}, sub:children("D"))
        assert.is_true(sub:is_leaf("G"))
        assert.is_true(sub:is_leaf("E"))
    end)

    it("root subtree equals whole tree", function()
        local t = make_sample_tree()
        local sub = t:subtree("A")
        assert.are.equal(t:size(), sub:size())
        assert.are.same(t:nodes(), sub:nodes())
    end)

    it("does not modify original", function()
        local t = make_sample_tree()
        local orig_size = t:size()
        t:subtree("B")
        assert.are.equal(orig_size, t:size())
    end)

    it("returns error for nonexistent node", function()
        local _, err = make_sample_tree():subtree("Z")
        assert.are.equal("node_not_found", err.type)
    end)

    it("subtree is independent", function()
        local t = make_sample_tree()
        local sub = t:subtree("B")
        sub:add_child("E", "new_node")
        assert.is_false(t:has_node("new_node"))
    end)

    it("right branch subtree", function()
        local sub = make_sample_tree():subtree("C")
        assert.are.equal("C", sub:root())
        assert.are.equal(2, sub:size())
        assert.are.same({"F"}, sub:children("C"))
    end)
end)

-- =========================================================================
-- 9. to_ascii
-- =========================================================================

describe("to_ascii", function()
    it("single node", function()
        assert.are.equal("root", Tree.new("root"):to_ascii())
    end)

    it("root with one child", function()
        local t = Tree.new("root")
        t:add_child("root", "child")
        assert.are.equal("root\n\xe2\x94\x94\xe2\x94\x80\xe2\x94\x80 child", t:to_ascii())
    end)

    it("root with two children", function()
        local t = Tree.new("root")
        t:add_child("root", "A")
        t:add_child("root", "B")
        assert.are.equal(
            "root\n\xe2\x94\x9c\xe2\x94\x80\xe2\x94\x80 A\n\xe2\x94\x94\xe2\x94\x80\xe2\x94\x80 B",
            t:to_ascii()
        )
    end)

    it("sample tree", function()
        local expected = "A\n"
            .. "\xe2\x94\x9c\xe2\x94\x80\xe2\x94\x80 B\n"
            .. "\xe2\x94\x82   \xe2\x94\x9c\xe2\x94\x80\xe2\x94\x80 D\n"
            .. "\xe2\x94\x82   \xe2\x94\x82   \xe2\x94\x94\xe2\x94\x80\xe2\x94\x80 G\n"
            .. "\xe2\x94\x82   \xe2\x94\x94\xe2\x94\x80\xe2\x94\x80 E\n"
            .. "\xe2\x94\x94\xe2\x94\x80\xe2\x94\x80 C\n"
            .. "    \xe2\x94\x94\xe2\x94\x80\xe2\x94\x80 F"
        assert.are.equal(expected, make_sample_tree():to_ascii())
    end)

    it("deep chain", function()
        local t = Tree.new("A")
        t:add_child("A", "B")
        t:add_child("B", "C")
        assert.are.equal(
            "A\n\xe2\x94\x94\xe2\x94\x80\xe2\x94\x80 B\n    \xe2\x94\x94\xe2\x94\x80\xe2\x94\x80 C",
            t:to_ascii()
        )
    end)

    it("wide tree", function()
        local t = Tree.new("root")
        t:add_child("root", "A")
        t:add_child("root", "B")
        t:add_child("root", "C")
        t:add_child("root", "D")
        local expected = "root\n"
            .. "\xe2\x94\x9c\xe2\x94\x80\xe2\x94\x80 A\n"
            .. "\xe2\x94\x9c\xe2\x94\x80\xe2\x94\x80 B\n"
            .. "\xe2\x94\x9c\xe2\x94\x80\xe2\x94\x80 C\n"
            .. "\xe2\x94\x94\xe2\x94\x80\xe2\x94\x80 D"
        assert.are.equal(expected, t:to_ascii())
    end)
end)

-- =========================================================================
-- 10. Edge cases
-- =========================================================================

describe("edge cases", function()
    it("single node traversals", function()
        local t = Tree.new("solo")
        assert.are.same({"solo"}, t:preorder())
        assert.are.same({"solo"}, t:postorder())
        assert.are.same({"solo"}, t:level_order())
    end)

    it("single node leaves", function()
        assert.are.same({"solo"}, Tree.new("solo"):leaves())
    end)

    it("deep chain height", function()
        local t = Tree.new("n0")
        for i = 1, 99 do
            t:add_child("n" .. (i - 1), "n" .. i)
        end
        assert.are.equal(99, t:height())
        assert.are.equal(100, t:size())
    end)

    it("wide tree height", function()
        local t = Tree.new("root")
        for i = 0, 99 do
            t:add_child("root", "child" .. i)
        end
        assert.are.equal(1, t:height())
        assert.are.equal(101, t:size())
    end)

    it("balanced binary tree", function()
        local t = Tree.new("1")
        t:add_child("1", "2")
        t:add_child("1", "3")
        t:add_child("2", "4")
        t:add_child("2", "5")
        t:add_child("3", "6")
        t:add_child("3", "7")
        assert.are.equal(7, t:size())
        assert.are.equal(2, t:height())
        assert.are.same({"4", "5", "6", "7"}, t:leaves())
    end)

    it("node names with spaces", function()
        local t = Tree.new("my root")
        t:add_child("my root", "my child")
        assert.are.equal("my root", t:parent("my child"))
    end)

    it("node names with special chars", function()
        local t = Tree.new("root:main")
        t:add_child("root:main", "child.1")
        assert.is_true(t:has_node("child.1"))
    end)

    it("path to single node", function()
        assert.are.same({"solo"}, Tree.new("solo"):path_to("solo"))
    end)

    it("lca in single node tree", function()
        assert.are.equal("solo", Tree.new("solo"):lca("solo", "solo"))
    end)

    it("subtree of single node", function()
        local sub = Tree.new("solo"):subtree("solo")
        assert.are.equal("solo", sub:root())
        assert.are.equal(1, sub:size())
    end)

    it("remove and rebuild", function()
        local t = Tree.new("root")
        t:add_child("root", "A")
        t:add_child("A", "B")
        t:remove_subtree("A")
        t:add_child("root", "A")
        t:add_child("A", "C")
        assert.are.same({"C"}, t:children("A"))
        assert.is_false(t:has_node("B"))
    end)
end)

-- =========================================================================
-- 11. Graph property
-- =========================================================================

describe("graph", function()
    it("has correct nodes", function()
        local t = make_sample_tree()
        local nodes = t:graph():nodes()
        table.sort(nodes)
        assert.are.same({"A", "B", "C", "D", "E", "F", "G"}, nodes)
    end)

    it("has correct edges", function()
        local edges = make_sample_tree():graph():edges()
        local edge_set = {}
        for _, e in ipairs(edges) do
            edge_set[e[1] .. "->" .. e[2]] = true
        end
        for _, e in ipairs({"A->B", "A->C", "B->D", "B->E", "C->F", "D->G"}) do
            assert.is_true(edge_set[e], "missing edge: " .. e)
        end
    end)

    it("has correct edge count", function()
        assert.are.equal(6, #make_sample_tree():graph():edges())
    end)

    it("has no cycles", function()
        assert.is_false(make_sample_tree():graph():has_cycle())
    end)

    it("topological sort starts with root", function()
        local topo = make_sample_tree():graph():topological_sort()
        assert.are.equal("A", topo[1])
    end)
end)

-- =========================================================================
-- Error constructor exports
-- =========================================================================

describe("error constructors", function()
    it("NodeNotFoundError is exported", function()
        local err = tree_mod.NodeNotFoundError("X")
        assert.are.equal("node_not_found", err.type)
        assert.are.equal("X", err.node)
    end)

    it("DuplicateNodeError is exported", function()
        local err = tree_mod.DuplicateNodeError("X")
        assert.are.equal("duplicate_node", err.type)
        assert.are.equal("X", err.node)
    end)

    it("RootRemovalError is exported", function()
        local err = tree_mod.RootRemovalError()
        assert.are.equal("root_removal", err.type)
    end)
end)
