# frozen_string_literal: true

require_relative "test_helper"

# Tests for MockDecoder -- the simple ISA decoder for testing.
class TestMockDecoderProtocol < Minitest::Test
  def test_instruction_size
    d = CodingAdventures::Core::MockDecoder.new
    assert_equal 4, d.instruction_size
  end
end

class TestMockDecoderDecode < Minitest::Test
  def setup
    @d = CodingAdventures::Core::MockDecoder.new
  end

  def decode(raw)
    token = CodingAdventures::CpuPipeline.new_token
    @d.decode(raw, token)
  end

  def test_nop
    tok = decode(CodingAdventures::Core.encode_nop)
    assert_equal "NOP", tok.opcode
    assert_equal(-1, tok.rd)
    assert_equal(-1, tok.rs1)
    assert_equal(-1, tok.rs2)
  end

  def test_add
    tok = decode(CodingAdventures::Core.encode_add(3, 1, 2))
    assert_equal "ADD", tok.opcode
    assert_equal 3, tok.rd
    assert_equal 1, tok.rs1
    assert_equal 2, tok.rs2
    assert tok.reg_write
  end

  def test_sub
    tok = decode(CodingAdventures::Core.encode_sub(3, 1, 2))
    assert_equal "SUB", tok.opcode
    assert_equal 3, tok.rd
    assert_equal 1, tok.rs1
    assert_equal 2, tok.rs2
    assert tok.reg_write
  end

  def test_addi
    tok = decode(CodingAdventures::Core.encode_addi(1, 0, 42))
    assert_equal "ADDI", tok.opcode
    assert_equal 1, tok.rd
    assert_equal 0, tok.rs1
    assert_equal(-1, tok.rs2)
    assert tok.reg_write
  end

  def test_load
    tok = decode(CodingAdventures::Core.encode_load(1, 2, 100))
    assert_equal "LOAD", tok.opcode
    assert_equal 1, tok.rd
    assert_equal 2, tok.rs1
    assert_equal(-1, tok.rs2)
    assert tok.reg_write
    assert tok.mem_read
  end

  def test_store
    tok = decode(CodingAdventures::Core.encode_store(2, 3, 100))
    assert_equal "STORE", tok.opcode
    assert_equal(-1, tok.rd)
    assert_equal 2, tok.rs1
    assert_equal 3, tok.rs2
    assert tok.mem_write
  end

  def test_branch
    tok = decode(CodingAdventures::Core.encode_branch(1, 2, 4))
    assert_equal "BRANCH", tok.opcode
    assert_equal(-1, tok.rd)
    assert_equal 1, tok.rs1
    assert_equal 2, tok.rs2
    assert tok.is_branch
  end

  def test_halt
    tok = decode(CodingAdventures::Core.encode_halt)
    assert_equal "HALT", tok.opcode
    assert_equal(-1, tok.rd)
    assert_equal(-1, tok.rs1)
    assert_equal(-1, tok.rs2)
    assert tok.is_halt
  end

  def test_unknown_opcode
    tok = decode(0xFF << 24)
    assert_equal "NOP", tok.opcode
  end
end

class TestMockDecoderExecute < Minitest::Test
  def setup
    @d = CodingAdventures::Core::MockDecoder.new
    @reg_file = CodingAdventures::Core::RegisterFile.new
    @reg_file.write(1, 10)
    @reg_file.write(2, 20)
  end

  def decode_and_execute(raw, pc: 0)
    token = CodingAdventures::CpuPipeline.new_token
    token.pc = pc
    @d.decode(raw, token)
    @d.execute(token, @reg_file)
    token
  end

  def test_add_execute
    tok = decode_and_execute(CodingAdventures::Core.encode_add(3, 1, 2))
    assert_equal 30, tok.alu_result
  end

  def test_sub_execute
    tok = decode_and_execute(CodingAdventures::Core.encode_sub(3, 2, 1))
    assert_equal 10, tok.alu_result
  end

  def test_addi_execute
    tok = decode_and_execute(CodingAdventures::Core.encode_addi(3, 1, 5))
    assert_equal 15, tok.alu_result
  end

  def test_load_effective_address
    tok = decode_and_execute(CodingAdventures::Core.encode_load(3, 1, 100))
    assert_equal 110, tok.alu_result
  end

  def test_branch_taken
    tok = decode_and_execute(CodingAdventures::Core.encode_branch(1, 1, 3), pc: 100)
    assert tok.branch_taken, "branch should be taken (Rs1 == Rs1)"
    assert_equal 100 + 3 * 4, tok.branch_target
  end

  def test_branch_not_taken
    tok = decode_and_execute(CodingAdventures::Core.encode_branch(1, 2, 3), pc: 100)
    refute tok.branch_taken, "branch should not be taken (10 != 20)"
  end
end

class TestInstructionEncoding < Minitest::Test
  def test_encode_program
    program = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 42),
      CodingAdventures::Core.encode_halt
    )
    assert_equal 8, program.length
    # Each instruction is 4 bytes.
    assert program.all? { |b| b.is_a?(Integer) && b >= 0 && b <= 255 }
  end
end
