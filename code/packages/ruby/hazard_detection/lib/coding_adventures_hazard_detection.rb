# frozen_string_literal: true

# Entry point for the coding_adventures_hazard_detection gem.
#
# This gem detects pipeline hazards in a classic 5-stage CPU pipeline:
#
# - DataHazardDetector: Detects RAW data hazards, resolves via forwarding/stalling
# - ControlHazardDetector: Detects branch mispredictions, triggers flushes
# - StructuralHazardDetector: Detects resource conflicts (ALU, FP, memory port)
# - HazardUnit: Combined unit that runs all detectors each cycle
#
# Usage:
#   require "coding_adventures_hazard_detection"
#
#   unit = CodingAdventures::HazardDetection::HazardUnit.new(num_alus: 2)
#   result = unit.check(if_stage, id_stage, ex_stage, mem_stage)

require_relative "coding_adventures/hazard_detection/version"
require_relative "coding_adventures/hazard_detection/types"
require_relative "coding_adventures/hazard_detection/data_hazard"
require_relative "coding_adventures/hazard_detection/control_hazard"
require_relative "coding_adventures/hazard_detection/structural_hazard"
require_relative "coding_adventures/hazard_detection/hazard_unit"
