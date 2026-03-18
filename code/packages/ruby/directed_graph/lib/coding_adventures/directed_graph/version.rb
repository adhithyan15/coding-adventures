# frozen_string_literal: true

# --------------------------------------------------------------------------
# version.rb — Gem version constant
# --------------------------------------------------------------------------
#
# We keep the version in its own file so the gemspec can require it without
# loading the entire library.  This avoids circular-dependency headaches
# and keeps `bundle exec rake build` fast.
# --------------------------------------------------------------------------

module CodingAdventures
  module DirectedGraph
    VERSION = "0.1.0"
  end
end
