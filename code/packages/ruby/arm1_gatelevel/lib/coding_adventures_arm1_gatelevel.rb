# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_logic_gates"
require "coding_adventures_arithmetic"
require "coding_adventures_arm1_simulator"

require_relative "coding_adventures/arm1_gatelevel/version"
require_relative "coding_adventures/arm1_gatelevel/simulator"

module CodingAdventures
  # ARM1 gate-level simulator built from logic gates
  module Arm1Gatelevel
  end
end
