defmodule CodingAdventures.Parser.ASTNode do
  @moduledoc """
  A generic AST node produced by grammar-driven parsing.

  Every node records which grammar rule created it (`rule_name`) and the
  matched sub-structure (`children`). Children are a mix of `ASTNode`
  structs and `Token` structs.

  Position fields (`start_line`, `start_column`, `end_line`, `end_column`)
  record the source span of the node, computed from the first and last leaf
  tokens in the children tree. These are `nil` when the node has no leaf
  tokens (e.g., an empty repetition).

  This generic representation makes the parser language-agnostic -- the same
  `ASTNode` type works for JSON, Python, Ruby, or any language whose grammar
  is written in a `.grammar` file.
  """

  alias CodingAdventures.Lexer.Token

  defstruct [:rule_name, :start_line, :start_column, :end_line, :end_column, children: []]

  @type t :: %__MODULE__{
          rule_name: String.t(),
          children: [t() | Token.t()],
          start_line: pos_integer() | nil,
          start_column: pos_integer() | nil,
          end_line: pos_integer() | nil,
          end_column: pos_integer() | nil
        }

  @doc "True if this node wraps a single token (no sub-structure)."
  def leaf?(%__MODULE__{children: [%Token{}]}), do: true
  def leaf?(_), do: false

  @doc "The token if this is a leaf node, nil otherwise."
  def token(%__MODULE__{children: [%Token{} = t]}), do: t
  def token(_), do: nil

  @doc """
  Check if a child element is an ASTNode (not a Token).
  """
  def ast_node?(%__MODULE__{}), do: true
  def ast_node?(_), do: false

  @doc """
  Depth-first walk of an AST tree with enter/leave visitor callbacks.

  The visitor is a map or keyword list with optional `:enter` and `:leave`
  functions. Each receives the current node and its parent (nil for root).
  Returning `{:replace, new_node}` replaces the visited node; returning
  `:continue` or any other value keeps the original.

  Token children are not visited -- only ASTNode children are walked.

  This is the generic traversal primitive. Language packages use it for
  cover grammar rewriting, desugaring, and semantic analysis.

  ## Examples

      # Count all nodes:
      count = 0
      walk_ast(root, enter: fn node, _parent -> count = count + 1; :continue end)

      # Find all nodes with a specific rule name:
      nodes = find_nodes(root, "expression")
  """
  def walk_ast(node, visitor) do
    walk_node(node, nil, visitor)
  end

  defp walk_node(node, parent, visitor) do
    # Enter phase
    enter_fn = visitor_get(visitor, :enter)

    current =
      if enter_fn do
        case enter_fn.(node, parent) do
          {:replace, replacement} -> replacement
          _ -> node
        end
      else
        node
      end

    # Walk children recursively
    {new_children, changed?} =
      Enum.map_reduce(current.children, false, fn child, changed ->
        if ast_node?(child) do
          walked = walk_node(child, current, visitor)
          {walked, changed or walked != child}
        else
          {child, changed}
        end
      end)

    current =
      if changed? do
        %{current | children: new_children}
      else
        current
    end

    # Leave phase
    leave_fn = visitor_get(visitor, :leave)

    if leave_fn do
      case leave_fn.(current, parent) do
        {:replace, replacement} -> replacement
        _ -> current
      end
    else
      current
    end
  end

  defp visitor_get(visitor, key) when is_list(visitor), do: Keyword.get(visitor, key)
  defp visitor_get(visitor, key) when is_map(visitor), do: Map.get(visitor, key)
  defp visitor_get(_, _), do: nil

  @doc """
  Find all nodes matching a rule name (depth-first order).
  """
  def find_nodes(node, rule_name) do
    results = :ets.new(:find_nodes_tmp, [:bag, :private])

    walk_ast(node,
      enter: fn n, _parent ->
        if n.rule_name == rule_name, do: :ets.insert(results, {n})
        :continue
      end
    )

    found = :ets.tab2list(results) |> Enum.map(fn {n} -> n end)
    :ets.delete(results)
    found
  end

  @doc """
  Collect all tokens in depth-first order, optionally filtered by type.
  """
  def collect_tokens(node, type \\ nil) do
    do_collect_tokens(node, type, []) |> Enum.reverse()
  end

  defp do_collect_tokens(%__MODULE__{children: children}, type, acc) do
    Enum.reduce(children, acc, fn child, inner_acc ->
      if ast_node?(child) do
        do_collect_tokens(child, type, inner_acc)
      else
        if type == nil or child.type == type do
          [child | inner_acc]
        else
          inner_acc
        end
      end
    end)
  end
end
