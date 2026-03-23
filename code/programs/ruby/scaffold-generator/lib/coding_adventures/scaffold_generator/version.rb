# frozen_string_literal: true

# version.rb -- Version constant for the scaffold-generator program
# ===================================================================
#
# We keep the version in its own file so that the gemspec can load it
# without pulling in the entire dependency tree. This is standard Ruby
# gem practice.

module CodingAdventures
  module ScaffoldGenerator
    VERSION = "1.0.0"
  end
end
