local script = arg and arg[0] or "tests/test_graph.lua"
local script_dir = script:match("^(.*)[/\\][^/\\]+$") or "."
local package_root = script_dir:gsub("[/\\]tests$", "")
if package_root == "tests" then
    package_root = "."
end

package.path = package_root .. "/src/?.lua;" .. package_root .. "/src/?/init.lua;" .. package.path

local graph = require("coding_adventures.graph")
local Graph = graph.Graph
local GraphRepr = graph.GraphRepr

local function assert_true(value, message)
    if not value then
        error(message or "expected true")
    end
end

local function assert_equals(expected, actual, message)
    if expected ~= actual then
        error((message or "values differ") .. string.format(" (expected=%s actual=%s)", tostring(expected), tostring(actual)))
    end
end

local function assert_list_equals(expected, actual, message)
    assert_equals(#expected, #actual, (message or "list length mismatch"))
    for i = 1, #expected do
        assert_equals(expected[i], actual[i], message or ("list mismatch at index " .. i))
    end
end

local function make_graph(repr)
    local g = Graph.new({ repr = repr })
    g:add_edge("London", "Paris", 300)
    g:add_edge("London", "Amsterdam", 520)
    g:add_edge("Paris", "Berlin", 878)
    g:add_edge("Amsterdam", "Berlin", 655)
    g:add_edge("Amsterdam", "Brussels", 180)
    return g
end

local function run_for_repr(repr)
    local g = make_graph(repr)
    assert_true(g:has_node("London"))
    assert_true(g:has_edge("London", "Paris"))
    assert_equals(5, g:size())
    assert_equals(520, assert(g:edge_weight("London", "Amsterdam")))

    local neighbors = assert(g:neighbors("Amsterdam"))
    assert_list_equals({ "Berlin", "Brussels", "London" }, neighbors)
    assert_equals(3, assert(g:degree("Amsterdam")))

    local bfs_order = assert(graph.bfs(g, "London"))
    assert_list_equals({ "London", "Amsterdam", "Paris", "Berlin", "Brussels" }, bfs_order)

    local dfs_order = assert(graph.dfs(g, "London"))
    assert_list_equals({ "London", "Amsterdam", "Berlin", "Paris", "Brussels" }, dfs_order)

    assert_true(graph.is_connected(g))
    assert_true(graph.has_cycle(g))

    local path = assert(graph.shortest_path(g, "London", "Berlin"))
    assert_list_equals({ "London", "Amsterdam", "Berlin" }, path)

    local mst = assert(graph.minimum_spanning_tree(g))
    assert_equals(4, #mst)
    assert_equals("Amsterdam", mst[1][1])
    assert_equals("Brussels", mst[1][2])
    assert_equals(180, mst[1][3])
end

run_for_repr(GraphRepr.ADJACENCY_LIST)
run_for_repr(GraphRepr.ADJACENCY_MATRIX)

do
    local g = Graph.new()
    g:add_edge("A", "B")
    g:add_edge("C", "D")
    local components = graph.connected_components(g)
    assert_equals(2, #components)
    assert_list_equals({ "A", "B" }, components[1])
    assert_list_equals({ "C", "D" }, components[2])
    assert_true(not graph.is_connected(g), "expected disconnected graph")
end

do
    local g = Graph.new()
    g:add_edge("A", "B")
    assert_true(g:remove_edge("A", "B"))
    assert_true(not g:has_edge("A", "B"))
    assert_true(g:remove_node("A"))
    assert_true(not g:has_node("A"))
end

do
    local g = Graph.new()
    assert_true(g:set_graph_property("name", "city-map"))
    assert_true(g:set_graph_property("version", 1))
    assert_equals("city-map", g:graph_properties().name)
    assert_equals(1, g:graph_properties().version)
    assert_true(g:remove_graph_property("version"))
    assert_equals(nil, g:graph_properties().version)

    g:add_node("A", { kind = "input" })
    g:add_node("A", { trainable = false })
    assert_true(g:set_node_property("A", "slot", 0))
    assert_equals("input", g:node_properties("A").kind)
    assert_equals(false, g:node_properties("A").trainable)
    assert_equals(0, g:node_properties("A").slot)
    assert_true(g:remove_node_property("A", "slot"))
    assert_equals(nil, g:node_properties("A").slot)

    g:add_edge("A", "B", 2.5, { role = "distance" })
    assert_equals("distance", g:edge_properties("B", "A").role)
    assert_equals(2.5, g:edge_properties("B", "A").weight)
    assert_true(g:set_edge_property("B", "A", "weight", 7.0))
    assert_equals(7.0, assert(g:edge_weight("A", "B")))
    assert_true(g:set_edge_property("A", "B", "trainable", true))
    assert_true(g:remove_edge_property("A", "B", "role"))
    assert_equals(true, g:edge_properties("A", "B").trainable)
    assert_equals(nil, g:edge_properties("A", "B").role)

    assert_true(g:remove_edge("A", "B"))
    local properties, err = g:edge_properties("A", "B")
    assert_equals(nil, properties)
    assert_equals("edge_not_found", err.type)
end

print("lua/graph tests passed")
