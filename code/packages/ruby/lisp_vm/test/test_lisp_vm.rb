# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_lisp_vm"

# ================================================================
# Tests for the Lisp VM
# ================================================================
#
# Rather than testing the full compiler+VM pipeline (that is tested
# in lisp_compiler), these tests construct CodeObject instructions
# directly and verify that each opcode works as expected.
# ================================================================

VM_MOD = CodingAdventures::VirtualMachine
GC_LVM = CodingAdventures::GarbageCollector
LOP = CodingAdventures::LispVm::LispOp
NIL_VAL = CodingAdventures::LispVm::NIL

class TestLispVm < Minitest::Test
  def setup
    @gc = GC_LVM::MarkAndSweepGC.new
    @vm = CodingAdventures::LispVm.create_lisp_vm(gc: @gc)
  end

  def make_code(instructions, constants: [], names: [])
    VM_MOD::CodeObject.new(
      instructions: instructions,
      constants:    constants,
      names:        names
    )
  end

  def instr(opcode, operand = 0)
    VM_MOD::Instruction.new(opcode: opcode, operand: operand)
  end

  def test_version_exists
    refute_nil CodingAdventures::LispVm::VERSION
  end

  # ------------------------------------------------------------------
  # LOAD_CONST
  # ------------------------------------------------------------------

  def test_load_const_pushes_value
    code = make_code([instr(LOP::LOAD_CONST, 0), instr(LOP::HALT)], constants: [42])
    @vm.execute(code)
    assert_equal 42, @vm.stack.last
  end

  # ------------------------------------------------------------------
  # LOAD_NIL
  # ------------------------------------------------------------------

  def test_load_nil_pushes_nil_sentinel
    code = make_code([instr(LOP::LOAD_NIL), instr(LOP::HALT)])
    @vm.execute(code)
    assert_equal NIL_VAL, @vm.stack.last
  end

  # ------------------------------------------------------------------
  # LOAD_TRUE
  # ------------------------------------------------------------------

  def test_load_true_pushes_true
    code = make_code([instr(LOP::LOAD_TRUE), instr(LOP::HALT)])
    @vm.execute(code)
    assert_equal true, @vm.stack.last
  end

  # ------------------------------------------------------------------
  # Arithmetic
  # ------------------------------------------------------------------

  def test_add
    code = make_code([
      instr(LOP::LOAD_CONST, 0), instr(LOP::LOAD_CONST, 1),
      instr(LOP::ADD), instr(LOP::HALT)
    ], constants: [3, 4])
    @vm.execute(code)
    assert_equal 7, @vm.stack.last
  end

  def test_sub
    code = make_code([
      instr(LOP::LOAD_CONST, 0), instr(LOP::LOAD_CONST, 1),
      instr(LOP::SUB), instr(LOP::HALT)
    ], constants: [10, 3])
    @vm.execute(code)
    assert_equal 7, @vm.stack.last
  end

  def test_mul
    code = make_code([
      instr(LOP::LOAD_CONST, 0), instr(LOP::LOAD_CONST, 1),
      instr(LOP::MUL), instr(LOP::HALT)
    ], constants: [6, 7])
    @vm.execute(code)
    assert_equal 42, @vm.stack.last
  end

  # ------------------------------------------------------------------
  # Comparison
  # ------------------------------------------------------------------

  def test_cmp_eq_true
    code = make_code([
      instr(LOP::LOAD_CONST, 0), instr(LOP::LOAD_CONST, 0),
      instr(LOP::CMP_EQ), instr(LOP::HALT)
    ], constants: [5])
    @vm.execute(code)
    assert_equal true, @vm.stack.last
  end

  def test_cmp_eq_false
    code = make_code([
      instr(LOP::LOAD_CONST, 0), instr(LOP::LOAD_CONST, 1),
      instr(LOP::CMP_EQ), instr(LOP::HALT)
    ], constants: [3, 5])
    @vm.execute(code)
    assert_equal NIL_VAL, @vm.stack.last
  end

  # ------------------------------------------------------------------
  # CONS / CAR / CDR
  # ------------------------------------------------------------------

  def test_cons_creates_heap_object
    code = make_code([
      instr(LOP::LOAD_CONST, 0), instr(LOP::LOAD_NIL),
      instr(LOP::CONS), instr(LOP::HALT)
    ], constants: [42])
    @vm.execute(code)
    addr = @vm.stack.last
    assert addr.is_a?(Integer)
    assert @gc.valid_address?(addr)
    cell = @gc.deref(addr)
    assert_equal 42, cell.car
    assert_equal NIL_VAL, cell.cdr
  end

  def test_car_extracts_head
    code = make_code([
      instr(LOP::LOAD_CONST, 0), instr(LOP::LOAD_NIL),
      instr(LOP::CONS), instr(LOP::CAR), instr(LOP::HALT)
    ], constants: [99])
    @vm.execute(code)
    assert_equal 99, @vm.stack.last
  end

  def test_cdr_extracts_tail
    code = make_code([
      instr(LOP::LOAD_CONST, 0), instr(LOP::LOAD_NIL),
      instr(LOP::CONS), instr(LOP::CDR), instr(LOP::HALT)
    ], constants: [99])
    @vm.execute(code)
    assert_equal NIL_VAL, @vm.stack.last
  end

  # ------------------------------------------------------------------
  # IS_NIL / IS_ATOM
  # ------------------------------------------------------------------

  def test_is_nil_true
    code = make_code([instr(LOP::LOAD_NIL), instr(LOP::IS_NIL), instr(LOP::HALT)])
    @vm.execute(code)
    assert_equal true, @vm.stack.last
  end

  def test_is_nil_false
    code = make_code([instr(LOP::LOAD_CONST, 0), instr(LOP::IS_NIL), instr(LOP::HALT)],
                     constants: [42])
    @vm.execute(code)
    assert_equal NIL_VAL, @vm.stack.last
  end

  # ------------------------------------------------------------------
  # STORE_NAME / LOAD_NAME
  # ------------------------------------------------------------------

  def test_store_and_load_name
    code = make_code([
      instr(LOP::LOAD_CONST, 0),
      instr(LOP::STORE_NAME, 0),
      instr(LOP::LOAD_NAME, 0),
      instr(LOP::HALT)
    ], constants: [42], names: ["x"])
    @vm.execute(code)
    assert_equal 42, @vm.stack.last
    assert_equal 42, @vm.variables["x"]
  end

  # ------------------------------------------------------------------
  # PRINT
  # ------------------------------------------------------------------

  def test_print_adds_to_output
    code = make_code([instr(LOP::LOAD_CONST, 0), instr(LOP::PRINT), instr(LOP::HALT)],
                     constants: [42])
    @vm.execute(code)
    assert_equal ["42"], @vm.output
  end

  # ------------------------------------------------------------------
  # Jump
  # ------------------------------------------------------------------

  def test_jump_if_false_jumps_on_nil
    # Jump over the LOAD_CONST 99 when condition is NIL
    code = make_code([
      instr(LOP::LOAD_NIL),                   # 0: push NIL (falsy)
      instr(LOP::JUMP_IF_FALSE, 3),            # 1: jump to 3 (past 99)
      instr(LOP::LOAD_CONST, 0),               # 2: push 99 (should skip)
      instr(LOP::LOAD_CONST, 1),               # 3: push 42
      instr(LOP::HALT)                         # 4
    ], constants: [99, 42])
    @vm.execute(code)
    assert_equal 42, @vm.stack.last
  end
end
