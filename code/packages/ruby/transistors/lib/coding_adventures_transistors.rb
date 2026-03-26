# frozen_string_literal: true

# Entry point for the coding_adventures_transistors gem.
#
# This gem implements transistor-level circuit simulation: MOSFETs (NMOS/PMOS),
# BJTs (NPN/PNP), CMOS logic gates, TTL logic gates, analog amplifier analysis,
# and electrical analysis (noise margins, power, timing).
#
# Usage:
#   require "coding_adventures_transistors"
#
#   # MOSFET transistors
#   nmos = CodingAdventures::Transistors::NMOS.new
#   nmos.conducting?(vgs: 3.3)  # => true
#
#   # CMOS logic gates
#   inv = CodingAdventures::Transistors::CMOSInverter.new
#   inv.evaluate_digital(0)  # => 1
#
#   # TTL logic gates
#   nand = CodingAdventures::Transistors::TTLNand.new
#   nand.evaluate_digital(1, 1)  # => 0

require_relative "coding_adventures/transistors/version"
require_relative "coding_adventures/transistors/types"
require_relative "coding_adventures/transistors/mosfet"
require_relative "coding_adventures/transistors/bjt"
require_relative "coding_adventures/transistors/cmos_gates"
require_relative "coding_adventures/transistors/ttl_gates"
require_relative "coding_adventures/transistors/amplifier"
require_relative "coding_adventures/transistors/analysis"
