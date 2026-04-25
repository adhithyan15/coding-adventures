defmodule CodingAdventures.AVLTree do
  @moduledoc "Persistent AVL tree with order statistics."

  defmodule Node do
    @moduledoc "An AVL tree node."
    defstruct [:value, :left, :right, height: 0, size: 1]
  end

  defstruct [:root]

  def empty, do: %__MODULE__{root: nil}

  def from_values(values), do: Enum.reduce(values, empty(), &insert(&2, &1))

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
  def valid_bst?(%__MODULE__{root: root}), do: validate_bst(root, nil, nil)
  def valid_avl?(%__MODULE__{root: root}), do: validate_avl(root, nil, nil) != nil
  def balance_factor(nil), do: 0
  def balance_factor(%Node{left: left, right: right}), do: height_node(left) - height_node(right)
  def height(%__MODULE__{root: root}), do: height_node(root)
  def size(%__MODULE__{root: root}), do: size_node(root)

  defp search_node(nil, _value), do: nil

  defp search_node(%Node{value: current, left: left}, value) when value < current,
    do: search_node(left, value)

  defp search_node(%Node{value: current, right: right}, value) when value > current,
    do: search_node(right, value)

  defp search_node(%Node{} = node, _value), do: node

  defp insert_node(nil, value), do: %Node{value: value}

  defp insert_node(%Node{value: current, left: left, right: right}, value) when value < current,
    do: rebalance(new_node(current, insert_node(left, value), right))

  defp insert_node(%Node{value: current, left: left, right: right}, value) when value > current,
    do: rebalance(new_node(current, left, insert_node(right, value)))

  defp insert_node(%Node{} = node, _value), do: node

  defp delete_node(nil, _value), do: nil

  defp delete_node(%Node{value: current, left: left, right: right}, value) when value < current,
    do: rebalance(new_node(current, delete_node(left, value), right))

  defp delete_node(%Node{value: current, left: left, right: right}, value) when value > current,
    do: rebalance(new_node(current, left, delete_node(right, value)))

  defp delete_node(%Node{left: nil, right: right}, _value), do: right
  defp delete_node(%Node{left: left, right: nil}, _value), do: left

  defp delete_node(%Node{left: left, right: right}, _value) do
    {new_right, successor} = extract_min(right)
    rebalance(new_node(successor, left, new_right))
  end

  defp extract_min(%Node{left: nil, right: right, value: value}), do: {right, value}

  defp extract_min(%Node{value: value, left: left, right: right}) do
    {new_left, minimum} = extract_min(left)
    {rebalance(new_node(value, new_left, right)), minimum}
  end

  defp rebalance(%Node{} = node) do
    bf = balance_factor(node)

    cond do
      bf > 1 ->
        left = if balance_factor(node.left) < 0, do: rotate_left(node.left), else: node.left
        rotate_right(new_node(node.value, left, node.right))

      bf < -1 ->
        right = if balance_factor(node.right) > 0, do: rotate_right(node.right), else: node.right
        rotate_left(new_node(node.value, node.left, right))

      true ->
        node
    end
  end

  defp rotate_left(%Node{right: nil} = root), do: root

  defp rotate_left(%Node{value: value, left: left, right: %Node{} = right}) do
    new_left = new_node(value, left, right.left)
    new_node(right.value, new_left, right.right)
  end

  defp rotate_right(%Node{left: nil} = root), do: root

  defp rotate_right(%Node{value: value, left: %Node{} = left, right: right}) do
    new_right = new_node(value, left.right, right)
    new_node(left.value, left.left, new_right)
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

  defp validate_bst(nil, _min, _max), do: true
  defp validate_bst(%Node{value: value}, min, _max) when min != nil and value <= min, do: false
  defp validate_bst(%Node{value: value}, _min, max) when max != nil and value >= max, do: false

  defp validate_bst(%Node{value: value, left: left, right: right}, min, max),
    do: validate_bst(left, min, value) and validate_bst(right, value, max)

  defp validate_avl(nil, _min, _max), do: {-1, 0}
  defp validate_avl(%Node{value: value}, min, _max) when min != nil and value <= min, do: nil
  defp validate_avl(%Node{value: value}, _min, max) when max != nil and value >= max, do: nil

  defp validate_avl(
         %Node{value: value, left: left, right: right, height: stored_h, size: stored_s},
         min,
         max
       ) do
    with {left_h, left_s} <- validate_avl(left, min, value),
         {right_h, right_s} <- validate_avl(right, value, max),
         computed_h = 1 + max(left_h, right_h),
         computed_s = 1 + left_s + right_s,
         true <- stored_h == computed_h and stored_s == computed_s and abs(left_h - right_h) <= 1 do
      {computed_h, computed_s}
    else
      _ -> nil
    end
  end

  defp height_node(nil), do: -1
  defp height_node(%Node{height: height}), do: height
  defp size_node(nil), do: 0
  defp size_node(%Node{size: size}), do: size

  defp new_node(value, left, right) do
    %Node{
      value: value,
      left: left,
      right: right,
      height: 1 + max(height_node(left), height_node(right)),
      size: 1 + size_node(left) + size_node(right)
    }
  end
end
