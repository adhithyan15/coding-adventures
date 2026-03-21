# frozen_string_literal: true

require "test_helper"
require "coding_adventures_display"
require "coding_adventures/interrupt_handler"

class MockRegAccess
  attr_accessor :regs
  def initialize = @regs = {}
  def read_register(index) = @regs.fetch(index, 0)
  def write_register(index, value) = @regs[index] = value
end

class MockMemAccess
  def initialize(data = {}) = @data = data
  def read_memory_byte(addr) = @data.fetch(addr, 0)
end

class TestProcessManagement < Minitest::Test
  include CodingAdventures::OsKernel

  def test_create_process
    k = Kernel.new(CodingAdventures::OsKernel.default_kernel_config)
    pid = k.create_process("test", "", 0x40000, 0x10000)
    assert_equal 0, pid
    assert_equal 1, k.process_table.length
  end

  def test_max_processes
    cfg = KernelConfig.new(max_processes: 2)
    k = Kernel.new(cfg)
    k.create_process("p0", "", 0x30000, 0x10000)
    k.create_process("p1", "", 0x40000, 0x10000)
    assert_equal(-1, k.create_process("p2", "", 0x50000, 0x10000))
  end
end

class TestScheduler < Minitest::Test
  include CodingAdventures::OsKernel

  def test_round_robin
    procs = [
      ProcessControlBlock.new(pid: 0, state: PROCESS_READY, name: "idle"),
      ProcessControlBlock.new(pid: 1, state: PROCESS_READY, name: "hello")
    ]
    sched = Scheduler.new(procs)
    sched.current = 0
    assert_equal 1, sched.schedule
  end

  def test_skip_terminated
    procs = [
      ProcessControlBlock.new(pid: 0, state: PROCESS_READY, name: "idle"),
      ProcessControlBlock.new(pid: 1, state: PROCESS_TERMINATED, name: "hello")
    ]
    procs[1].state = PROCESS_TERMINATED
    sched = Scheduler.new(procs)
    sched.current = 0
    assert_equal 0, sched.schedule
  end

  def test_context_switch
    procs = [
      ProcessControlBlock.new(pid: 0, state: PROCESS_RUNNING, name: "idle"),
      ProcessControlBlock.new(pid: 1, state: PROCESS_READY, name: "hello")
    ]
    procs[0].state = PROCESS_RUNNING
    sched = Scheduler.new(procs)
    sched.context_switch(0, 1)
    assert_equal PROCESS_READY, procs[0].state
    assert_equal PROCESS_RUNNING, procs[1].state
  end
end

class TestMemoryManager < Minitest::Test
  include CodingAdventures::OsKernel

  def test_find_region
    mm = MemoryManager.new([
      MemoryRegion.new(base: 0x1000, size: 0x1000, permissions: PERM_READ, owner: -1, name: "A")
    ])
    r = mm.find_region(0x1500)
    assert_equal "A", r.name
    assert_nil mm.find_region(0x6000)
  end

  def test_check_access
    mm = MemoryManager.new([
      MemoryRegion.new(base: 0x1000, size: 0x1000, permissions: PERM_READ | PERM_WRITE, owner: -1, name: "K"),
      MemoryRegion.new(base: 0x3000, size: 0x1000, permissions: PERM_READ, owner: 1, name: "P1")
    ])
    assert mm.check_access(0, 0x1000, PERM_READ)
    refute mm.check_access(0, 0x3000, PERM_READ)
  end
end

class TestSyscalls < Minitest::Test
  include CodingAdventures::OsKernel

  def new_booted_kernel
    ic = CodingAdventures::InterruptHandler::InterruptController.new
    k = Kernel.new(CodingAdventures::OsKernel.default_kernel_config, ic)
    k.boot
    k
  end

  def test_sys_exit
    k = new_booted_kernel
    regs = MockRegAccess.new
    regs.regs[REG_A0] = 42
    k.current_process = 1
    k.process_table[1].state = PROCESS_RUNNING
    k.handle_syscall(SYS_EXIT, regs, MockMemAccess.new)
    assert_equal PROCESS_TERMINATED, k.process_table[1].state
    assert_equal 42, k.process_table[1].exit_code
  end

  def test_sys_yield
    k = new_booted_kernel
    k.current_process = 1
    k.process_table[1].state = PROCESS_RUNNING
    k.handle_syscall(SYS_YIELD, MockRegAccess.new, MockMemAccess.new)
    assert_equal PROCESS_READY, k.process_table[1].state
  end

  def test_unknown_syscall
    k = new_booted_kernel
    k.current_process = 1
    k.process_table[1].state = PROCESS_RUNNING
    ok = k.handle_syscall(99, MockRegAccess.new, MockMemAccess.new)
    refute ok
    assert_equal PROCESS_TERMINATED, k.process_table[1].state
  end
end

class TestKernelBoot < Minitest::Test
  include CodingAdventures::OsKernel

  def test_boot
    ic = CodingAdventures::InterruptHandler::InterruptController.new
    k = Kernel.new(CodingAdventures::OsKernel.default_kernel_config, ic)
    k.boot
    assert k.booted
    assert_equal 2, k.process_count
    assert_equal "idle", k.process_table[0].name
    assert_equal "hello-world", k.process_table[1].name
    assert_equal 1, k.current_process
    assert ic.registry.has_handler?(INTERRUPT_TIMER)
  end

  def test_is_idle
    ic = CodingAdventures::InterruptHandler::InterruptController.new
    k = Kernel.new(CodingAdventures::OsKernel.default_kernel_config, ic)
    k.boot
    refute k.idle?
    k.process_table[1].state = PROCESS_TERMINATED
    assert k.idle?
  end
end

class TestPrograms < Minitest::Test
  include CodingAdventures::OsKernel

  def test_idle_program
    binary = Programs.generate_idle_program
    assert binary.length > 0
    assert_equal 0, binary.length % 4
  end

  def test_hello_world_program
    binary = Programs.generate_hello_world_program(0x00040000)
    assert binary.length > 0
    message = "Hello World\n"
    found = binary[0x100, message.length]
    assert_equal message, found
  end
end
