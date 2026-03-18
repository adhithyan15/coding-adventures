# frozen_string_literal: true

require "test_helper"

# Use a short alias for the module to avoid confusion with the class
WS = CodingAdventures::WasmSimulator

class TestWasmEncoding < Minitest::Test
  def test_encode_i32_const
    bytes = WS.encode_i32_const(1)
    assert_equal "\x41\x01\x00\x00\x00".b, bytes
  end

  def test_encode_i32_const_negative
    bytes = WS.encode_i32_const(-1)
    assert_equal "\x41\xFF\xFF\xFF\xFF".b, bytes
  end

  def test_encode_i32_add
    assert_equal "\x6A".b, WS.encode_i32_add
  end

  def test_encode_i32_sub
    assert_equal "\x6B".b, WS.encode_i32_sub
  end

  def test_encode_local_get
    assert_equal "\x20\x00".b, WS.encode_local_get(0)
  end

  def test_encode_local_set
    assert_equal "\x21\x00".b, WS.encode_local_set(0)
  end

  def test_encode_end
    assert_equal "\x0B".b, WS.encode_end
  end

  def test_assemble_wasm
    program = WS.assemble_wasm([
      WS.encode_i32_const(1),
      WS.encode_end
    ])
    assert_equal 6, program.bytesize
  end
end

class TestWasmDecoder < Minitest::Test
  def setup
    @decoder = WS::WasmDecoder.new
  end

  def test_decode_i32_const
    bytecode = WS.encode_i32_const(42)
    instr = @decoder.decode(bytecode, 0)
    assert_equal "i32.const", instr.mnemonic
    assert_equal 42, instr.operand
    assert_equal 5, instr.size
  end

  def test_decode_i32_add
    bytecode = "\x6A".b
    instr = @decoder.decode(bytecode, 0)
    assert_equal "i32.add", instr.mnemonic
    assert_nil instr.operand
    assert_equal 1, instr.size
  end

  def test_decode_i32_sub
    bytecode = "\x6B".b
    instr = @decoder.decode(bytecode, 0)
    assert_equal "i32.sub", instr.mnemonic
  end

  def test_decode_local_get
    bytecode = "\x20\x02".b
    instr = @decoder.decode(bytecode, 0)
    assert_equal "local.get", instr.mnemonic
    assert_equal 2, instr.operand
    assert_equal 2, instr.size
  end

  def test_decode_local_set
    bytecode = "\x21\x03".b
    instr = @decoder.decode(bytecode, 0)
    assert_equal "local.set", instr.mnemonic
    assert_equal 3, instr.operand
  end

  def test_decode_end
    bytecode = "\x0B".b
    instr = @decoder.decode(bytecode, 0)
    assert_equal "end", instr.mnemonic
  end

  def test_decode_unknown
    assert_raises(ArgumentError) { @decoder.decode("\xFF".b, 0) }
  end
end

class TestWasmExecutor < Minitest::Test
  def setup
    @executor = WS::WasmExecutor.new
  end

  def test_execute_unknown
    instr = WS::WasmInstruction.new(opcode: 0xFF, mnemonic: "nope", operand: nil, size: 1)
    assert_raises(ArgumentError) { @executor.execute(instr, [], [0], 0) }
  end
end

class TestWasmSimulator < Minitest::Test
  def test_x_equals_1_plus_2
    sim = WS::WasmSimulator.new(num_locals: 4)
    program = WS.assemble_wasm([
      WS.encode_i32_const(1),
      WS.encode_i32_const(2),
      WS.encode_i32_add,
      WS.encode_local_set(0),
      WS.encode_end
    ])
    traces = sim.run(program)
    assert_equal 5, traces.size
    assert_equal 3, sim.locals[0]
  end

  def test_subtraction
    sim = WS::WasmSimulator.new
    program = WS.assemble_wasm([
      WS.encode_i32_const(5),
      WS.encode_i32_const(3),
      WS.encode_i32_sub,
      WS.encode_local_set(0),
      WS.encode_end
    ])
    sim.run(program)
    assert_equal 2, sim.locals[0]
  end

  def test_local_get_and_set
    sim = WS::WasmSimulator.new(num_locals: 4)
    program = WS.assemble_wasm([
      WS.encode_i32_const(42),
      WS.encode_local_set(0),
      WS.encode_local_get(0),
      WS.encode_local_set(1),
      WS.encode_end
    ])
    sim.run(program)
    assert_equal 42, sim.locals[0]
    assert_equal 42, sim.locals[1]
  end

  def test_halted_raises
    sim = WS::WasmSimulator.new
    program = WS.assemble_wasm([WS.encode_end])
    sim.run(program)
    assert_raises(RuntimeError) { sim.step }
  end

  def test_step_by_step
    sim = WS::WasmSimulator.new
    sim.load(WS.assemble_wasm([
      WS.encode_i32_const(10),
      WS.encode_end
    ]))
    trace = sim.step
    assert_equal "i32.const", trace.instruction.mnemonic
    assert_equal [10], sim.stack
  end

  def test_trace_immutability
    trace = WS::WasmStepTrace.new(
      pc: 0, instruction: WS::WasmInstruction.new(opcode: 0, mnemonic: "nop", operand: nil, size: 1),
      stack_before: [], stack_after: [], locals_snapshot: [],
      description: "nop"
    )
    assert_equal false, trace.halted
  end
end
