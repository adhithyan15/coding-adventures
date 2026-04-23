# frozen_string_literal: true

# --------------------------------------------------------------------------
# version.rb — Gem version constant
# --------------------------------------------------------------------------
#
# Keeping the version in its own file lets the gemspec `require_relative` it
# without loading the entire library.  This avoids dependency-resolution
# headaches and keeps `gem build` fast.
# --------------------------------------------------------------------------

module CodingAdventures
  module MarkovChainVersion
    VERSION = "0.1.0"
  end
end
