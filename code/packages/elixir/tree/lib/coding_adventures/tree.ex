defmodule CodingAdventures.Tree do
  @moduledoc """
  Tree Library for Elixir
  ========================

  A rooted tree data structure backed by a directed graph. Provides tree traversals,
  lowest common ancestor (LCA), subtree extraction, and ASCII visualization.

  This module serves as the public API, delegating to `CodingAdventures.Tree.Tree`
  for the actual implementation.

  ## What Is a Tree?

  A tree is one of the most fundamental data structures in computer science.
  File systems, HTML documents, Abstract Syntax Trees (ASTs), and organization
  charts are all trees.

  Formally, a tree is a connected, acyclic graph where:

  1. There is exactly **one root** node (no parent).
  2. Every other node has exactly **one parent**.
  3. There are **no cycles**.

  ## Quick Start

      alias CodingAdventures.Tree.Tree

      tree = Tree.new("Program")
      {:ok, tree} = Tree.add_child(tree, "Program", "Assignment")
      {:ok, tree} = Tree.add_child(tree, "Program", "Print")
      {:ok, tree} = Tree.add_child(tree, "Assignment", "Name")

      Tree.preorder(tree)   # ["Program", "Assignment", "Name", "Print"]
      Tree.to_ascii(tree)   # ASCII art visualization

  ## Immutability

  All operations return new Tree structs (or `{:ok, tree}` / `{:error, reason}`
  tuples). The original tree is never modified. This is idiomatic Elixir.
  """
end
