# frozen_string_literal: true

# Cross-validation: behavioral vs gate-level simulators.
#
# Run the same programs on both simulators and verify identical results.
# This is the ultimate correctness test -- the gate-level simulator must
# produce exactly the same output as the behavioral one for any program.

require "test_helper"

# The behavioral simulator may or may not be installed
begin
  require "coding_adventures_intel4004_simulator"
  HAS_BEHAVIORAL = true
rescue LoadError
  HAS_BEHAVIORAL = false
end

module CodingAdventures
  module Intel4004Gatelevel
    PROGRAMS = {
      "x = 1 + 2" => [0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01],
      "multiply 3x4" => [
        0xD3, 0xB0, 0xDC, 0xB1,
        0xD0, 0x80, 0x71, 0x05,
        0xB2, 0x01
      ],
      "BCD 7+8" => [
        0xD8, 0xB0, 0xD7, 0x80, 0xFB, 0x01
      ],
      "complement and add" => [
        0xD5, 0xF4, 0xB0, # LDM 5, CMA, XCH R0 (R0=10)
        0xD3, 0x80,       # LDM 3, ADD R0 (A=13)
        0xB1, 0x01        # XCH R1, HLT
      ],
      "rotate left" => [
        0xD5, 0xF5, 0xF5, 0xB0, 0x01 # LDM 5, RAL, RAL, XCH R0, HLT
      ],
      "subroutine call" => [
        0x50, 0x04, # JMS 0x004
        0x01,       # HLT
        0x00,       # padding
        0xC7        # BBL 7
      ],
      "countdown" => [
        0xD5, 0xF8, 0x1C, 0x01, 0x01
      ],
      "all accum ops" => [
        0xD5,       # LDM 5
        0xFA,       # STC (carry=1)
        0xF7,       # TCC (A=1, carry=0)
        0xF2,       # IAC (A=2)
        0xF3,       # CMC (carry=1)
        0xF1,       # CLC (carry=0)
        0xF4,       # CMA (A=~2=13)
        0xF8,       # DAC (A=12)
        0xB0, 0x01  # XCH R0, HLT
      ]
    }.freeze

    class TestCrossValidation < Minitest::Test
      PROGRAMS.each do |name, program|
        define_method("test_#{name.gsub(/\W+/, "_")}") do
          skip "behavioral simulator not installed" unless HAS_BEHAVIORAL

          behavioral = Intel4004Simulator::Simulator.new
          gate_level = Intel4004GateLevel.new

          b_traces = behavioral.run(program)
          g_traces = gate_level.run(program)

          # Same number of instructions executed
          assert_equal b_traces.length, g_traces.length,
            "[#{name}] trace length: behavioral=#{b_traces.length}, gate-level=#{g_traces.length}"

          # Same final register state
          16.times do |i|
            b_val = behavioral.registers[i]
            g_val = gate_level.registers[i]
            assert_equal b_val, g_val,
              "[#{name}] R#{i}: behavioral=#{b_val}, gate-level=#{g_val}"
          end

          # Same accumulator and carry
          assert_equal behavioral.accumulator, gate_level.accumulator,
            "[#{name}] A: behavioral=#{behavioral.accumulator}, gate-level=#{gate_level.accumulator}"
          assert_equal behavioral.carry, gate_level.carry,
            "[#{name}] carry: behavioral=#{behavioral.carry}, gate-level=#{gate_level.carry}"

          # Same mnemonic trace
          b_traces.zip(g_traces).each_with_index do |(bt, gt), i|
            assert_equal bt.mnemonic, gt.mnemonic,
              "[#{name}] step #{i}: behavioral=#{bt.mnemonic}, gate-level=#{gt.mnemonic}"
          end
        end
      end
    end
  end
end
