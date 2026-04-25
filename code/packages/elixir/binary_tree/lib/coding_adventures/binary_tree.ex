defmodule CodingAdventures.BinaryTree do
  @moduledoc """
  Generic binary tree utilities.
  """

  defmodule Node do
    @moduledoc "A binary tree node."
    defstruct [:value, :left, :right]

    @type t(value) :: %__MODULE__{
            value: value,
            left: t(value) | nil,
            right: t(value) | nil
          }
  end

  defstruct [:root]

  @type t(value) :: %__MODULE__{root: Node.t(value) | nil}

  @spec new(Node.t(value) | value | nil) :: t(value) when value: var
  def new(root \\ nil)
  def new(nil), do: %__MODULE__{root: nil}
  def new(%Node{} = root), do: %__MODULE__{root: root}
  def new(value), do: %__MODULE__{root: %Node{value: value}}

  @spec with_root(Node.t(value) | nil) :: t(value) when value: var
  def with_root(root), do: new(root)

  @spec from_level_order([value | nil]) :: t(value) when value: var
  def from_level_order(values) do
    %__MODULE__{root: build_from_level_order(List.to_tuple(values), 0)}
  end

  @spec find(t(value), value) :: Node.t(value) | nil when value: var
  def find(%__MODULE__{root: root}, value), do: find_node(root, value)

  @spec left_child(t(value), value) :: Node.t(value) | nil when value: var
  def left_child(tree, value) do
    case find(tree, value) do
      nil -> nil
      %Node{left: left} -> left
    end
  end

  @spec right_child(t(value), value) :: Node.t(value) | nil when value: var
  def right_child(tree, value) do
    case find(tree, value) do
      nil -> nil
      %Node{right: right} -> right
    end
  end

  @spec full?(t(any())) :: boolean()
  def full?(%__MODULE__{root: root}), do: full_node?(root)

  @spec complete?(t(any())) :: boolean()
  def complete?(%__MODULE__{root: root}), do: complete_node?(root)

  @spec perfect?(t(any())) :: boolean()
  def perfect?(%__MODULE__{root: root}), do: perfect_node?(root)

  @spec height(t(any())) :: integer()
  def height(%__MODULE__{root: root}), do: height_node(root)

  @spec size(t(any())) :: non_neg_integer()
  def size(%__MODULE__{root: root}), do: size_node(root)

  @spec inorder(t(value)) :: [value] when value: var
  def inorder(%__MODULE__{root: root}), do: inorder_node(root)

  @spec preorder(t(value)) :: [value] when value: var
  def preorder(%__MODULE__{root: root}), do: preorder_node(root)

  @spec postorder(t(value)) :: [value] when value: var
  def postorder(%__MODULE__{root: root}), do: postorder_node(root)

  @spec level_order(t(value)) :: [value] when value: var
  def level_order(%__MODULE__{root: nil}), do: []

  def level_order(%__MODULE__{root: root}) do
    level_order_queue(:queue.from_list([root]), [])
  end

  @spec to_array(t(value)) :: [value | nil] when value: var
  def to_array(%__MODULE__{} = tree) do
    tree_height = height(tree)

    if tree_height < 0 do
      []
    else
      values = List.duplicate(nil, Bitwise.bsl(1, tree_height + 1) - 1)
      fill_array(tree.root, 0, values)
    end
  end

  @spec to_ascii(t(any())) :: String.t()
  def to_ascii(%__MODULE__{root: nil}), do: ""

  def to_ascii(%__MODULE__{root: root}) do
    root
    |> render_ascii("", true, [])
    |> Enum.reverse()
    |> Enum.join("\n")
  end

  defp find_node(nil, _value), do: nil
  defp find_node(%Node{value: value} = node, value), do: node

  defp find_node(%Node{left: left, right: right}, value) do
    find_node(left, value) || find_node(right, value)
  end

  defp full_node?(nil), do: true
  defp full_node?(%Node{left: nil, right: nil}), do: true
  defp full_node?(%Node{left: nil}), do: false
  defp full_node?(%Node{right: nil}), do: false
  defp full_node?(%Node{left: left, right: right}), do: full_node?(left) and full_node?(right)

  defp complete_node?(root) do
    complete_queue(:queue.from_list([root]), false)
  end

  defp complete_queue(queue, seen_nil) do
    case :queue.out(queue) do
      {:empty, _queue} ->
        true

      {{:value, nil}, rest} ->
        complete_queue(rest, true)

      {{:value, %Node{}}, _rest} when seen_nil ->
        false

      {{:value, %Node{left: left, right: right}}, rest} ->
        rest = :queue.in(left, rest)
        rest = :queue.in(right, rest)
        complete_queue(rest, false)
    end
  end

  defp perfect_node?(root) do
    tree_height = height_node(root)

    if tree_height < 0 do
      size_node(root) == 0
    else
      size_node(root) == Bitwise.bsl(1, tree_height + 1) - 1
    end
  end

  defp height_node(nil), do: -1

  defp height_node(%Node{left: left, right: right}) do
    1 + max(height_node(left), height_node(right))
  end

  defp size_node(nil), do: 0

  defp size_node(%Node{left: left, right: right}) do
    1 + size_node(left) + size_node(right)
  end

  defp build_from_level_order(values, index) when index >= tuple_size(values), do: nil

  defp build_from_level_order(values, index) do
    case elem(values, index) do
      nil ->
        nil

      value ->
        %Node{
          value: value,
          left: build_from_level_order(values, 2 * index + 1),
          right: build_from_level_order(values, 2 * index + 2)
        }
    end
  end

  defp inorder_node(nil), do: []

  defp inorder_node(%Node{value: value, left: left, right: right}),
    do: inorder_node(left) ++ [value] ++ inorder_node(right)

  defp preorder_node(nil), do: []

  defp preorder_node(%Node{value: value, left: left, right: right}),
    do: [value] ++ preorder_node(left) ++ preorder_node(right)

  defp postorder_node(nil), do: []

  defp postorder_node(%Node{value: value, left: left, right: right}),
    do: postorder_node(left) ++ postorder_node(right) ++ [value]

  defp level_order_queue(queue, out) do
    case :queue.out(queue) do
      {:empty, _queue} ->
        Enum.reverse(out)

      {{:value, %Node{value: value, left: left, right: right}}, rest} ->
        rest =
          [left, right]
          |> Enum.reject(&is_nil/1)
          |> Enum.reduce(rest, fn child, acc -> :queue.in(child, acc) end)

        level_order_queue(rest, [value | out])
    end
  end

  defp fill_array(nil, _index, out), do: out
  defp fill_array(_node, index, out) when index >= length(out), do: out

  defp fill_array(%Node{value: value, left: left, right: right}, index, out) do
    out
    |> List.replace_at(index, value)
    |> then(&fill_array(left, 2 * index + 1, &1))
    |> then(&fill_array(right, 2 * index + 2, &1))
  end

  defp render_ascii(%Node{value: value, left: left, right: right}, prefix, is_tail, out) do
    connector = if is_tail, do: "`-- ", else: "|-- "
    line = "#{prefix}#{connector}#{inspect(value)}"
    children = Enum.reject([left, right], &is_nil/1)
    next_prefix = "#{prefix}#{if is_tail, do: "    ", else: "|   "}"

    children
    |> Enum.with_index()
    |> Enum.reduce([line | out], fn {child, index}, acc ->
      render_ascii(child, next_prefix, index + 1 == length(children), acc)
    end)
  end
end

defimpl String.Chars, for: CodingAdventures.BinaryTree do
  alias CodingAdventures.BinaryTree

  def to_string(%BinaryTree{root: nil}), do: "BinaryTree(root=nil, size=0)"

  def to_string(%BinaryTree{root: %BinaryTree.Node{value: value}} = tree) do
    "BinaryTree(root=#{inspect(value)}, size=#{BinaryTree.size(tree)})"
  end
end
