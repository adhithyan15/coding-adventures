# frozen_string_literal: true

# --------------------------------------------------------------------------
# coding_adventures_tree.rb -- Gem entry point
# --------------------------------------------------------------------------
#
# This is the file that gets loaded when someone writes:
#
#   require "coding_adventures_tree"
#
# It pulls in the internal files in dependency order:
#   1. errors  -- the custom exception classes (no dependencies)
#   2. tree    -- the Tree class (uses errors + DirectedGraph)
#
# Usage:
#   require "coding_adventures_tree"
#
#   t = CodingAdventures::Tree::Tree.new("root")
#   t.add_child("root", "child1")
#   t.add_child("root", "child2")
#   puts t.to_ascii
# --------------------------------------------------------------------------

require_relative "coding_adventures/tree/errors"
require_relative "coding_adventures/tree"
