# frozen_string_literal: true

# =============================================================================
# coding_adventures_b_tree.rb -- Gem entry point
# =============================================================================
#
# This is the file loaded when a user writes:
#
#   require "coding_adventures_b_tree"
#
# It pulls in internal files in dependency order:
#   1. b_tree/node  -- the BTreeNode class (no dependencies)
#   2. b_tree       -- the BTree class (uses BTreeNode)
#
# Usage:
#   require "coding_adventures_b_tree"
#
#   tree = CodingAdventures::BTree.new(t: 3)
#   tree.insert(10, "ten")
#   tree.search(10)   # => "ten"
# =============================================================================

require_relative "coding_adventures/b_tree/node"
require_relative "coding_adventures/b_tree"
