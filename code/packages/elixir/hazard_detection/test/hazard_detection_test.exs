defmodule CodingAdventures.HazardDetectionTest do
  use ExUnit.Case

  alias CodingAdventures.HazardDetection
  alias CodingAdventures.HazardDetection.ControlHazardDetector
  alias CodingAdventures.HazardDetection.DataHazardDetector
  alias CodingAdventures.HazardDetection.HazardUnit
  alias CodingAdventures.HazardDetection.StructuralHazardDetector

  defp slot(attrs), do: HazardDetection.slot(attrs)
  defp empty_slot, do: HazardDetection.empty_slot()

  test "data detector forwards from ex and stalls on load use" do
    result =
      DataHazardDetector.detect(
        slot(valid: true, source_regs: [1, 5], uses_alu: false),
        slot(valid: true, dest_reg: 1, dest_value: 42, uses_alu: false),
        empty_slot()
      )

    assert result.action == :forward_ex
    assert result.forwarded_value == 42
    assert result.forwarded_from == "EX"

    result =
      DataHazardDetector.detect(
        slot(valid: true, source_regs: [1], uses_alu: false),
        slot(valid: true, dest_reg: 1, mem_read: true, uses_alu: false),
        empty_slot()
      )

    assert result.action == :stall
    assert result.stall_cycles == 1
  end

  test "data detector handles mem forwarding and ex priority" do
    result =
      DataHazardDetector.detect(
        slot(valid: true, source_regs: [1], uses_alu: false),
        empty_slot(),
        slot(valid: true, dest_reg: 1, dest_value: 99, uses_alu: false)
      )

    assert result.action == :forward_mem
    assert result.forwarded_value == 99

    result =
      DataHazardDetector.detect(
        slot(valid: true, source_regs: [1], uses_alu: false),
        slot(valid: true, dest_reg: 1, dest_value: 10, uses_alu: false),
        slot(valid: true, dest_reg: 1, dest_value: 20, uses_alu: false)
      )

    assert result.action == :forward_ex
    assert result.forwarded_value == 10
  end

  test "control detector flushes mispredicted branches" do
    result =
      ControlHazardDetector.detect(
        slot(
          valid: true,
          is_branch: true,
          pc: 0x100,
          branch_taken: true,
          branch_predicted_taken: false,
          uses_alu: false
        )
      )

    assert result.action == :flush
    assert result.flush_count == 2
    assert result.reason =~ "not-taken, actually taken"

    result =
      ControlHazardDetector.detect(
        slot(
          valid: true,
          is_branch: true,
          pc: 0x200,
          branch_taken: false,
          branch_predicted_taken: true,
          uses_alu: false
        )
      )

    assert result.action == :flush
    assert result.reason =~ "taken, actually not-taken"
  end

  test "control detector returns none for non-branches and correct predictions" do
    result = ControlHazardDetector.detect(empty_slot())
    assert result.action == :none

    result = ControlHazardDetector.detect(slot(valid: true, is_branch: false, uses_alu: false))
    assert result.action == :none

    result =
      ControlHazardDetector.detect(
        slot(valid: true, is_branch: true, branch_taken: true, branch_predicted_taken: true, uses_alu: false)
      )

    assert result.action == :none
  end

  test "structural detector catches alu fp and memory conflicts" do
    detector = StructuralHazardDetector.new(num_alus: 1)

    result =
      StructuralHazardDetector.detect(
        detector,
        slot(valid: true, pc: 0x10, uses_alu: true),
        slot(valid: true, pc: 0x14, uses_alu: true)
      )

    assert result.action == :stall

    detector = StructuralHazardDetector.new(num_fp_units: 1)

    result =
      StructuralHazardDetector.detect(
        detector,
        slot(valid: true, pc: 0x10, uses_alu: false, uses_fp: true),
        slot(valid: true, pc: 0x14, uses_alu: false, uses_fp: true)
      )

    assert result.action == :stall

    detector = StructuralHazardDetector.new(split_caches: false)

    result =
      StructuralHazardDetector.detect(
        detector,
        slot(valid: true, uses_alu: false),
        slot(valid: true, uses_alu: false),
        if_stage: slot(valid: true, pc: 0x10, uses_alu: false),
        mem_stage: slot(valid: true, pc: 0x04, mem_read: true, uses_alu: false)
      )

    assert result.action == :stall
  end

  test "structural detector allows enough hardware and split caches" do
    detector = StructuralHazardDetector.new(num_alus: 2, num_fp_units: 2, split_caches: true)

    result =
      StructuralHazardDetector.detect(
        detector,
        slot(valid: true, uses_alu: true),
        slot(valid: true, uses_alu: true),
        if_stage: slot(valid: true, uses_alu: false),
        mem_stage: slot(valid: true, mem_read: true, uses_alu: false)
      )

    assert result.action == :none
  end

  test "hazard unit applies flush then stall then forwarding priority" do
    unit = HazardUnit.new(num_alus: 2)

    {unit, result} =
      HazardUnit.check(
        unit,
        slot(valid: true, uses_alu: false),
        slot(valid: true, source_regs: [1], uses_alu: false),
        slot(valid: true, dest_reg: 1, dest_value: 42, uses_alu: false),
        empty_slot()
      )

    assert result.action == :forward_ex

    {unit, result} =
      HazardUnit.check(
        unit,
        slot(valid: true, uses_alu: false),
        slot(valid: true, source_regs: [1], uses_alu: false),
        slot(valid: true, dest_reg: 1, mem_read: true, uses_alu: false),
        empty_slot()
      )

    assert result.action == :stall

    {unit, result} =
      HazardUnit.check(
        unit,
        slot(valid: true, uses_alu: false),
        slot(valid: true, source_regs: [1], uses_alu: false),
        slot(
          valid: true,
          dest_reg: 1,
          dest_value: 42,
          is_branch: true,
          branch_taken: true,
          branch_predicted_taken: false,
          uses_alu: false
        ),
        empty_slot()
      )

    assert result.action == :flush
    assert length(unit.history) == 3
    assert HazardUnit.stall_count(unit) == 1
    assert HazardUnit.flush_count(unit) == 1
    assert HazardUnit.forward_count(unit) == 1
  end

  test "hazard unit can surface structural stalls and all-empty none" do
    unit = HazardUnit.new(num_alus: 1)

    {_unit, result} =
      HazardUnit.check(
        unit,
        slot(valid: true, uses_alu: false),
        slot(valid: true, source_regs: [], uses_alu: true),
        slot(valid: true, dest_reg: 5, uses_alu: true),
        empty_slot()
      )

    assert result.action == :stall

    {unit, result} = HazardUnit.check(HazardUnit.new(), empty_slot(), empty_slot(), empty_slot(), empty_slot())
    assert result.action == :none
    assert length(unit.history) == 1
  end
end
