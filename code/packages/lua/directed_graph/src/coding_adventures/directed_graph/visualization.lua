-- visualization — Graph Visualization in Multiple Formats
-- ========================================================
--
-- This module converts directed graphs into human-readable text formats.
-- It supports three output formats, each serving a different purpose:
--
-- 1. DOT format (Graphviz) — the industry standard for graph visualization.
--    Paste the output into https://dreampuf.github.io/GraphvizOnline/
--    or pipe it to `dot -Tpng` to get a rendered image.
--
-- 2. Mermaid format — a lightweight alternative that renders directly
--    in GitHub Markdown, Notion, and many other tools.
--
-- 3. ASCII table — a plain-text representation for terminal output.
--    For labeled graphs, this produces a transition table (like an FSM
--    state table). For unlabeled graphs, it produces an adjacency list.
--
-- # Why three formats?
--
-- Each format has a sweet spot:
--
--   - DOT is the most powerful: supports node shapes, colors, subgraphs.
--   - Mermaid is the most convenient: renders inline in documentation.
--   - ASCII tables are the most portable: work everywhere.
--
-- # Function naming
--
-- We use separate functions for DirectedGraph and LabeledGraph:
--
--   - to_dot / labeled_to_dot
--   - to_mermaid / labeled_to_mermaid
--   - to_ascii_table / labeled_to_ascii_table
--
-- This mirrors the Go implementation's explicit naming style.

local visualization = {}

-- =========================================================================
-- DotOptions — controls DOT output rendering
-- =========================================================================
--
-- Fields:
--   name     (string)  Graph name, appears as `digraph <name> { ... }`.
--                       Defaults to "G".
--   rankdir  (string)  Layout direction: "LR" (left-to-right) or
--                       "TB" (top-to-bottom). Defaults to "LR".
--   initial  (string)  If non-empty, adds an invisible start node with
--                       an arrow to this node. Standard FSM diagram style.
--   node_attrs (table) Per-node DOT attributes. Keyed by node name,
--                       values are tables of {attr_name = attr_value}.

--- Apply defaults to a DotOptions table.
-- Returns a new table with all fields filled in.
--
-- @param opts table|nil User-provided options.
-- @return table Options with defaults applied.
local function default_dot_options(opts)
    local result = {
        name = "G",
        rankdir = "LR",
        node_attrs = nil,
        initial = "",
    }
    if opts then
        if opts.name and opts.name ~= "" then
            result.name = opts.name
        end
        if opts.rankdir and opts.rankdir ~= "" then
            result.rankdir = opts.rankdir
        end
        if opts.node_attrs then
            result.node_attrs = opts.node_attrs
        end
        if opts.initial then
            result.initial = opts.initial
        end
    end
    return result
end

--- Format a table of DOT attributes into the bracketed format.
--
-- Example: {shape = "circle", color = "red"} -> `[color=red, shape=circle]`
--
-- Attributes are sorted by key for deterministic output.
--
-- @param attrs table A table of {attr_name = attr_value}.
-- @return string The formatted attribute string.
local function format_dot_attrs(attrs)
    local keys = {}
    for k, _ in pairs(attrs) do
        keys[#keys + 1] = k
    end
    table.sort(keys)

    local parts = {}
    for _, k in ipairs(keys) do
        parts[#parts + 1] = string.format("%s=%s", k, attrs[k])
    end
    return "[" .. table.concat(parts, ", ") .. "]"
end

-- =========================================================================
-- to_dot — Graphviz DOT format for unlabeled graphs
-- =========================================================================
--
-- The DOT language is the standard input format for Graphviz. A DOT file
-- describes a graph using a simple text syntax:
--
--   digraph G {
--       A -> B;
--       B -> C;
--   }
--
-- Nodes are declared explicitly so that isolated nodes (with no edges)
-- still appear in the output. Edges are listed after nodes.

--- Convert an unlabeled DirectedGraph to Graphviz DOT format.
--
-- @param g DirectedGraph The graph to convert.
-- @param opts table|nil Optional DotOptions.
-- @return string The DOT format string.
function visualization.to_dot(g, opts)
    local o = default_dot_options(opts)
    local lines = {}

    lines[#lines + 1] = string.format("digraph %s {", o.name)
    lines[#lines + 1] = string.format("    rankdir=%s;", o.rankdir)

    -- Initial state marker: invisible node with arrow.
    if o.initial ~= "" then
        lines[#lines + 1] = '    "" [shape=none];'
        lines[#lines + 1] = string.format('    "" -> %s;', o.initial)
    end

    -- Node declarations (sorted for deterministic output).
    local nodes = g:nodes()
    for _, node in ipairs(nodes) do
        if o.node_attrs and o.node_attrs[node] then
            lines[#lines + 1] = string.format("    %s %s;", node, format_dot_attrs(o.node_attrs[node]))
        else
            lines[#lines + 1] = string.format("    %s;", node)
        end
    end

    -- Edge declarations (sorted for deterministic output).
    local edges = g:edges()
    for _, edge in ipairs(edges) do
        lines[#lines + 1] = string.format("    %s -> %s;", edge[1], edge[2])
    end

    lines[#lines + 1] = "}"
    return table.concat(lines, "\n")
end

-- =========================================================================
-- labeled_to_dot — Graphviz DOT format for labeled graphs
-- =========================================================================
--
-- For labeled graphs, edges get [label="..."] attributes. If multiple
-- labels exist on the same (from, to) pair, they are combined as
-- "a, b" in a single label attribute.

--- Collect edge labels grouped by (from, to) pair.
-- Returns a table keyed by "from\0to" with combined label strings.
local function collect_labeled_edge_labels(lg)
    local edges = lg:edges()  -- array of {from, to, label}
    local grouped = {}  -- "from\0to" -> array of labels

    for _, edge in ipairs(edges) do
        local key = edge[1] .. "\0" .. edge[2]
        if not grouped[key] then
            grouped[key] = {}
        end
        local labels = grouped[key]
        labels[#labels + 1] = edge[3]
    end

    -- Sort and join labels.
    local result = {}
    for key, labels in pairs(grouped) do
        table.sort(labels)
        result[key] = table.concat(labels, ", ")
    end
    return result
end

--- Extract unique structural (from, to) edges from a labeled graph, sorted.
local function unique_structural_edges(lg)
    local seen = {}
    local result = {}
    local edges = lg:edges()

    for _, edge in ipairs(edges) do
        local key = edge[1] .. "\0" .. edge[2]
        if not seen[key] then
            seen[key] = true
            result[#result + 1] = {edge[1], edge[2]}
        end
    end

    table.sort(result, function(a, b)
        if a[1] ~= b[1] then return a[1] < b[1] end
        return a[2] < b[2]
    end)
    return result
end

--- Convert a LabeledGraph to Graphviz DOT format.
--
-- @param lg LabeledGraph The labeled graph to convert.
-- @param opts table|nil Optional DotOptions.
-- @return string The DOT format string.
function visualization.labeled_to_dot(lg, opts)
    local o = default_dot_options(opts)
    local lines = {}

    lines[#lines + 1] = string.format("digraph %s {", o.name)
    lines[#lines + 1] = string.format("    rankdir=%s;", o.rankdir)

    -- Initial state marker.
    if o.initial ~= "" then
        lines[#lines + 1] = '    "" [shape=none];'
        lines[#lines + 1] = string.format('    "" -> %s;', o.initial)
    end

    -- Node declarations.
    local nodes = lg:nodes()
    for _, node in ipairs(nodes) do
        if o.node_attrs and o.node_attrs[node] then
            lines[#lines + 1] = string.format("    %s %s;", node, format_dot_attrs(o.node_attrs[node]))
        else
            lines[#lines + 1] = string.format("    %s;", node)
        end
    end

    -- Build combined edge labels.
    local edge_labels = collect_labeled_edge_labels(lg)
    local structural_edges = unique_structural_edges(lg)

    for _, edge in ipairs(structural_edges) do
        local key = edge[1] .. "\0" .. edge[2]
        local label = edge_labels[key]
        if label then
            lines[#lines + 1] = string.format('    %s -> %s [label="%s"];', edge[1], edge[2], label)
        else
            lines[#lines + 1] = string.format("    %s -> %s;", edge[1], edge[2])
        end
    end

    lines[#lines + 1] = "}"
    return table.concat(lines, "\n")
end

-- =========================================================================
-- to_mermaid — Mermaid flowchart format for unlabeled graphs
-- =========================================================================
--
-- Mermaid is a JavaScript-based diagramming tool that renders directly
-- in Markdown. The syntax for a flowchart is:
--
--   graph LR
--       A --> B
--       B --> C

--- Convert an unlabeled DirectedGraph to Mermaid flowchart format.
--
-- @param g DirectedGraph The graph to convert.
-- @param direction string|nil Flow direction: "LR" or "TD". Defaults to "LR".
-- @return string The Mermaid format string.
function visualization.to_mermaid(g, direction)
    if not direction or direction == "" then
        direction = "LR"
    end

    local lines = {}
    lines[#lines + 1] = string.format("graph %s", direction)

    local edges = g:edges()
    for _, edge in ipairs(edges) do
        lines[#lines + 1] = string.format("    %s --> %s", edge[1], edge[2])
    end

    return table.concat(lines, "\n")
end

-- =========================================================================
-- labeled_to_mermaid — Mermaid flowchart format for labeled graphs
-- =========================================================================
--
-- For labeled edges, Mermaid uses the -->|label| syntax:
--
--   A -->|coin| B
--
-- If multiple labels exist on the same edge, we combine them:
--
--   A -->|coin, push| B

--- Convert a LabeledGraph to Mermaid flowchart format.
--
-- @param lg LabeledGraph The labeled graph to convert.
-- @param direction string|nil Flow direction: "LR" or "TD". Defaults to "LR".
-- @return string The Mermaid format string.
function visualization.labeled_to_mermaid(lg, direction)
    if not direction or direction == "" then
        direction = "LR"
    end

    local edge_labels = collect_labeled_edge_labels(lg)
    local structural_edges = unique_structural_edges(lg)

    local lines = {}
    lines[#lines + 1] = string.format("graph %s", direction)

    for _, edge in ipairs(structural_edges) do
        local key = edge[1] .. "\0" .. edge[2]
        local label = edge_labels[key]
        if label then
            lines[#lines + 1] = string.format("    %s -->|%s| %s", edge[1], label, edge[2])
        else
            lines[#lines + 1] = string.format("    %s --> %s", edge[1], edge[2])
        end
    end

    return table.concat(lines, "\n")
end

-- =========================================================================
-- to_ascii_table — Plain text adjacency list for unlabeled graphs
-- =========================================================================
--
-- For unlabeled graphs, we produce a two-column table:
--
--   Node    | Successors
--   --------+-----------
--   A       | B, C
--   B       | D
--   C       | D
--   D       | -
--
-- The dash "-" indicates no successors. This is simple and readable.

--- Convert an unlabeled DirectedGraph to a plain-text adjacency table.
--
-- @param g DirectedGraph The graph to convert.
-- @return string The ASCII table string.
function visualization.to_ascii_table(g)
    local nodes = g:nodes()

    -- Build successor strings for each node.
    local succ_strings = {}
    for _, node in ipairs(nodes) do
        local succs, _ = g:successors(node)
        table.sort(succs)
        if #succs > 0 then
            succ_strings[node] = table.concat(succs, ", ")
        else
            succ_strings[node] = "-"
        end
    end

    -- Calculate column widths.
    local node_col_width = #"Node"
    for _, node in ipairs(nodes) do
        if #node > node_col_width then
            node_col_width = #node
        end
    end

    local succ_col_width = #"Successors"
    for _, s in pairs(succ_strings) do
        if #s > succ_col_width then
            succ_col_width = #s
        end
    end

    -- Build the table.
    local lines = {}

    -- Header.
    lines[#lines + 1] = string.format("%-" .. node_col_width .. "s | %-" .. succ_col_width .. "s",
        "Node", "Successors")
    -- Separator.
    lines[#lines + 1] = string.rep("-", node_col_width) .. "-+-" .. string.rep("-", succ_col_width)
    -- Data rows.
    for _, node in ipairs(nodes) do
        lines[#lines + 1] = string.format("%-" .. node_col_width .. "s | %-" .. succ_col_width .. "s",
            node, succ_strings[node])
    end

    return table.concat(lines, "\n")
end

-- =========================================================================
-- labeled_to_ascii_table — Transition table for labeled graphs
-- =========================================================================
--
-- For labeled graphs, we produce a transition table where rows are nodes
-- (states), columns are unique labels (input symbols), and cells are
-- destination nodes (next states):
--
--   State      | coin      | push
--   -----------+-----------+----------
--   locked     | unlocked  | locked
--   unlocked   | unlocked  | locked
--
-- This is the standard representation of a finite state machine.
-- A "-" in a cell means no transition exists for that (state, label) pair.

--- Convert a LabeledGraph to a plain-text transition table.
--
-- @param lg LabeledGraph The labeled graph to convert.
-- @return string The ASCII transition table string.
function visualization.labeled_to_ascii_table(lg)
    local nodes = lg:nodes()
    local edges = lg:edges()  -- array of {from, to, label}

    -- Step 1: Collect all unique labels.
    local label_set = {}
    for _, edge in ipairs(edges) do
        label_set[edge[3]] = true
    end
    local labels = {}
    for l, _ in pairs(label_set) do
        labels[#labels + 1] = l
    end
    table.sort(labels)

    -- Handle edge case: no labels.
    if #labels == 0 then
        local state_col_width = #"State"
        for _, node in ipairs(nodes) do
            if #node > state_col_width then
                state_col_width = #node
            end
        end
        local lines = {}
        lines[#lines + 1] = string.format("%-" .. state_col_width .. "s", "State")
        lines[#lines + 1] = string.rep("-", state_col_width)
        for _, node in ipairs(nodes) do
            lines[#lines + 1] = string.format("%-" .. state_col_width .. "s", node)
        end
        return table.concat(lines, "\n")
    end

    -- Step 2: Build transition map.
    -- transitions[node .. "\0" .. label] = sorted list of destination nodes.
    local transitions = {}
    for _, edge in ipairs(edges) do
        local key = edge[1] .. "\0" .. edge[3]
        if not transitions[key] then
            transitions[key] = {}
        end
        local dests = transitions[key]
        -- Only add if not already present.
        local found = false
        for _, d in ipairs(dests) do
            if d == edge[2] then
                found = true
                break
            end
        end
        if not found then
            dests[#dests + 1] = edge[2]
            table.sort(dests)
        end
    end

    -- Step 3: Calculate column widths.
    local state_col_width = #"State"
    for _, node in ipairs(nodes) do
        if #node > state_col_width then
            state_col_width = #node
        end
    end

    local label_col_widths = {}
    for i, label in ipairs(labels) do
        label_col_widths[i] = #label
        for _, node in ipairs(nodes) do
            local key = node .. "\0" .. label
            local dests = transitions[key]
            local cell_text = "-"
            if dests and #dests > 0 then
                cell_text = table.concat(dests, ", ")
            end
            if #cell_text > label_col_widths[i] then
                label_col_widths[i] = #cell_text
            end
        end
    end

    -- Step 4: Build the formatted table.
    local lines = {}

    -- Header row.
    local header = string.format("%-" .. state_col_width .. "s", "State")
    for i, label in ipairs(labels) do
        header = header .. string.format(" | %-" .. label_col_widths[i] .. "s", label)
    end
    lines[#lines + 1] = header

    -- Separator row.
    local sep = string.rep("-", state_col_width)
    for i, _ in ipairs(labels) do
        sep = sep .. "-+-" .. string.rep("-", label_col_widths[i])
    end
    lines[#lines + 1] = sep

    -- Data rows.
    for _, node in ipairs(nodes) do
        local row = string.format("%-" .. state_col_width .. "s", node)
        for i, label in ipairs(labels) do
            local key = node .. "\0" .. label
            local dests = transitions[key]
            local cell_text = "-"
            if dests and #dests > 0 then
                cell_text = table.concat(dests, ", ")
            end
            row = row .. string.format(" | %-" .. label_col_widths[i] .. "s", cell_text)
        end
        lines[#lines + 1] = row
    end

    return table.concat(lines, "\n")
end

return visualization
