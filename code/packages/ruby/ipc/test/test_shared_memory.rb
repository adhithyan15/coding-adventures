# frozen_string_literal: true

require_relative "test_helper"

# Tests for the SharedMemoryRegion class -- zero-copy shared data regions.
#
# We test:
#   1. Creation with correct name, size, owner
#   2. Attach and detach processes
#   3. Read and write at various offsets
#   4. Bounds checking (out-of-range access raises error)
#   5. Multiple PIDs attached simultaneously
#   6. Data visibility across "processes" (shared backing array)
#   7. Byte masking on write
class TestSharedMemory < Minitest::Test
  # -- Creation --

  def test_creation
    shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
      name: "test_region", size: 1024, owner_pid: 1
    )

    assert_equal "test_region", shm.name
    assert_equal 1024, shm.size
    assert_equal 1, shm.owner_pid
    assert_empty shm.attached_pids
  end

  def test_initial_data_is_zeroed
    shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
      name: "test", size: 8, owner_pid: 1
    )

    result = shm.read(0, 8)
    assert_equal [0, 0, 0, 0, 0, 0, 0, 0], result
  end

  # -- Attach/Detach --

  def test_attach_adds_pid
    shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
      name: "test", size: 64, owner_pid: 1
    )

    shm.attach(1)
    shm.attach(2)

    assert_includes shm.attached_pids, 1
    assert_includes shm.attached_pids, 2
    assert_equal 2, shm.attached_pids.size
  end

  def test_detach_removes_pid
    shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
      name: "test", size: 64, owner_pid: 1
    )

    shm.attach(1)
    shm.attach(2)
    shm.detach(1)

    refute_includes shm.attached_pids, 1
    assert_includes shm.attached_pids, 2
  end

  def test_attach_same_pid_twice_is_idempotent
    shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
      name: "test", size: 64, owner_pid: 1
    )

    shm.attach(1)
    shm.attach(1)

    assert_equal 1, shm.attached_pids.size
  end

  # -- Read/Write --

  def test_write_and_read
    shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
      name: "test", size: 64, owner_pid: 1
    )

    shm.write(0, [72, 101, 108, 108, 111]) # "Hello"
    result = shm.read(0, 5)
    assert_equal [72, 101, 108, 108, 111], result
  end

  def test_write_at_offset
    shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
      name: "test", size: 64, owner_pid: 1
    )

    shm.write(10, [1, 2, 3])
    result = shm.read(10, 3)
    assert_equal [1, 2, 3], result

    # Data before offset should still be zero.
    result = shm.read(0, 3)
    assert_equal [0, 0, 0], result
  end

  def test_overwrite_existing_data
    shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
      name: "test", size: 64, owner_pid: 1
    )

    shm.write(0, [1, 2, 3])
    shm.write(0, [4, 5, 6])
    result = shm.read(0, 3)
    assert_equal [4, 5, 6], result
  end

  # -- Bounds checking --

  def test_read_out_of_bounds_raises
    shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
      name: "test", size: 8, owner_pid: 1
    )

    assert_raises(CodingAdventures::Ipc::SharedMemoryBoundsError) do
      shm.read(5, 10) # 5 + 10 = 15 > 8
    end
  end

  def test_write_out_of_bounds_raises
    shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
      name: "test", size: 8, owner_pid: 1
    )

    assert_raises(CodingAdventures::Ipc::SharedMemoryBoundsError) do
      shm.write(5, [1, 2, 3, 4, 5]) # 5 + 5 = 10 > 8
    end
  end

  def test_negative_offset_raises
    shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
      name: "test", size: 8, owner_pid: 1
    )

    assert_raises(CodingAdventures::Ipc::SharedMemoryBoundsError) do
      shm.read(-1, 1)
    end
  end

  def test_read_at_exact_boundary_succeeds
    shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
      name: "test", size: 8, owner_pid: 1
    )

    shm.write(0, [1, 2, 3, 4, 5, 6, 7, 8])
    result = shm.read(0, 8) # exactly at boundary
    assert_equal [1, 2, 3, 4, 5, 6, 7, 8], result
  end

  # -- Multi-process visibility --

  def test_shared_data_visible_to_multiple_pids
    # This test verifies the "shared" part of shared memory.
    # In a real OS, two processes mapping the same physical pages would
    # see each other's writes. In our simulation, both "processes" access
    # the same SharedMemoryRegion object, so writes by "process 1" are
    # visible to "process 2."
    shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
      name: "shared", size: 64, owner_pid: 1
    )

    shm.attach(1)
    shm.attach(2)

    # "Process 1" writes.
    shm.write(0, [42, 43, 44])

    # "Process 2" reads -- should see the same data.
    result = shm.read(0, 3)
    assert_equal [42, 43, 44], result
  end

  # -- Byte masking --

  def test_write_masks_bytes
    shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
      name: "test", size: 8, owner_pid: 1
    )

    shm.write(0, [256, -1])
    result = shm.read(0, 2)
    assert_equal [0, 255], result
  end

  # -- Read returns a copy --

  def test_read_returns_independent_copy
    shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
      name: "test", size: 8, owner_pid: 1
    )

    shm.write(0, [1, 2, 3])
    result = shm.read(0, 3)
    result[0] = 99 # mutating the returned array

    # The mutation should not affect the shared region.
    assert_equal [1, 2, 3], shm.read(0, 3)
  end
end
