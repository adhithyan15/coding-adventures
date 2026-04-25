defmodule CodingAdventures.Trie.Node do
  @moduledoc false
  defstruct children: %{}, terminal: false, value: nil
end

defmodule CodingAdventures.Trie do
  @moduledoc """
  Immutable trie for string keys with prefix-oriented operations.
  """

  alias CodingAdventures.Trie.Node

  defstruct root: %Node{}, size: 0

  @type entry :: {String.t(), any()}
  @type t :: %__MODULE__{root: %Node{}, size: non_neg_integer()}

  def new, do: %__MODULE__{}

  def from_list(entries) when is_list(entries) do
    Enum.reduce(entries, new(), fn
      {key, value}, trie -> insert(trie, key, value)
      other, _trie -> raise ArgumentError, "expected {key, value} tuple, got #{inspect(other)}"
    end)
  end

  def insert(%__MODULE__{} = trie, key, value \\ true) when is_binary(key) do
    {root, grew?} = insert_node(trie.root, String.codepoints(key), value)
    %{trie | root: root, size: if(grew?, do: trie.size + 1, else: trie.size)}
  end

  def search(%__MODULE__{} = trie, key) when is_binary(key) do
    case find_node(trie.root, String.codepoints(key)) do
      %Node{terminal: true, value: value} -> {:ok, value}
      _ -> :error
    end
  end

  def get(%__MODULE__{} = trie, key, default \\ nil) when is_binary(key) do
    case search(trie, key) do
      {:ok, value} -> value
      :error -> default
    end
  end

  def contains?(%__MODULE__{} = trie, key) when is_binary(key) do
    match?({:ok, _value}, search(trie, key))
  end

  def delete(%__MODULE__{} = trie, key) when is_binary(key) do
    if contains?(trie, key) do
      {root, _prune?} = delete_node(trie.root, String.codepoints(key))
      {%{trie | root: root, size: trie.size - 1}, true}
    else
      {trie, false}
    end
  end

  def starts_with?(%__MODULE__{} = trie, prefix) when is_binary(prefix) do
    if prefix == "" do
      trie.size > 0
    else
      find_node(trie.root, String.codepoints(prefix)) != nil
    end
  end

  def words_with_prefix(%__MODULE__{} = trie, prefix) when is_binary(prefix) do
    case find_node(trie.root, String.codepoints(prefix)) do
      nil -> []
      node -> collect(node, prefix)
    end
  end

  def all_words(%__MODULE__{} = trie), do: collect(trie.root, "")

  def keys(%__MODULE__{} = trie) do
    trie
    |> all_words()
    |> Enum.map(fn {key, _value} -> key end)
  end

  def longest_prefix_match(%__MODULE__{} = trie, input) when is_binary(input) do
    longest_prefix_match(trie.root, String.codepoints(input), "", :error)
  end

  def size(%__MODULE__{size: size}), do: size

  def empty?(%__MODULE__{size: 0}), do: true
  def empty?(%__MODULE__{}), do: false

  def valid?(%__MODULE__{} = trie), do: count_endpoints(trie.root) == trie.size

  defp insert_node(%Node{} = node, [], value) do
    grew? = !node.terminal
    {%Node{node | terminal: true, value: value}, grew?}
  end

  defp insert_node(%Node{} = node, [char | rest], value) do
    child = Map.get(node.children, char, %Node{})
    {child, grew?} = insert_node(child, rest, value)
    {%Node{node | children: Map.put(node.children, char, child)}, grew?}
  end

  defp delete_node(%Node{} = node, []) do
    node = %Node{node | terminal: false, value: nil}
    {node, map_size(node.children) == 0}
  end

  defp delete_node(%Node{} = node, [char | rest]) do
    {child, prune?} = node.children |> Map.fetch!(char) |> delete_node(rest)

    children =
      if prune? do
        Map.delete(node.children, char)
      else
        Map.put(node.children, char, child)
      end

    node = %Node{node | children: children}
    {node, map_size(children) == 0 and !node.terminal}
  end

  defp find_node(%Node{} = node, []), do: node

  defp find_node(%Node{} = node, [char | rest]) do
    case Map.fetch(node.children, char) do
      {:ok, child} -> find_node(child, rest)
      :error -> nil
    end
  end

  defp collect(%Node{} = node, current) do
    own =
      if node.terminal do
        [{current, node.value}]
      else
        []
      end

    children =
      node.children
      |> Map.keys()
      |> Enum.sort()
      |> Enum.flat_map(fn char ->
        child = Map.fetch!(node.children, char)
        collect(child, current <> char)
      end)

    own ++ children
  end

  defp longest_prefix_match(%Node{} = node, chars, current, best) do
    best =
      if node.terminal do
        {:ok, {current, node.value}}
      else
        best
      end

    case chars do
      [] ->
        best

      [char | rest] ->
        case Map.fetch(node.children, char) do
          {:ok, child} -> longest_prefix_match(child, rest, current <> char, best)
          :error -> best
        end
    end
  end

  defp count_endpoints(%Node{} = node) do
    base = if node.terminal, do: 1, else: 0

    Enum.reduce(node.children, base, fn {_char, child}, acc ->
      acc + count_endpoints(child)
    end)
  end
end
