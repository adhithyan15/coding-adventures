# frozen_string_literal: true

# ============================================================================
# coding_adventures_repl — top-level require entry point
# ============================================================================
#
# This file is what users `require` in their applications:
#
#   require "coding_adventures_repl"
#
# It loads the entire framework by delegating to the main module file, which
# in turn requires all subfiles in dependency order.
#
# ## Why two files (this one and coding_adventures/repl.rb)?
#
# Ruby gem conventions use a top-level file named after the gem:
#
#   lib/coding_adventures_repl.rb        ← what users `require`
#   lib/coding_adventures/repl.rb        ← the actual module definition
#   lib/coding_adventures/repl/*.rb      ← subcomponents
#
# The top-level file is a simple shim that loads the real module. This keeps
# the gem's public require path clean ("coding_adventures_repl") while keeping
# the source code organized under a namespaced directory.

require_relative "coding_adventures/repl"
