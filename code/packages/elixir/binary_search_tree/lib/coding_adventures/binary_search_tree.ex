defmodule CodingAdventures.BinarySearchTree do
  @moduledoc """
  Functional binary search tree with order statistics.
  """

  defmodule Node do
    @moduledoc "A binary search tree node."
    defstruct [:value, :left, :right, size: 1]
  end

  defstruct [:root]

  def empty, do: %__MODULE__{root: nil}

  def from_sorted_array(values) do
    %__MODULE__{root: build_balanced(values)}
  end

  def insert(%__MODULE__{root: root}, value), do: %__MODULE__{root: insert_node(root, value)}

  def delete(%__MODULE__{root: root}, value), do: %__MODULE__{root: delete_node(root, value)}

  def search(%__MODULE__{root: root}, value), do: search_node(root, value)

  def contains?(tree, value), do: search(tree, value) != nil

  def min_value(%__MODULE__{root: root}), do: min_node(root)

  def max_value(%__MODULE__{root: root}), do: max_node(root)

  def predecessor(%__MODULE__{root: root}, value), do: predecessor_node(root, value, nil)

  def successor(%__MODULE__{root: root}, value), do: successor_node(root, value, nil)

  def kth_smallest(%__MODULE__{root: root}, k), do: kth_node(root, k)

  def rank(%__MODULE__{root: root}, value), do: rank_node(root, value)

  def to_sorted_array(%__MODULE__{root: root}), do: inorder(root)

  def valid?(%__MODULE__{root: root}), do: validate(root, nil, nil) != nil

  def height(%__MODULE__{root: root}), do: height_node(root)

  def size(%__MODULE__{root: root}), do: size_node(root)

  defp search_node(nil, _value), do: nil

  defp search_node(%Node{value: current, left: left}, value) when value < current,
    do: search_node(left, value)

  defp search_node(%Node{value: current, right: right}, value) when value > current,
    do: search_node(right, value)

  defp search_node(%Node{} = node, _value), do: node

  defp insert_node(nil, value), do: %Node{value: value}

  defp insert_node(%Node{value: current, left: left, right: right} = node, value)
       when value < current do
    node_with_children(node, insert_node(left, value), right)
  end

  defp insert_node(%Node{value: current, left: left, right: right} = node, value)
       when value > current do
    node_with_children(node, left, insert_node(right, value))
  end

  defp insert_node(%Node{} = node, _value), do: node

  defp delete_node(nil, _value), do: nil

  defp delete_node(%Node{value: current, left: left, right: right} = node, value)
       when value < current do
    node_with_children(node, delete_node(left, value), right)
  end

  defp delete_node(%Node{value: current, left: left, right: right} = node, value)
       when value > current do
    node_with_children(node, left, delete_node(right, value))
  end

  defp delete_node(%Node{left: nil, right: right}, _value), do: right
  defp delete_node(%Node{left: left, right: nil}, _value), do: left

  defp delete_node(%Node{left: left, right: right}, _value) do
    {new_right, successor} = extract_min(right)
    new_node(successor, left, new_right)
  end

  defp extract_min(%Node{left: nil, right: right, value: value}), do: {right, value}

  defp extract_min(%Node{left: left, right: right} = node) do
    {new_left, minimum} = extract_min(left)
    {node_with_children(node, new_left, right), minimum}
  end

  defp min_node(nil), do: nil
  defp min_node(%Node{left: nil, value: value}), do: value
  defp min_node(%Node{left: left}), do: min_node(left)

  defp max_node(nil), do: nil
  defp max_node(%Node{right: nil, value: value}), do: value
  defp max_node(%Node{right: right}), do: max_node(right)

  defp predecessor_node(nil, _value, best), do: best

  defp predecessor_node(%Node{value: current, left: left}, value, best) when value <= current,
    do: predecessor_node(left, value, best)

  defp predecessor_node(%Node{value: current, right: right}, value, _best),
    do: predecessor_node(right, value, current)

  defp successor_node(nil, _value, best), do: best

  defp successor_node(%Node{value: current, right: right}, value, best) when value >= current,
    do: successor_node(right, value, best)

  defp successor_node(%Node{value: current, left: left}, value, _best),
    do: successor_node(left, value, current)

  defp kth_node(nil, _k), do: nil
  defp kth_node(_node, k) when k <= 0, do: nil

  defp kth_node(%Node{value: value, left: left, right: right}, k) do
    left_size = size_node(left)

    cond do
      k == left_size + 1 -> value
      k <= left_size -> kth_node(left, k)
      true -> kth_node(right, k - left_size - 1)
    end
  end

  defp rank_node(nil, _value), do: 0

  defp rank_node(%Node{value: current, left: left}, value) when value < current,
    do: rank_node(left, value)

  defp rank_node(%Node{value: current, left: left, right: right}, value) when value > current,
    do: size_node(left) + 1 + rank_node(right, value)

  defp rank_node(%Node{left: left}, _value), do: size_node(left)

  defp inorder(nil), do: []

  defp inorder(%Node{value: value, left: left, right: right}),
    do: inorder(left) ++ [value] ++ inorder(right)

  defp validate(nil, _min, _max), do: {-1, 0}

  defp validate(%Node{value: value}, min, _max) when min != nil and value <= min, do: nil
  defp validate(%Node{value: value}, _min, max) when max != nil and value >= max, do: nil

  defp validate(%Node{value: value, left: left, right: right, size: stored_size}, min, max) do
    with {left_height, left_size} <- validate(left, min, value),
         {right_height, right_size} <- validate(right, value, max),
         computed_size = 1 + left_size + right_size,
         true <- stored_size == computed_size do
      {1 + max(left_height, right_height), computed_size}
    else
      _ -> nil
    end
  end

  defp height_node(nil), do: -1

  defp height_node(%Node{left: left, right: right}),
    do: 1 + max(height_node(left), height_node(right))

  defp size_node(nil), do: 0
  defp size_node(%Node{size: size}), do: size

  defp build_balanced([]), do: nil

  defp build_balanced(values) do
    mid = div(length(values), 2)
    {left_values, [value | right_values]} = Enum.split(values, mid)
    new_node(value, build_balanced(left_values), build_balanced(right_values))
  end

  defp node_with_children(%Node{value: value}, left, right), do: new_node(value, left, right)

  defp new_node(value, left, right) do
    %Node{value: value, left: left, right: right, size: 1 + size_node(left) + size_node(right)}
  end
end

defimpl String.Chars, for: CodingAdventures.BinarySearchTree do
  alias CodingAdventures.BinarySearchTree

  def to_string(%BinarySearchTree{root: nil}), do: "BinarySearchTree(root=nil, size=0)"

  def to_string(%BinarySearchTree{root: %BinarySearchTree.Node{value: value}} = tree) do
    "BinarySearchTree(root=#{inspect(value)}, size=#{BinarySearchTree.size(tree)})"
  end
end
