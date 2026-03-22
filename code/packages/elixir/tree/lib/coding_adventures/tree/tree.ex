defmodule CodingAdventures.Tree.Tree do
  @moduledoc """
  A Rooted Tree Backed by a Directed Graph
  ==========================================

  What Is a Tree?
  ---------------

  A **tree** is one of the most fundamental data structures in computer science.
  You encounter trees everywhere:

  - File systems: directories contain files and subdirectories
  - HTML/XML: elements contain child elements
  - Programming languages: Abstract Syntax Trees (ASTs) represent code structure
  - Organization charts: managers have direct reports

  Formally, a tree is a connected, acyclic graph where:

  1. There is exactly **one root** node (a node with no parent).
  2. Every other node has exactly **one parent**.
  3. There are **no cycles** -- you can never follow edges and return to where
     you started.

  These constraints mean a tree with N nodes always has exactly N-1 edges.

  Tree vs. Graph
  ~~~~~~~~~~~~~~

  A tree IS a graph (specifically, a directed acyclic graph with the
  single-parent constraint). We leverage this by building our Tree on top
  of the `CodingAdventures.DirectedGraph.Graph` module. The `Graph` handles
  all the low-level node/edge storage, while this `Tree` module enforces the
  tree invariants and provides tree-specific operations like traversals,
  depth calculation, and lowest common ancestor.

  Edges point from parent to child:

      Program
      ├── Assignment    (edge: Program → Assignment)
      │   ├── Name      (edge: Assignment → Name)
      │   └── BinaryOp  (edge: Assignment → BinaryOp)
      └── Print         (edge: Program → Print)

  Immutability
  ~~~~~~~~~~~~

  Unlike the Python version which mutates in-place, the Elixir version is
  fully immutable. Every operation returns a new tree struct (wrapped in
  `{:ok, tree}` or `{:error, reason}`). This is idiomatic Elixir and plays
  well with concurrent code.

  ## Tree Terminology

  - **Root**: The topmost node. It has no parent. Every tree has exactly one.
  - **Parent**: The node directly above another node.
  - **Child**: A node directly below another node.
  - **Siblings**: Nodes that share the same parent.
  - **Leaf**: A node with no children.
  - **Depth**: The number of edges from the root to a node. The root has depth 0.
  - **Height**: The maximum depth of any node in the tree.
  - **Subtree**: A node together with all its descendants forms a smaller tree.
  - **Path**: The sequence of nodes from the root to a given node.
  - **LCA (Lowest Common Ancestor)**: The deepest node that is an ancestor of
    both node A and node B.
  """

  alias CodingAdventures.DirectedGraph.Graph

  # ---------------------------------------------------------------------------
  # Struct
  # ---------------------------------------------------------------------------
  # The struct holds the root node name and the underlying directed graph.
  # We use @enforce_keys to ensure both fields are always set.

  @enforce_keys [:root, :graph]
  defstruct [:root, :graph]

  @type t :: %__MODULE__{
          root: any(),
          graph: Graph.t()
        }

  # ---------------------------------------------------------------------------
  # Constructor
  # ---------------------------------------------------------------------------

  @doc """
  Create a new tree with the given root node.

  A tree always starts with exactly one node -- the root. You can't have an
  empty tree (that would be a forest, or nothing at all).

  ## Example

      iex> tree = CodingAdventures.Tree.Tree.new("root")
      iex> CodingAdventures.Tree.Tree.root(tree)
      "root"
      iex> CodingAdventures.Tree.Tree.size(tree)
      1
  """
  @spec new(any()) :: t()
  def new(root) do
    graph = Graph.new()
    {:ok, graph} = Graph.add_node(graph, root)
    %__MODULE__{root: root, graph: graph}
  end

  # ---------------------------------------------------------------------------
  # Mutation
  # ---------------------------------------------------------------------------

  @doc """
  Add a child node under the given parent.

  Returns `{:ok, new_tree}` on success. Returns `{:error, reason}` if:
  - The parent doesn't exist in the tree (`:node_not_found`)
  - The child already exists in the tree (`:duplicate_node`)

  Each call adds one new node and one edge (parent → child). In a tree,
  every node has exactly one parent, so a node can only be added once.

  ## Example

      iex> tree = CodingAdventures.Tree.Tree.new("root")
      iex> {:ok, tree} = CodingAdventures.Tree.Tree.add_child(tree, "root", "child")
      iex> CodingAdventures.Tree.Tree.children(tree, "root")
      {:ok, ["child"]}
  """
  @spec add_child(t(), any(), any()) :: {:ok, t()} | {:error, {:node_not_found, any()} | {:duplicate_node, any()}}
  def add_child(%__MODULE__{} = tree, parent, child) do
    cond do
      not Graph.has_node?(tree.graph, parent) ->
        {:error, {:node_not_found, parent}}

      Graph.has_node?(tree.graph, child) ->
        {:error, {:duplicate_node, child}}

      true ->
        {:ok, new_graph} = Graph.add_edge(tree.graph, parent, child)
        {:ok, %{tree | graph: new_graph}}
    end
  end

  @doc """
  Remove a node and all its descendants from the tree.

  Returns `{:ok, new_tree}` on success. Returns `{:error, reason}` if:
  - The node doesn't exist (`:node_not_found`)
  - The node is the root (`:root_removal`)

  This is a "prune" operation -- it cuts off an entire branch. We collect
  all descendants via BFS, then remove them from bottom up (children before
  parents) to keep the graph consistent at each step.

  ## Example

      iex> tree = CodingAdventures.Tree.Tree.new("A")
      iex> {:ok, tree} = CodingAdventures.Tree.Tree.add_child(tree, "A", "B")
      iex> {:ok, tree} = CodingAdventures.Tree.Tree.add_child(tree, "B", "C")
      iex> {:ok, tree} = CodingAdventures.Tree.Tree.remove_subtree(tree, "B")
      iex> CodingAdventures.Tree.Tree.size(tree)
      1
  """
  @spec remove_subtree(t(), any()) :: {:ok, t()} | {:error, {:node_not_found, any()} | :root_removal}
  def remove_subtree(%__MODULE__{} = tree, node) do
    cond do
      not Graph.has_node?(tree.graph, node) ->
        {:error, {:node_not_found, node}}

      node == tree.root ->
        {:error, :root_removal}

      true ->
        # Collect all nodes in the subtree (BFS order: parent before children)
        subtree_nodes = collect_subtree_nodes(tree.graph, node)

        # Remove in reverse order (children before parents)
        new_graph =
          subtree_nodes
          |> Enum.reverse()
          |> Enum.reduce(tree.graph, fn n, g ->
            {:ok, g} = Graph.remove_node(g, n)
            g
          end)

        {:ok, %{tree | graph: new_graph}}
    end
  end

  # Collect all nodes in the subtree rooted at `node` using BFS.
  # Returns a list starting with `node` followed by all descendants
  # in breadth-first order.
  @spec collect_subtree_nodes(Graph.t(), any()) :: [any()]
  defp collect_subtree_nodes(graph, node) do
    do_collect_bfs([node], [], graph)
  end

  defp do_collect_bfs([], result, _graph), do: Enum.reverse(result)

  defp do_collect_bfs([current | rest], result, graph) do
    {:ok, children} = Graph.successors(graph, current)
    sorted_children = Enum.sort(children)
    do_collect_bfs(rest ++ sorted_children, [current | result], graph)
  end

  # ---------------------------------------------------------------------------
  # Queries
  # ---------------------------------------------------------------------------

  @doc """
  Return the root node of the tree.
  """
  @spec root(t()) :: any()
  def root(%__MODULE__{root: root}), do: root

  @doc """
  Return the parent of a node, or `nil` if the node is the root.

  Returns `{:ok, parent}` or `{:ok, nil}` on success, or
  `{:error, {:node_not_found, node}}` if the node doesn't exist.
  """
  @spec parent(t(), any()) :: {:ok, any() | nil} | {:error, {:node_not_found, any()}}
  def parent(%__MODULE__{} = tree, node) do
    if not Graph.has_node?(tree.graph, node) do
      {:error, {:node_not_found, node}}
    else
      {:ok, predecessors} = Graph.predecessors(tree.graph, node)

      case predecessors do
        [] -> {:ok, nil}
        [p] -> {:ok, p}
      end
    end
  end

  @doc """
  Return the children of a node (sorted).

  Returns `{:ok, children}` on success, or `{:error, {:node_not_found, node}}`
  if the node doesn't exist.
  """
  @spec children(t(), any()) :: {:ok, [any()]} | {:error, {:node_not_found, any()}}
  def children(%__MODULE__{} = tree, node) do
    if not Graph.has_node?(tree.graph, node) do
      {:error, {:node_not_found, node}}
    else
      {:ok, succs} = Graph.successors(tree.graph, node)
      {:ok, Enum.sort(succs)}
    end
  end

  @doc """
  Return the siblings of a node (other children of the same parent).

  The root has no siblings. Returns a sorted list.
  """
  @spec siblings(t(), any()) :: {:ok, [any()]} | {:error, {:node_not_found, any()}}
  def siblings(%__MODULE__{} = tree, node) do
    if not Graph.has_node?(tree.graph, node) do
      {:error, {:node_not_found, node}}
    else
      {:ok, parent_node} = parent(tree, node)

      case parent_node do
        nil ->
          {:ok, []}

        p ->
          {:ok, all_children} = children(tree, p)
          {:ok, Enum.filter(all_children, &(&1 != node))}
      end
    end
  end

  @doc """
  Return `true` if the node has no children (is a leaf).
  """
  @spec is_leaf?(t(), any()) :: boolean()
  def is_leaf?(%__MODULE__{} = tree, node) do
    {:ok, succs} = Graph.successors(tree.graph, node)
    succs == []
  end

  @doc """
  Return `true` if the node is the root.
  """
  @spec is_root?(t(), any()) :: boolean()
  def is_root?(%__MODULE__{root: root}, node), do: node == root

  @doc """
  Return the depth of a node (number of edges from root to node).

  The root has depth 0. Returns `{:ok, depth}` or `{:error, {:node_not_found, node}}`.
  """
  @spec depth(t(), any()) :: {:ok, non_neg_integer()} | {:error, {:node_not_found, any()}}
  def depth(%__MODULE__{} = tree, node) do
    if not Graph.has_node?(tree.graph, node) do
      {:error, {:node_not_found, node}}
    else
      {:ok, do_depth(tree, node, 0)}
    end
  end

  defp do_depth(%__MODULE__{root: root}, node, acc) when node == root, do: acc

  defp do_depth(%__MODULE__{} = tree, node, acc) do
    {:ok, p} = parent(tree, node)
    do_depth(tree, p, acc + 1)
  end

  @doc """
  Return the height of the tree (maximum depth of any node).

  A single-node tree has height 0.
  """
  @spec height(t()) :: non_neg_integer()
  def height(%__MODULE__{} = tree) do
    # BFS with depth tracking
    do_height([{tree.root, 0}], 0, tree.graph)
  end

  defp do_height([], max_depth, _graph), do: max_depth

  defp do_height([{current, d} | rest], max_depth, graph) do
    new_max = max(d, max_depth)
    {:ok, succs} = Graph.successors(graph, current)
    children_with_depth = Enum.map(succs, fn c -> {c, d + 1} end)
    do_height(rest ++ children_with_depth, new_max, graph)
  end

  @doc """
  Return the total number of nodes in the tree.
  """
  @spec size(t()) :: non_neg_integer()
  def size(%__MODULE__{} = tree), do: Graph.size(tree.graph)

  @doc """
  Return a sorted list of all nodes in the tree.
  """
  @spec nodes(t()) :: [any()]
  def nodes(%__MODULE__{} = tree), do: Enum.sort(Graph.nodes(tree.graph))

  @doc """
  Return a sorted list of all leaf nodes.
  """
  @spec leaves(t()) :: [any()]
  def leaves(%__MODULE__{} = tree) do
    tree.graph
    |> Graph.nodes()
    |> Enum.filter(fn node ->
      {:ok, succs} = Graph.successors(tree.graph, node)
      succs == []
    end)
    |> Enum.sort()
  end

  @doc """
  Return `true` if the node exists in the tree.
  """
  @spec has_node?(t(), any()) :: boolean()
  def has_node?(%__MODULE__{} = tree, node), do: Graph.has_node?(tree.graph, node)

  # ---------------------------------------------------------------------------
  # Traversals
  # ---------------------------------------------------------------------------
  #
  # Tree traversals visit every node exactly once, but in different orders.
  #
  # 1. **Preorder** (root first): Visit a node, then visit all its children.
  #    Good for: copying a tree, prefix notation.
  #
  # 2. **Postorder** (root last): Visit all children, then visit the node.
  #    Good for: computing sizes, deleting trees, postfix notation.
  #
  # 3. **Level-order** (breadth-first): Visit by depth level.
  #    Good for: finding shortest paths, printing by level.
  #
  # For a tree:
  #       A
  #      / \
  #     B   C
  #    / \
  #   D   E
  #
  # Preorder:    [A, B, D, E, C]
  # Postorder:   [D, E, B, C, A]
  # Level-order: [A, B, C, D, E]

  @doc """
  Return nodes in preorder (parent before children).

  Children are visited in sorted order.
  """
  @spec preorder(t()) :: [any()]
  def preorder(%__MODULE__{} = tree) do
    do_preorder([tree.root], [], tree.graph)
  end

  # Iterative preorder using an explicit stack.
  # We push children in reverse sorted order so smallest pops first.
  defp do_preorder([], result, _graph), do: Enum.reverse(result)

  defp do_preorder([node | rest], result, graph) do
    {:ok, succs} = Graph.successors(graph, node)
    # In a list-as-stack (LIFO from the head), we prepend sorted children
    # so the smallest child is processed next. This mirrors the Python
    # implementation where we push in reverse order onto an actual stack.
    sorted_children = Enum.sort(succs)
    do_preorder(sorted_children ++ rest, [node | result], graph)
  end

  @doc """
  Return nodes in postorder (children before parent).

  Children are visited in sorted order.
  """
  @spec postorder(t()) :: [any()]
  def postorder(%__MODULE__{} = tree) do
    do_postorder(tree.root, tree.graph)
  end

  defp do_postorder(node, graph) do
    {:ok, succs} = Graph.successors(graph, node)

    children_results =
      succs
      |> Enum.sort()
      |> Enum.flat_map(&do_postorder(&1, graph))

    children_results ++ [node]
  end

  @doc """
  Return nodes in level-order (breadth-first).

  Within each depth level, nodes are in sorted order.
  """
  @spec level_order(t()) :: [any()]
  def level_order(%__MODULE__{} = tree) do
    do_level_order([tree.root], [], tree.graph)
  end

  defp do_level_order([], result, _graph), do: Enum.reverse(result)

  defp do_level_order([node | rest], result, graph) do
    {:ok, succs} = Graph.successors(graph, node)
    sorted_children = Enum.sort(succs)
    do_level_order(rest ++ sorted_children, [node | result], graph)
  end

  # ---------------------------------------------------------------------------
  # Utilities
  # ---------------------------------------------------------------------------

  @doc """
  Return the path from the root to the given node.

  The path is a list starting with the root and ending with the target node.

  Returns `{:ok, path}` or `{:error, {:node_not_found, node}}`.
  """
  @spec path_to(t(), any()) :: {:ok, [any()]} | {:error, {:node_not_found, any()}}
  def path_to(%__MODULE__{} = tree, node) do
    if not Graph.has_node?(tree.graph, node) do
      {:error, {:node_not_found, node}}
    else
      {:ok, do_path_to(tree, node, [])}
    end
  end

  defp do_path_to(tree, node, acc) do
    {:ok, p} = parent(tree, node)

    case p do
      nil -> [node | acc]
      parent_node -> do_path_to(tree, parent_node, [node | acc])
    end
  end

  @doc """
  Return the lowest common ancestor (LCA) of nodes `a` and `b`.

  The LCA is the deepest node that is an ancestor of both `a` and `b`.

  Algorithm:
  1. Compute the path from root to `a`.
  2. Compute the path from root to `b`.
  3. Walk both paths from root; the last node where both paths agree is the LCA.

  Returns `{:ok, lca_node}` or `{:error, {:node_not_found, node}}`.
  """
  @spec lca(t(), any(), any()) :: {:ok, any()} | {:error, {:node_not_found, any()}}
  def lca(%__MODULE__{} = tree, a, b) do
    cond do
      not Graph.has_node?(tree.graph, a) ->
        {:error, {:node_not_found, a}}

      not Graph.has_node?(tree.graph, b) ->
        {:error, {:node_not_found, b}}

      true ->
        {:ok, path_a} = path_to(tree, a)
        {:ok, path_b} = path_to(tree, b)
        {:ok, do_lca(path_a, path_b, tree.root)}
    end
  end

  defp do_lca([], _path_b, lca_node), do: lca_node
  defp do_lca(_path_a, [], lca_node), do: lca_node

  defp do_lca([ha | ta], [hb | tb], lca_node) do
    if ha == hb do
      do_lca(ta, tb, ha)
    else
      lca_node
    end
  end

  @doc """
  Extract the subtree rooted at the given node.

  Returns a NEW `Tree` struct containing the node and all its descendants.
  The original tree is not modified.

  Returns `{:ok, new_tree}` or `{:error, {:node_not_found, node}}`.
  """
  @spec subtree(t(), any()) :: {:ok, t()} | {:error, {:node_not_found, any()}}
  def subtree(%__MODULE__{} = tree, node) do
    if not Graph.has_node?(tree.graph, node) do
      {:error, {:node_not_found, node}}
    else
      new_tree = new(node)
      {:ok, do_build_subtree([node], new_tree, tree.graph)}
    end
  end

  defp do_build_subtree([], new_tree, _graph), do: new_tree

  defp do_build_subtree([current | rest], new_tree, graph) do
    {:ok, succs} = Graph.successors(graph, current)
    sorted_children = Enum.sort(succs)

    new_tree =
      Enum.reduce(sorted_children, new_tree, fn child, acc ->
        {:ok, acc} = add_child(acc, current, child)
        acc
      end)

    do_build_subtree(rest ++ sorted_children, new_tree, graph)
  end

  # ---------------------------------------------------------------------------
  # Visualization
  # ---------------------------------------------------------------------------

  @doc """
  Render the tree as an ASCII art diagram.

  Produces output like:

      Program
      ├── Assignment
      │   ├── BinaryOp
      │   └── Name
      └── Print

  Children are displayed in sorted (alphabetical) order.
  """
  @spec to_ascii(t()) :: String.t()
  def to_ascii(%__MODULE__{} = tree) do
    lines = ascii_recursive(tree.graph, tree.root, "", "")
    Enum.join(lines, "\n")
  end

  # Recursive helper for to_ascii.
  #
  # For each node, we produce one line with the appropriate prefix,
  # then recursively produce lines for children with updated prefixes.
  #
  # The box-drawing characters:
  # - "├── " for non-last children
  # - "└── " for the last child
  # - "│   " for continuation lines under non-last children
  # - "    " for continuation lines under the last child
  defp ascii_recursive(graph, node, prefix, child_prefix) do
    {:ok, succs} = Graph.successors(graph, node)
    sorted_children = Enum.sort(succs)
    num_children = length(sorted_children)

    child_lines =
      sorted_children
      |> Enum.with_index()
      |> Enum.flat_map(fn {child, i} ->
        if i < num_children - 1 do
          # Not the last child
          ascii_recursive(graph, child, child_prefix <> "├── ", child_prefix <> "│   ")
        else
          # Last child
          ascii_recursive(graph, child, child_prefix <> "└── ", child_prefix <> "    ")
        end
      end)

    [prefix <> to_string(node) | child_lines]
  end

  @doc """
  Access the underlying `DirectedGraph`.

  This is exposed for advanced use cases. Modifying the graph directly
  may violate tree invariants.
  """
  @spec graph(t()) :: Graph.t()
  def graph(%__MODULE__{graph: graph}), do: graph
end
