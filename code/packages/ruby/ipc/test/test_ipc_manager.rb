# frozen_string_literal: true

require_relative "test_helper"

# Tests for the IpcManager class -- the kernel's IPC resource manager.
#
# We test:
#   1. Pipe creation, retrieval, closing, listing
#   2. Message queue creation, retrieval, deletion, listing
#   3. Shared memory creation, retrieval, deletion, listing
#   4. Idempotent create for message queues and shared memory
class TestIpcManager < Minitest::Test
  def setup
    @manager = CodingAdventures::Ipc::IpcManager.new
  end

  # ---------------------------------------------------------------
  # Pipe operations
  # ---------------------------------------------------------------

  def test_create_pipe_returns_triple
    pipe_id, read_fd, write_fd = @manager.create_pipe

    assert_kind_of Integer, pipe_id
    assert_kind_of Integer, read_fd
    assert_kind_of Integer, write_fd
    refute_equal read_fd, write_fd
  end

  def test_get_pipe
    pipe_id, _, _ = @manager.create_pipe
    pipe = @manager.get_pipe(pipe_id)

    assert_instance_of CodingAdventures::Ipc::Pipe, pipe
  end

  def test_get_nonexistent_pipe_returns_nil
    assert_nil @manager.get_pipe(999)
  end

  def test_close_pipe_read
    pipe_id, _, _ = @manager.create_pipe
    @manager.close_pipe_read(pipe_id)

    pipe = @manager.get_pipe(pipe_id)
    assert_equal 0, pipe.reader_count
  end

  def test_close_pipe_write
    pipe_id, _, _ = @manager.create_pipe
    @manager.close_pipe_write(pipe_id)

    pipe = @manager.get_pipe(pipe_id)
    assert_equal 0, pipe.writer_count
  end

  def test_close_pipe_nonexistent_does_not_raise
    # Closing a non-existent pipe should be a no-op.
    @manager.close_pipe_read(999)
    @manager.close_pipe_write(999)
  end

  def test_list_pipes
    @manager.create_pipe
    @manager.create_pipe

    pipes = @manager.list_pipes
    assert_equal 2, pipes.length
  end

  def test_list_pipes_empty
    assert_empty @manager.list_pipes
  end

  def test_multiple_pipes_have_unique_ids
    id1, _, _ = @manager.create_pipe
    id2, _, _ = @manager.create_pipe
    refute_equal id1, id2
  end

  def test_pipe_write_and_read_through_manager
    pipe_id, _, _ = @manager.create_pipe
    pipe = @manager.get_pipe(pipe_id)

    pipe.write([1, 2, 3])
    result = pipe.read(3)
    assert_equal [1, 2, 3], result
  end

  # ---------------------------------------------------------------
  # Message queue operations
  # ---------------------------------------------------------------

  def test_create_message_queue
    mq = @manager.create_message_queue("test_queue")
    assert_instance_of CodingAdventures::Ipc::MessageQueue, mq
  end

  def test_create_message_queue_idempotent
    mq1 = @manager.create_message_queue("test_queue")
    mq2 = @manager.create_message_queue("test_queue")

    # Same name should return the same queue object.
    assert_same mq1, mq2
  end

  def test_get_message_queue
    @manager.create_message_queue("q1")
    mq = @manager.get_message_queue("q1")

    assert_instance_of CodingAdventures::Ipc::MessageQueue, mq
  end

  def test_get_nonexistent_message_queue_returns_nil
    assert_nil @manager.get_message_queue("nonexistent")
  end

  def test_delete_message_queue
    @manager.create_message_queue("q1")
    @manager.delete_message_queue("q1")

    assert_nil @manager.get_message_queue("q1")
  end

  def test_delete_nonexistent_queue_does_not_raise
    @manager.delete_message_queue("nonexistent")
  end

  def test_list_message_queues
    @manager.create_message_queue("q1")
    @manager.create_message_queue("q2")

    queues = @manager.list_message_queues
    assert_includes queues, "q1"
    assert_includes queues, "q2"
    assert_equal 2, queues.length
  end

  def test_list_message_queues_empty
    assert_empty @manager.list_message_queues
  end

  def test_message_queue_send_receive_through_manager
    @manager.create_message_queue("q1")
    mq = @manager.get_message_queue("q1")

    mq.send(1, [10, 20])
    msg = mq.receive
    assert_equal 1, msg.msg_type
    assert_equal [10, 20], msg.body
  end

  # ---------------------------------------------------------------
  # Shared memory operations
  # ---------------------------------------------------------------

  def test_create_shared_memory
    shm = @manager.create_shared_memory("region1", size: 1024, owner_pid: 1)
    assert_instance_of CodingAdventures::Ipc::SharedMemoryRegion, shm
  end

  def test_create_shared_memory_idempotent
    shm1 = @manager.create_shared_memory("region1", size: 1024, owner_pid: 1)
    shm2 = @manager.create_shared_memory("region1", size: 2048, owner_pid: 2)

    # Same name returns the same region (ignores new size/owner).
    assert_same shm1, shm2
    assert_equal 1024, shm2.size
  end

  def test_get_shared_memory
    @manager.create_shared_memory("region1", size: 512, owner_pid: 1)
    shm = @manager.get_shared_memory("region1")

    assert_instance_of CodingAdventures::Ipc::SharedMemoryRegion, shm
    assert_equal 512, shm.size
  end

  def test_get_nonexistent_shared_memory_returns_nil
    assert_nil @manager.get_shared_memory("nonexistent")
  end

  def test_delete_shared_memory
    @manager.create_shared_memory("region1", size: 512, owner_pid: 1)
    @manager.delete_shared_memory("region1")

    assert_nil @manager.get_shared_memory("region1")
  end

  def test_delete_nonexistent_shared_memory_does_not_raise
    @manager.delete_shared_memory("nonexistent")
  end

  def test_list_shared_memory
    @manager.create_shared_memory("r1", size: 100, owner_pid: 1)
    @manager.create_shared_memory("r2", size: 200, owner_pid: 1)

    regions = @manager.list_shared_memory
    assert_includes regions, "r1"
    assert_includes regions, "r2"
    assert_equal 2, regions.length
  end

  def test_list_shared_memory_empty
    assert_empty @manager.list_shared_memory
  end

  def test_shared_memory_write_read_through_manager
    @manager.create_shared_memory("region1", size: 64, owner_pid: 1)
    shm = @manager.get_shared_memory("region1")

    shm.attach(1)
    shm.write(0, [42, 43, 44])
    result = shm.read(0, 3)
    assert_equal [42, 43, 44], result
  end

  # ---------------------------------------------------------------
  # Cross-type: all resources are independent
  # ---------------------------------------------------------------

  def test_different_resource_types_are_independent
    @manager.create_pipe
    @manager.create_message_queue("q1")
    @manager.create_shared_memory("r1", size: 64, owner_pid: 1)

    assert_equal 1, @manager.list_pipes.length
    assert_equal 1, @manager.list_message_queues.length
    assert_equal 1, @manager.list_shared_memory.length
  end
end
