# frozen_string_literal: true

require "test_helper"

class TestBootToHelloWorld < Minitest::Test
  def test_hello_world_on_display
    board = CodingAdventures::SystemBoard::Board.new(CodingAdventures::SystemBoard.default_system_config)
    board.power_on
    board.run(100_000)
    snap = board.display_snapshot
    refute_nil snap
    assert snap.contains("Hello World")
  end

  def test_idle_after_hello_world
    board = CodingAdventures::SystemBoard::Board.new(CodingAdventures::SystemBoard.default_system_config)
    board.power_on
    board.run(100_000)
    assert board.idle?
  end
end

class TestPowerOn < Minitest::Test
  def test_new_board
    board = CodingAdventures::SystemBoard::Board.new(CodingAdventures::SystemBoard.default_system_config)
    refute board.powered
  end

  def test_power_on
    board = CodingAdventures::SystemBoard::Board.new(CodingAdventures::SystemBoard.default_system_config)
    board.power_on
    assert board.powered
    refute_nil board.cpu
    refute_nil board.display
    refute_nil board.kernel
  end

  def test_double_power_on
    board = CodingAdventures::SystemBoard::Board.new(CodingAdventures::SystemBoard.default_system_config)
    board.power_on
    board.power_on
    assert board.powered
  end
end

class TestBootPhases < Minitest::Test
  def test_phases_present
    board = CodingAdventures::SystemBoard::Board.new(CodingAdventures::SystemBoard.default_system_config)
    board.power_on
    board.run(100_000)
    phases = board.trace.phases
    assert phases.length > 0
    assert_includes phases, CodingAdventures::SystemBoard::PHASE_POWER_ON
    assert_includes phases, CodingAdventures::SystemBoard::PHASE_BIOS
  end

  def test_trace_has_events
    board = CodingAdventures::SystemBoard::Board.new(CodingAdventures::SystemBoard.default_system_config)
    board.power_on
    board.run(100_000)
    assert board.trace.events.length > 0
  end

  def test_total_cycles
    board = CodingAdventures::SystemBoard::Board.new(CodingAdventures::SystemBoard.default_system_config)
    board.power_on
    board.run(100_000)
    assert board.trace.total_cycles > 0
  end

  def test_phase_start_cycle
    board = CodingAdventures::SystemBoard::Board.new(CodingAdventures::SystemBoard.default_system_config)
    board.power_on
    board.run(100_000)
    assert_equal 0, board.trace.phase_start_cycle(CodingAdventures::SystemBoard::PHASE_POWER_ON)
    assert_equal(-1, board.trace.phase_start_cycle(99))
  end

  def test_phase_names
    include CodingAdventures::SystemBoard
    assert_equal "PowerOn", PHASE_NAMES[PHASE_POWER_ON]
    assert_equal "Idle", PHASE_NAMES[PHASE_IDLE]
  end
end

class TestKeystroke < Minitest::Test
  def test_inject_keystroke
    board = CodingAdventures::SystemBoard::Board.new(CodingAdventures::SystemBoard.default_system_config)
    board.power_on
    board.run(100_000)
    board.inject_keystroke(65) # 'A'
    assert_equal 1, board.kernel.keyboard_buffer.length
    assert_equal 65, board.kernel.keyboard_buffer[0]
  end
end

class TestCycleCount < Minitest::Test
  def test_positive_cycles
    board = CodingAdventures::SystemBoard::Board.new(CodingAdventures::SystemBoard.default_system_config)
    board.power_on
    board.run(100_000)
    assert board.cycle > 0
  end

  def test_step_before_power_on
    board = CodingAdventures::SystemBoard::Board.new(CodingAdventures::SystemBoard.default_system_config)
    board.step
    assert_equal 0, board.cycle
  end
end
