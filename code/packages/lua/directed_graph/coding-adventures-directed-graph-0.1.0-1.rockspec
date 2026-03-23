package = "coding-adventures-directed-graph"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Directed graph with topological sort, cycle detection, and reachability",
    detailed = [[
        A directed graph library with algorithms for topological sorting,
        cycle detection, transitive closure, independent group computation,
        and affected-node analysis. Includes both unlabeled and labeled
        graph variants, plus DOT, Mermaid, and ASCII table visualization.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.directed_graph"] = "src/coding_adventures/directed_graph/init.lua",
        ["coding_adventures.directed_graph.labeled_graph"] = "src/coding_adventures/directed_graph/labeled_graph.lua",
        ["coding_adventures.directed_graph.visualization"] = "src/coding_adventures/directed_graph/visualization.lua",
    },
}
