"""
visualization.py -- Graph Visualization in DOT, Mermaid, and ASCII Formats
============================================================================

This module provides three complementary visualization functions for directed
graphs:

1. **to_dot()** -- Generates Graphviz DOT format, the gold standard for graph
   visualization. DOT files can be rendered to PNG, SVG, or PDF using the
   ``dot`` command-line tool (part of the Graphviz suite).

2. **to_mermaid()** -- Generates Mermaid diagram syntax, which renders directly
   in GitHub Markdown, Notion, and many documentation platforms. No external
   tools needed -- just paste into a ```mermaid code block.

3. **to_ascii_table()** -- Generates a plain-text representation that works
   everywhere: terminals, log files, error messages, docstrings. For labeled
   graphs, it produces a transition table (rows = nodes, columns = labels).
   For unlabeled graphs, it produces an adjacency list.

=== Why Three Formats? ===

Different contexts demand different formats:

- **Debugging in a terminal?** Use ``to_ascii_table()`` -- no tools needed.
- **Writing documentation?** Use ``to_mermaid()`` -- renders in Markdown.
- **Producing publication-quality diagrams?** Use ``to_dot()`` -- maximum control.

=== How It Works with Both Graph Types ===

Both ``DirectedGraph`` and ``LabeledDirectedGraph`` are supported. We use
``isinstance`` checks to detect the graph type and adjust behavior:

- **DirectedGraph**: Edges have no labels. DOT edges are plain arrows.
  Mermaid edges use ``A --> B``. ASCII shows adjacency lists.

- **LabeledDirectedGraph**: Edges carry string labels. DOT edges get
  ``[label="..."]`` attributes. Mermaid edges use ``A -->|label| B``.
  ASCII shows a transition table with labels as columns.

When a labeled graph has multiple labels between the same pair of nodes
(e.g., A->B with labels "x" and "y"), we combine them: ``"x, y"``.

=== Connection to State Machines ===

The DFA and NFA classes in the state-machine package have their own ``to_dot()``
methods that handle state-machine-specific conventions (accepting states as
double circles, initial state arrows). This module provides the *general*
graph visualization that those methods are built on top of. If you need to
visualize a graph that happens to represent a state machine, use the
state-machine's own methods. If you need to visualize a general dependency
graph, build graph, or knowledge graph, use these functions.
"""

from __future__ import annotations

from directed_graph.graph import DirectedGraph
from directed_graph.labeled_graph import LabeledDirectedGraph


# ---------------------------------------------------------------------------
# Helper: Escape strings for DOT format
# ---------------------------------------------------------------------------
# DOT uses double-quoted strings for labels and node names. We need to escape
# characters that have special meaning inside those quotes.
#
# The characters we escape:
#   "  -> \"    (terminates the string)
#   \  -> \\   (escape character itself)
#   <  -> \<   (HTML label start)
#   >  -> \>   (HTML label end)
#   {  -> \{   (record label field separator)
#   }  -> \}   (record label field separator)
#   |  -> \|   (record label field separator)


def _escape_dot(text: str) -> str:
    """Escape special characters for DOT label strings.

    DOT uses a subset of C-style escaping inside double-quoted strings.
    We handle the characters most likely to appear in node names and
    edge labels.

    Example:
        >>> _escape_dot('say "hello"')
        'say \\\\"hello\\\\"'
    """
    text = text.replace("\\", "\\\\")
    text = text.replace('"', '\\"')
    text = text.replace("<", "\\<")
    text = text.replace(">", "\\>")
    text = text.replace("{", "\\{")
    text = text.replace("}", "\\}")
    text = text.replace("|", "\\|")
    return text


# ---------------------------------------------------------------------------
# Helper: Escape strings for Mermaid format
# ---------------------------------------------------------------------------
# Mermaid uses its own syntax where certain characters in node labels or
# edge labels can break parsing. We wrap labels in quotes when needed.


def _escape_mermaid(text: str) -> str:
    """Escape special characters for Mermaid labels.

    Mermaid has issues with characters like quotes, pipes, brackets, and
    parentheses in labels. We replace double quotes with single quotes
    since Mermaid labels are typically quoted with double quotes.

    Example:
        >>> _escape_mermaid('hello "world"')
        "hello 'world'"
    """
    return text.replace('"', "'")


# ---------------------------------------------------------------------------
# Helper: Collect edges grouped by (source, target) for label combining
# ---------------------------------------------------------------------------
# When a labeled graph has multiple labels on the same edge pair, we want
# to combine them into a single label string like "a, b" rather than
# drawing separate edges.


def _collect_labeled_edges(
    graph: LabeledDirectedGraph,
) -> dict[tuple[object, object], list[str]]:
    """Collect all edge labels grouped by (source, target) pair.

    For a labeled graph with edges:
        A -> B [label="x"]
        A -> B [label="y"]
        A -> C [label="z"]

    Returns:
        {("A", "B"): ["x", "y"], ("A", "C"): ["z"]}

    Labels within each group are sorted for deterministic output.
    """
    edge_labels: dict[tuple[object, object], list[str]] = {}
    for from_node, to_node, label in graph.edges():
        key = (from_node, to_node)
        if key not in edge_labels:
            edge_labels[key] = []
        edge_labels[key].append(label)
    # Sort labels within each group for determinism
    for key in edge_labels:
        edge_labels[key] = sorted(edge_labels[key])
    return edge_labels


def _collect_unlabeled_edges(
    graph: DirectedGraph,
) -> list[tuple[object, object]]:
    """Collect all edges from an unlabeled graph, sorted for deterministic output.

    Returns a list of (source, target) tuples sorted by (str(source), str(target)).
    """
    edges = graph.edges()
    return sorted(edges, key=lambda e: (str(e[0]), str(e[1])))


# ===========================================================================
# to_dot() -- Graphviz DOT format
# ===========================================================================
#
# DOT is a plain-text graph description language. Here's what a simple
# graph looks like:
#
#     digraph G {
#         rankdir=LR;
#         "A" -> "B";
#         "B" -> "C";
#     }
#
# Render it with: dot -Tpng graph.dot -o graph.png
#
# The key elements we generate:
# - `digraph <name> { ... }` -- declares a directed graph
# - `rankdir=LR` -- left-to-right layout (good for pipelines and FSMs)
# - Node declarations with optional attributes: `"A" [shape=doublecircle];`
# - Edge declarations with optional labels: `"A" -> "B" [label="x"];`
# - An invisible start node for marking the initial state (FSM convention)


def to_dot(
    graph: DirectedGraph | LabeledDirectedGraph,
    *,
    name: str = "G",
    node_attrs: dict[str, dict[str, str]] | None = None,
    edge_attrs: dict[tuple[str, str], dict[str, str]] | None = None,
    initial: str | None = None,
    rankdir: str = "LR",
) -> str:
    """Generate a Graphviz DOT representation of the graph.

    Works with both ``DirectedGraph`` and ``LabeledDirectedGraph``. For
    labeled graphs, edges automatically get ``[label="..."]`` attributes.
    Multiple labels between the same pair of nodes are combined with
    commas: ``"a, b"``.

    Args:
        graph: The graph to visualize. Can be either type.
        name: The name of the digraph (appears in the DOT header).
            Default: ``"G"``.
        node_attrs: Optional per-node attributes. A dict mapping node names
            to dicts of DOT attributes. For example::

                {"q1": {"shape": "doublecircle", "color": "red"}}

            produces: ``"q1" [shape=doublecircle, color=red];``
        edge_attrs: Optional per-edge attributes. A dict mapping
            ``(source, target)`` tuples to dicts of DOT attributes.
            These are merged with any auto-generated label attributes.
        initial: If set, adds an invisible start node with an arrow
            pointing to this node. This is the standard convention for
            marking the initial state in automata diagrams.
        rankdir: Layout direction. ``"LR"`` for left-to-right (default),
            ``"TB"`` for top-to-bottom.

    Returns:
        A valid DOT format string that can be saved to a .dot file
        and rendered with the Graphviz ``dot`` command.

    Example:
        >>> from directed_graph import DirectedGraph
        >>> g = DirectedGraph()
        >>> g.add_edge("A", "B")
        >>> g.add_edge("B", "C")
        >>> print(to_dot(g))
        digraph G {
            rankdir=LR;
        <BLANKLINE>
            "A";
            "B";
            "C";
        <BLANKLINE>
            "A" -> "B";
            "B" -> "C";
        }
    """
    if node_attrs is None:
        node_attrs = {}
    if edge_attrs is None:
        edge_attrs = {}

    lines: list[str] = []

    # --- Header ---
    lines.append(f"digraph {_escape_dot(name)} {{")
    lines.append(f"    rankdir={rankdir};")
    lines.append("")

    # --- Initial state arrow ---
    # This is the standard FSM convention: an invisible "point" node with
    # an arrow to the initial state. The point node has zero width so it
    # appears as just an arrowhead.
    if initial is not None:
        lines.append("    __start [shape=point, width=0.2];")
        lines.append(f'    __start -> "{_escape_dot(str(initial))}";')
        lines.append("")

    # --- Node declarations ---
    # We sort nodes for deterministic output. Each node gets its own line
    # with optional attributes.
    sorted_nodes = sorted(graph.nodes(), key=str)
    for node in sorted_nodes:
        node_str = _escape_dot(str(node))
        attrs = node_attrs.get(str(node), {})
        if attrs:
            attr_str = ", ".join(
                f"{k}={v}" for k, v in sorted(attrs.items())
            )
            lines.append(f'    "{node_str}" [{attr_str}];')
        else:
            lines.append(f'    "{node_str}";')
    lines.append("")

    # --- Edge declarations ---
    is_labeled = isinstance(graph, LabeledDirectedGraph)

    if is_labeled:
        # For labeled graphs, collect edges grouped by (source, target)
        # and combine multiple labels into one string.
        grouped = _collect_labeled_edges(graph)
        for (from_node, to_node), labels in sorted(
            grouped.items(), key=lambda x: (str(x[0][0]), str(x[0][1]))
        ):
            from_str = _escape_dot(str(from_node))
            to_str = _escape_dot(str(to_node))
            combined_label = ", ".join(labels)

            # Start with the label attribute, then merge any user-specified
            # edge attributes.
            all_attrs: dict[str, str] = {"label": f'"{_escape_dot(combined_label)}"'}
            user_attrs = edge_attrs.get((str(from_node), str(to_node)), {})
            for k, v in user_attrs.items():
                all_attrs[k] = v

            attr_str = ", ".join(
                f"{k}={v}" for k, v in sorted(all_attrs.items())
            )
            lines.append(f'    "{from_str}" -> "{to_str}" [{attr_str}];')
    else:
        # For unlabeled graphs, edges are plain arrows.
        for from_node, to_node in _collect_unlabeled_edges(graph):
            from_str = _escape_dot(str(from_node))
            to_str = _escape_dot(str(to_node))

            user_attrs = edge_attrs.get((str(from_node), str(to_node)), {})
            if user_attrs:
                attr_str = ", ".join(
                    f"{k}={v}" for k, v in sorted(user_attrs.items())
                )
                lines.append(
                    f'    "{from_str}" -> "{to_str}" [{attr_str}];'
                )
            else:
                lines.append(f'    "{from_str}" -> "{to_str}";')

    lines.append("}")
    return "\n".join(lines)


# ===========================================================================
# to_mermaid() -- Mermaid diagram syntax
# ===========================================================================
#
# Mermaid is a JavaScript-based diagramming tool that renders directly in
# Markdown. GitHub, GitLab, Notion, and many other platforms support it
# natively.
#
# A simple Mermaid flowchart looks like:
#
#     graph LR
#         A --> B
#         B --> C
#
# For labeled edges:
#
#     graph LR
#         A -->|"coin"| B
#         B -->|"push"| A
#
# We use the `graph` keyword (not `flowchart`) for maximum compatibility.


def to_mermaid(
    graph: DirectedGraph | LabeledDirectedGraph,
    *,
    direction: str = "LR",
    initial: str | None = None,
) -> str:
    """Generate a Mermaid flowchart diagram of the graph.

    Works with both ``DirectedGraph`` and ``LabeledDirectedGraph``. For
    labeled graphs, edges get ``-->|"label"|`` syntax. Multiple labels
    between the same pair are combined: ``-->|"a, b"|``.

    Args:
        graph: The graph to visualize.
        direction: Layout direction. ``"LR"`` for left-to-right (default),
            ``"TD"`` for top-down.
        initial: If set, adds an invisible start node (using Mermaid's
            ``(( ))`` circle syntax) with an arrow to this node.

    Returns:
        A Mermaid diagram string suitable for embedding in Markdown.

    Example:
        >>> from directed_graph import DirectedGraph
        >>> g = DirectedGraph()
        >>> g.add_edge("A", "B")
        >>> g.add_edge("B", "C")
        >>> print(to_mermaid(g))
        graph LR
            A --> B
            B --> C
    """
    lines: list[str] = []

    # --- Header ---
    lines.append(f"graph {direction}")

    # --- Initial state marker ---
    if initial is not None:
        initial_escaped = _escape_mermaid(str(initial))
        lines.append(f"    __start(( )) --> {initial_escaped}")

    # --- Edges ---
    is_labeled = isinstance(graph, LabeledDirectedGraph)

    if is_labeled:
        grouped = _collect_labeled_edges(graph)
        for (from_node, to_node), labels in sorted(
            grouped.items(), key=lambda x: (str(x[0][0]), str(x[0][1]))
        ):
            from_str = _escape_mermaid(str(from_node))
            to_str = _escape_mermaid(str(to_node))
            combined_label = ", ".join(labels)
            escaped_label = _escape_mermaid(combined_label)
            lines.append(
                f'    {from_str} -->|"{escaped_label}"| {to_str}'
            )
    else:
        for from_node, to_node in _collect_unlabeled_edges(graph):
            from_str = _escape_mermaid(str(from_node))
            to_str = _escape_mermaid(str(to_node))
            lines.append(f"    {from_str} --> {to_str}")

    # --- Isolated nodes ---
    # Nodes with no edges still need to appear in the diagram.
    # We emit them as standalone node declarations.
    nodes_in_edges: set[str] = set()
    if is_labeled:
        for from_node, to_node, _label in graph.edges():
            nodes_in_edges.add(str(from_node))
            nodes_in_edges.add(str(to_node))
    else:
        for from_node, to_node in graph.edges():
            nodes_in_edges.add(str(from_node))
            nodes_in_edges.add(str(to_node))

    for node in sorted(graph.nodes(), key=str):
        if str(node) not in nodes_in_edges:
            lines.append(f"    {_escape_mermaid(str(node))}")

    return "\n".join(lines)


# ===========================================================================
# to_ascii_table() -- Plain-text visualization
# ===========================================================================
#
# For labeled graphs, we produce a transition table like this:
#
#     State    | coin     | push
#     ---------+----------+---------
#     locked   | unlocked | locked
#     unlocked | unlocked | locked
#
# This is the same format used by textbooks to represent DFA transition
# functions. Each row is a state, each column is an input symbol (label),
# and each cell is the target state(s).
#
# For unlabeled graphs, we produce an adjacency list:
#
#     Node | Successors
#     -----+-----------
#     A    | B, C
#     B    | C
#     C    | (none)
#
# This format is compact and easy to scan for debugging.


def to_ascii_table(graph: DirectedGraph | LabeledDirectedGraph) -> str:
    """Generate a plain-text table representation of the graph.

    For ``LabeledDirectedGraph``: produces a transition table where rows
    are nodes, columns are unique labels (sorted), and cells show target
    nodes. Multiple targets for the same (node, label) are comma-separated.

    For ``DirectedGraph``: produces an adjacency list where each row shows
    a node and its successors.

    Args:
        graph: The graph to visualize.

    Returns:
        A formatted ASCII table string with aligned columns.

    Example (labeled):
        >>> from directed_graph import LabeledDirectedGraph
        >>> lg = LabeledDirectedGraph()
        >>> lg.add_edge("locked", "unlocked", "coin")
        >>> lg.add_edge("locked", "locked", "push")
        >>> lg.add_edge("unlocked", "unlocked", "coin")
        >>> lg.add_edge("unlocked", "locked", "push")
        >>> print(to_ascii_table(lg))  # doctest: +SKIP
        State    | coin     | push
        ---------+----------+---------
        locked   | unlocked | locked
        unlocked | unlocked | locked

    Example (unlabeled):
        >>> from directed_graph import DirectedGraph
        >>> g = DirectedGraph()
        >>> g.add_edge("A", "B")
        >>> g.add_edge("A", "C")
        >>> g.add_edge("B", "C")
        >>> print(to_ascii_table(g))  # doctest: +SKIP
        Node | Successors
        -----+-----------
        A    | B, C
        B    | C
        C    | (none)
    """
    if isinstance(graph, LabeledDirectedGraph):
        return _ascii_table_labeled(graph)
    else:
        return _ascii_table_unlabeled(graph)


def _ascii_table_labeled(graph: LabeledDirectedGraph) -> str:
    """Build a transition table for a labeled graph.

    The table has one row per node (sorted) and one column per unique label
    (sorted). Each cell contains the target node(s) for that (node, label)
    combination, or a dash if no transition exists.

    This is the standard way textbooks display DFA/NFA transition functions:

        State  | a     | b
        -------+-------+------
        q0     | q1    | q0
        q1     | q1    | q2
    """
    sorted_nodes = sorted(graph.nodes(), key=str)

    # Collect all unique labels across all edges, sorted.
    all_labels: set[str] = set()
    for _from, _to, label in graph.edges():
        all_labels.add(label)
    sorted_labels = sorted(all_labels)

    # Handle empty graph or graph with no labels
    if not sorted_nodes:
        return "(empty graph)"
    if not sorted_labels:
        # Graph has nodes but no edges -- just list the nodes
        return _ascii_table_unlabeled_from_nodes(sorted_nodes)

    # Build the transition map: (node, label) -> sorted list of targets
    transition_map: dict[tuple[str, str], list[str]] = {}
    for from_node, to_node, label in graph.edges():
        key = (str(from_node), label)
        if key not in transition_map:
            transition_map[key] = []
        transition_map[key].append(str(to_node))
    for key in transition_map:
        transition_map[key] = sorted(transition_map[key])

    # Calculate column widths
    #   - First column: "State" header or longest node name
    #   - Label columns: label name or longest cell content
    state_col_width = max(len("State"), max(len(str(n)) for n in sorted_nodes))

    label_col_widths: list[int] = []
    for label in sorted_labels:
        max_cell = len(label)
        for node in sorted_nodes:
            key = (str(node), label)
            if key in transition_map:
                cell = ", ".join(transition_map[key])
            else:
                cell = "-"
            max_cell = max(max_cell, len(cell))
        label_col_widths.append(max_cell)

    # Build the header row
    header_parts = [f"{'State':<{state_col_width}}"]
    for i, label in enumerate(sorted_labels):
        header_parts.append(f" {label:<{label_col_widths[i]}}")
    header = " |".join(header_parts)

    # Build the separator
    sep_parts = ["-" * state_col_width]
    for width in label_col_widths:
        sep_parts.append("-" * (width + 1))
    separator = "-+".join(sep_parts)

    # Build data rows
    rows: list[str] = [header, separator]
    for node in sorted_nodes:
        row_parts = [f"{str(node):<{state_col_width}}"]
        for i, label in enumerate(sorted_labels):
            key = (str(node), label)
            if key in transition_map:
                cell = ", ".join(transition_map[key])
            else:
                cell = "-"
            row_parts.append(f" {cell:<{label_col_widths[i]}}")
        rows.append(" |".join(row_parts))

    return "\n".join(rows)


def _ascii_table_unlabeled(graph: DirectedGraph) -> str:
    """Build an adjacency list table for an unlabeled graph.

    Each row shows a node and its sorted successors:

        Node | Successors
        -----+-----------
        A    | B, C
        B    | C
        C    | (none)
    """
    sorted_nodes = sorted(graph.nodes(), key=str)

    if not sorted_nodes:
        return "(empty graph)"

    # Build successor lists
    successor_strs: dict[str, str] = {}
    for node in sorted_nodes:
        succs = sorted(graph.successors(node), key=str)
        if succs:
            successor_strs[str(node)] = ", ".join(str(s) for s in succs)
        else:
            successor_strs[str(node)] = "(none)"

    # Calculate column widths
    node_col_width = max(len("Node"), max(len(str(n)) for n in sorted_nodes))
    succ_col_width = max(
        len("Successors"),
        max(len(v) for v in successor_strs.values()),
    )

    # Build the table
    header = f"{'Node':<{node_col_width}} | {'Successors':<{succ_col_width}}"
    separator = f"{'-' * node_col_width}-+-{'-' * succ_col_width}"

    rows: list[str] = [header, separator]
    for node in sorted_nodes:
        rows.append(
            f"{str(node):<{node_col_width}} | "
            f"{successor_strs[str(node)]:<{succ_col_width}}"
        )

    return "\n".join(rows)


def _ascii_table_unlabeled_from_nodes(sorted_nodes: list) -> str:
    """Build a simple node listing when there are no edges.

    Used when a labeled graph has nodes but no labeled edges.
    """
    node_col_width = max(len("Node"), max(len(str(n)) for n in sorted_nodes))

    header = f"{'Node':<{node_col_width}} | Successors"
    separator = f"{'-' * node_col_width}-+-{'-' * len('Successors')}"

    rows: list[str] = [header, separator]
    for node in sorted_nodes:
        rows.append(f"{str(node):<{node_col_width}} | (none)")

    return "\n".join(rows)
