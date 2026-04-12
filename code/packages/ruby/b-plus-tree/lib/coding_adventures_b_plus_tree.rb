# frozen_string_literal: true

# =============================================================================
# coding_adventures_b_plus_tree.rb -- Gem entry point
# =============================================================================
#
# This file is loaded when a user writes:
#
#   require "coding_adventures_b_plus_tree"
#
# Load order:
#   1. b_plus_tree/node  -- BPlusLeafNode and BPlusInternalNode
#   2. b_plus_tree       -- BPlusTree class
# =============================================================================

require_relative "coding_adventures/b_plus_tree/node"
require_relative "coding_adventures/b_plus_tree"
