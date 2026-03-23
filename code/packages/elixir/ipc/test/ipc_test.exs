defmodule CodingAdventures.IPCTest do
  @moduledoc """
  # IPC Test Suite

  Comprehensive tests for all three IPC mechanisms (pipes, message queues,
  shared memory) and the IPC manager.
  """
  use ExUnit.Case, async: true

  alias CodingAdventures.IPC
  alias CodingAdventures.IPC.{MessageQueue, SharedMemoryRegion}

  # ============================================================================
  # Pipe Tests
  # ============================================================================

  describe "Pipe — write/read roundtrip" do
    test "writes and reads data back correctly" do
      pipe = IPC.new_pipe()
      {:ok, pipe, 5} = IPC.pipe_write(pipe, "hello")
      assert IPC.pipe_available(pipe) == 5

      {:ok, pipe, result} = IPC.pipe_read(pipe, 5)
      assert result == "hello"
      assert IPC.pipe_available(pipe) == 0
    end

    test "handles multiple sequential writes and reads" do
      pipe = IPC.new_pipe()
      {:ok, pipe, 3} = IPC.pipe_write(pipe, "abc")
      {:ok, pipe, 3} = IPC.pipe_write(pipe, "def")

      {:ok, _pipe, result} = IPC.pipe_read(pipe, 6)
      assert result == "abcdef"
    end

    test "handles partial reads" do
      pipe = IPC.new_pipe()
      {:ok, pipe, 5} = IPC.pipe_write(pipe, "hello")

      {:ok, pipe, first} = IPC.pipe_read(pipe, 3)
      assert first == "hel"
      assert IPC.pipe_available(pipe) == 2

      {:ok, _pipe, second} = IPC.pipe_read(pipe, 2)
      assert second == "lo"
    end

    test "returns empty binary when reading 0 or negative bytes" do
      pipe = IPC.new_pipe()
      {:ok, pipe, 4} = IPC.pipe_write(pipe, "data")
      {:ok, _pipe, result} = IPC.pipe_read(pipe, 0)
      assert result == <<>>
    end
  end

  describe "Pipe — circular wrapping" do
    test "wraps data around the end of the buffer" do
      pipe = IPC.new_pipe(8)

      {:ok, pipe, 6} = IPC.pipe_write(pipe, "abcdef")
      {:ok, pipe, _data} = IPC.pipe_read(pipe, 6)

      # Now read_pos=6, write_pos=6. Writing 5 bytes wraps around.
      {:ok, pipe, 5} = IPC.pipe_write(pipe, "ghijk")
      {:ok, _pipe, result} = IPC.pipe_read(pipe, 5)
      assert result == "ghijk"
    end

    test "handles multiple wrap-arounds" do
      pipe = IPC.new_pipe(4)

      {:ok, pipe, 4} = IPC.pipe_write(pipe, "abcd")
      {:ok, pipe, result} = IPC.pipe_read(pipe, 4)
      assert result == "abcd"

      {:ok, pipe, 4} = IPC.pipe_write(pipe, "efgh")
      {:ok, pipe, result} = IPC.pipe_read(pipe, 4)
      assert result == "efgh"

      {:ok, pipe, 4} = IPC.pipe_write(pipe, "ijkl")
      {:ok, _pipe, result} = IPC.pipe_read(pipe, 4)
      assert result == "ijkl"
    end
  end

  describe "Pipe — EOF" do
    test "signals EOF when all writers close and buffer is empty" do
      pipe = IPC.new_pipe()
      assert IPC.pipe_eof?(pipe) == false

      pipe = IPC.close_write(pipe)
      assert IPC.pipe_eof?(pipe) == true
    end

    test "is NOT EOF if data remains after writers close" do
      pipe = IPC.new_pipe()
      {:ok, pipe, 9} = IPC.pipe_write(pipe, "remaining")
      pipe = IPC.close_write(pipe)

      assert IPC.pipe_eof?(pipe) == false

      {:ok, pipe, _data} = IPC.pipe_read(pipe, 9)
      assert IPC.pipe_eof?(pipe) == true
    end

    test "returns empty binary when reading from EOF pipe" do
      pipe = IPC.new_pipe()
      pipe = IPC.close_write(pipe)
      {:ok, _pipe, result} = IPC.pipe_read(pipe, 10)
      assert result == <<>>
    end
  end

  describe "Pipe — BrokenPipe" do
    test "returns :broken_pipe error when writing with no readers" do
      pipe = IPC.new_pipe()
      pipe = IPC.close_read(pipe)

      assert {:error, :broken_pipe} = IPC.pipe_write(pipe, "doomed")
    end

    test "reports pipe_broken? when readers are 0" do
      pipe = IPC.new_pipe()
      assert IPC.pipe_broken?(pipe) == false
      pipe = IPC.close_read(pipe)
      assert IPC.pipe_broken?(pipe) == true
    end
  end

  describe "Pipe — capacity" do
    test "reports pipe_full? when buffer is at capacity" do
      pipe = IPC.new_pipe(4)
      {:ok, pipe, 4} = IPC.pipe_write(pipe, "abcd")
      assert IPC.pipe_full?(pipe) == true
      assert IPC.pipe_space(pipe) == 0
    end

    test "writes partial data when buffer has limited space" do
      pipe = IPC.new_pipe(4)
      {:ok, pipe, 2} = IPC.pipe_write(pipe, "ab")
      {:ok, pipe, 2} = IPC.pipe_write(pipe, "cdef")  # only 2 bytes fit
      assert IPC.pipe_full?(pipe) == true
      {:ok, _pipe, result} = IPC.pipe_read(pipe, 4)
      assert result == "abcd"
    end

    test "writes 0 bytes when buffer is completely full" do
      pipe = IPC.new_pipe(4)
      {:ok, pipe, 4} = IPC.pipe_write(pipe, "abcd")
      {:ok, _pipe, 0} = IPC.pipe_write(pipe, "e")
    end

    test "defaults to 4096 capacity" do
      pipe = IPC.new_pipe()
      assert pipe.capacity == 4096
    end
  end

  describe "Pipe — reference counts" do
    test "starts with 1 reader and 1 writer" do
      pipe = IPC.new_pipe()
      assert pipe.reader_count == 1
      assert pipe.writer_count == 1
    end

    test "does not go below 0 when closing multiple times" do
      pipe = IPC.new_pipe()
      pipe = IPC.close_read(pipe)
      pipe = IPC.close_read(pipe)
      assert pipe.reader_count == 0
    end

    test "tracks pipe_empty? correctly" do
      pipe = IPC.new_pipe()
      assert IPC.pipe_empty?(pipe) == true
      {:ok, pipe, 1} = IPC.pipe_write(pipe, "x")
      assert IPC.pipe_empty?(pipe) == false
      {:ok, pipe, _data} = IPC.pipe_read(pipe, 1)
      assert IPC.pipe_empty?(pipe) == true
    end
  end

  # ============================================================================
  # MessageQueue Tests
  # ============================================================================

  describe "MessageQueue — FIFO ordering" do
    test "delivers messages in FIFO order" do
      mq = IPC.new_message_queue()

      {:ok, mq} = IPC.mq_send(mq, 1, "first")
      {:ok, mq} = IPC.mq_send(mq, 1, "second")
      {:ok, mq} = IPC.mq_send(mq, 1, "third")

      {:ok, mq, msg1} = IPC.mq_receive(mq, 0)
      {:ok, mq, msg2} = IPC.mq_receive(mq, 0)
      {:ok, _mq, msg3} = IPC.mq_receive(mq, 0)

      assert msg1.body == "first"
      assert msg2.body == "second"
      assert msg3.body == "third"
    end

    test "returns :empty when queue is empty" do
      mq = IPC.new_message_queue()
      assert {:error, :empty} = IPC.mq_receive(mq, 0)
    end
  end

  describe "MessageQueue — type filtering" do
    test "filters messages by type" do
      mq = IPC.new_message_queue()

      {:ok, mq} = IPC.mq_send(mq, 1, "type1-first")
      {:ok, mq} = IPC.mq_send(mq, 2, "type2-only")
      {:ok, mq} = IPC.mq_send(mq, 1, "type1-second")

      {:ok, mq, msg} = IPC.mq_receive(mq, 2)
      assert msg.msg_type == 2
      assert msg.body == "type2-only"
      assert mq.message_count == 2
    end

    test "returns :empty when no message matches requested type" do
      mq = IPC.new_message_queue()
      {:ok, mq} = IPC.mq_send(mq, 1, "only type 1")
      assert {:error, :empty} = IPC.mq_receive(mq, 99)
      assert mq.message_count == 1
    end

    test "returns oldest matching message for a given type" do
      mq = IPC.new_message_queue()
      {:ok, mq} = IPC.mq_send(mq, 1, "a")
      {:ok, mq} = IPC.mq_send(mq, 2, "b")
      {:ok, mq} = IPC.mq_send(mq, 1, "c")

      {:ok, mq, msg1} = IPC.mq_receive(mq, 1)
      assert msg1.body == "a"

      {:ok, mq, msg2} = IPC.mq_receive(mq, 1)
      assert msg2.body == "c"

      assert mq.message_count == 1
      {:ok, _mq, msg3} = IPC.mq_receive(mq, 2)
      assert msg3.body == "b"
    end
  end

  describe "MessageQueue — capacity limits" do
    test "rejects sends when queue is full" do
      mq = IPC.new_message_queue(3)

      {:ok, mq} = IPC.mq_send(mq, 1, "a")
      {:ok, mq} = IPC.mq_send(mq, 1, "b")
      {:ok, mq} = IPC.mq_send(mq, 1, "c")
      assert IPC.mq_full?(mq) == true

      assert {:error, :full} = IPC.mq_send(mq, 1, "d")
    end

    test "rejects oversized messages" do
      mq = IPC.new_message_queue(256, 8)

      {:ok, _mq} = IPC.mq_send(mq, 1, "short")
      assert {:error, :oversized} = IPC.mq_send(mq, 1, "this is way too long")
    end

    test "accepts messages exactly at the size limit" do
      mq = IPC.new_message_queue(256, 5)
      {:ok, _mq} = IPC.mq_send(mq, 1, "12345")
    end
  end

  describe "MessageQueue — message structure" do
    test "preserves message type and size" do
      mq = IPC.new_message_queue()
      {:ok, mq} = IPC.mq_send(mq, 42, "test payload")

      {:ok, _mq, msg} = IPC.mq_receive(mq, 0)
      assert msg.msg_type == 42
      assert msg.msg_size == 12
      assert msg.body == "test payload"
    end
  end

  describe "MessageQueue — status" do
    test "reports mq_empty? and message_count correctly" do
      mq = IPC.new_message_queue()
      assert IPC.mq_empty?(mq) == true
      assert mq.message_count == 0

      {:ok, mq} = IPC.mq_send(mq, 1, "a")
      assert IPC.mq_empty?(mq) == false
      assert mq.message_count == 1

      {:ok, mq, _msg} = IPC.mq_receive(mq, 0)
      assert IPC.mq_empty?(mq) == true
      assert mq.message_count == 0
    end
  end

  # ============================================================================
  # SharedMemoryRegion Tests
  # ============================================================================

  describe "SharedMemoryRegion — attach/detach" do
    test "attaches a PID successfully" do
      region = IPC.new_shared_memory("test", 1024, 1)
      {:ok, region} = IPC.shm_attach(region, 100)
      assert IPC.shm_attached?(region, 100) == true
      assert IPC.shm_attached_count(region) == 1
    end

    test "returns :already_attached for duplicate PID" do
      region = IPC.new_shared_memory("test", 1024, 1)
      {:ok, region} = IPC.shm_attach(region, 100)
      assert {:error, :already_attached} = IPC.shm_attach(region, 100)
    end

    test "detaches a PID successfully" do
      region = IPC.new_shared_memory("test", 1024, 1)
      {:ok, region} = IPC.shm_attach(region, 100)
      {:ok, region} = IPC.shm_detach(region, 100)
      assert IPC.shm_attached?(region, 100) == false
      assert IPC.shm_attached_count(region) == 0
    end

    test "returns :not_attached for non-attached PID" do
      region = IPC.new_shared_memory("test", 1024, 1)
      assert {:error, :not_attached} = IPC.shm_detach(region, 999)
    end

    test "handles multiple PIDs attached simultaneously" do
      region = IPC.new_shared_memory("test", 1024, 1)
      {:ok, region} = IPC.shm_attach(region, 10)
      {:ok, region} = IPC.shm_attach(region, 20)
      {:ok, region} = IPC.shm_attach(region, 30)

      assert IPC.shm_attached_count(region) == 3

      {:ok, region} = IPC.shm_detach(region, 20)
      assert IPC.shm_attached_count(region) == 2
      assert IPC.shm_attached?(region, 20) == false
    end
  end

  describe "SharedMemoryRegion — read/write" do
    test "writes and reads data at a given offset" do
      region = IPC.new_shared_memory("test", 1024, 1)
      {:ok, region, 5} = IPC.shm_write(region, 0, "hello")
      {:ok, result} = IPC.shm_read(region, 0, 5)
      assert result == "hello"
    end

    test "writes at non-zero offset" do
      region = IPC.new_shared_memory("test", 1024, 1)
      {:ok, region, 5} = IPC.shm_write(region, 100, "world")
      {:ok, result} = IPC.shm_read(region, 100, 5)
      assert result == "world"
    end

    test "initializes all bytes to 0" do
      region = IPC.new_shared_memory("test", 16, 1)
      {:ok, result} = IPC.shm_read(region, 0, 16)
      assert result == :binary.copy(<<0>>, 16)
    end

    test "handles overlapping writes (last write wins)" do
      region = IPC.new_shared_memory("test", 1024, 1)
      {:ok, region, 4} = IPC.shm_write(region, 0, "aaaa")
      {:ok, region, 2} = IPC.shm_write(region, 2, "bb")
      {:ok, result} = IPC.shm_read(region, 0, 4)
      assert result == "aabb"
    end
  end

  describe "SharedMemoryRegion — bounds checking" do
    test "returns :out_of_bounds on read past end" do
      region = IPC.new_shared_memory("test", 16, 1)
      assert {:error, :out_of_bounds} = IPC.shm_read(region, 14, 4)
    end

    test "returns :out_of_bounds on write past end" do
      region = IPC.new_shared_memory("test", 16, 1)
      assert {:error, :out_of_bounds} = IPC.shm_write(region, 14, "abcd")
    end

    test "allows read/write at exact boundary" do
      region = IPC.new_shared_memory("test", 4, 1)
      {:ok, region, 4} = IPC.shm_write(region, 0, "abcd")
      {:ok, result} = IPC.shm_read(region, 0, 4)
      assert result == "abcd"
    end
  end

  describe "SharedMemoryRegion — constructor" do
    test "stores name, size, and owner_pid" do
      region = IPC.new_shared_memory("buffer_pool", 8192, 42)
      assert region.region_name == "buffer_pool"
      assert region.region_size == 8192
      assert region.owner_pid == 42
    end
  end

  # ============================================================================
  # IPCManager Tests
  # ============================================================================

  describe "IPCManager — pipe management" do
    test "creates a pipe and returns a handle" do
      mgr = IPC.new_manager()
      {_mgr, handle} = IPC.create_pipe(mgr)
      assert handle.pipe_id == 0
      assert is_integer(handle.read_fd)
      assert is_integer(handle.write_fd)
      assert handle.read_fd != handle.write_fd
    end

    test "retrieves a pipe by ID" do
      mgr = IPC.new_manager()
      {mgr, handle} = IPC.create_pipe(mgr)
      assert {:ok, _pipe} = IPC.get_pipe(mgr, handle.pipe_id)
    end

    test "returns :not_found for non-existent pipe" do
      mgr = IPC.new_manager()
      assert {:error, :not_found} = IPC.get_pipe(mgr, 999)
    end

    test "closes read/write ends via manager" do
      mgr = IPC.new_manager()
      {mgr, handle} = IPC.create_pipe(mgr)

      {:ok, mgr} = IPC.close_pipe_read(mgr, handle.pipe_id)
      {:ok, pipe} = IPC.get_pipe(mgr, handle.pipe_id)
      assert pipe.reader_count == 0

      {:ok, mgr} = IPC.close_pipe_write(mgr, handle.pipe_id)
      {:ok, pipe} = IPC.get_pipe(mgr, handle.pipe_id)
      assert pipe.writer_count == 0
    end

    test "returns :not_found when closing non-existent pipe" do
      mgr = IPC.new_manager()
      assert {:error, :not_found} = IPC.close_pipe_read(mgr, 999)
      assert {:error, :not_found} = IPC.close_pipe_write(mgr, 999)
    end

    test "destroys a pipe" do
      mgr = IPC.new_manager()
      {mgr, handle} = IPC.create_pipe(mgr)
      {:ok, mgr} = IPC.destroy_pipe(mgr, handle.pipe_id)
      assert {:error, :not_found} = IPC.get_pipe(mgr, handle.pipe_id)
    end

    test "returns :not_found when destroying non-existent pipe" do
      mgr = IPC.new_manager()
      assert {:error, :not_found} = IPC.destroy_pipe(mgr, 999)
    end

    test "creates pipe with custom capacity" do
      mgr = IPC.new_manager()
      {mgr, handle} = IPC.create_pipe(mgr, 128)
      {:ok, pipe} = IPC.get_pipe(mgr, handle.pipe_id)
      assert pipe.capacity == 128
    end
  end

  describe "IPCManager — message queue management" do
    test "creates a message queue" do
      mgr = IPC.new_manager()
      {_mgr, mq} = IPC.create_message_queue(mgr, "jobs")
      assert %MessageQueue{} = mq
    end

    test "returns existing queue for same name (idempotent)" do
      mgr = IPC.new_manager()
      {mgr, mq1} = IPC.create_message_queue(mgr, "jobs")
      {_mgr, mq2} = IPC.create_message_queue(mgr, "jobs")
      assert mq1 == mq2
    end

    test "retrieves a queue by name" do
      mgr = IPC.new_manager()
      {mgr, _mq} = IPC.create_message_queue(mgr, "alerts")
      assert {:ok, _mq} = IPC.get_message_queue(mgr, "alerts")
    end

    test "returns :not_found for non-existent queue" do
      mgr = IPC.new_manager()
      assert {:error, :not_found} = IPC.get_message_queue(mgr, "nope")
    end

    test "deletes a queue" do
      mgr = IPC.new_manager()
      {mgr, _mq} = IPC.create_message_queue(mgr, "temp")
      {:ok, mgr} = IPC.delete_message_queue(mgr, "temp")
      assert {:error, :not_found} = IPC.get_message_queue(mgr, "temp")
    end

    test "returns :not_found when deleting non-existent queue" do
      mgr = IPC.new_manager()
      assert {:error, :not_found} = IPC.delete_message_queue(mgr, "nope")
    end
  end

  describe "IPCManager — shared memory management" do
    test "creates a shared memory region" do
      mgr = IPC.new_manager()
      {_mgr, region} = IPC.create_shared_memory(mgr, "buffer", 4096, 1)
      assert %SharedMemoryRegion{} = region
      assert region.region_name == "buffer"
    end

    test "returns existing region for same name (idempotent)" do
      mgr = IPC.new_manager()
      {mgr, r1} = IPC.create_shared_memory(mgr, "pool", 4096, 1)
      {_mgr, r2} = IPC.create_shared_memory(mgr, "pool", 8192, 2)
      assert r1 == r2
    end

    test "retrieves a region by name" do
      mgr = IPC.new_manager()
      {mgr, _region} = IPC.create_shared_memory(mgr, "data", 1024, 1)
      assert {:ok, _region} = IPC.get_shared_memory(mgr, "data")
    end

    test "returns :not_found for non-existent region" do
      mgr = IPC.new_manager()
      assert {:error, :not_found} = IPC.get_shared_memory(mgr, "nope")
    end

    test "deletes a region" do
      mgr = IPC.new_manager()
      {mgr, _region} = IPC.create_shared_memory(mgr, "temp", 512, 1)
      {:ok, mgr} = IPC.delete_shared_memory(mgr, "temp")
      assert {:error, :not_found} = IPC.get_shared_memory(mgr, "temp")
    end

    test "returns :not_found when deleting non-existent region" do
      mgr = IPC.new_manager()
      assert {:error, :not_found} = IPC.delete_shared_memory(mgr, "nope")
    end
  end

  describe "IPCManager — listing resources" do
    test "lists all pipes" do
      mgr = IPC.new_manager()
      {mgr, _h1} = IPC.create_pipe(mgr)
      {mgr, _h2} = IPC.create_pipe(mgr)
      assert IPC.list_pipes(mgr) == [0, 1]
    end

    test "lists all message queues" do
      mgr = IPC.new_manager()
      {mgr, _mq} = IPC.create_message_queue(mgr, "a")
      {mgr, _mq} = IPC.create_message_queue(mgr, "b")
      result = IPC.list_message_queues(mgr)
      assert Enum.sort(result) == ["a", "b"]
    end

    test "lists all shared regions" do
      mgr = IPC.new_manager()
      {mgr, _r} = IPC.create_shared_memory(mgr, "x", 100, 1)
      {mgr, _r} = IPC.create_shared_memory(mgr, "y", 200, 2)
      result = IPC.list_shared_regions(mgr)
      assert Enum.sort(result) == ["x", "y"]
    end

    test "returns empty lists when no resources exist" do
      mgr = IPC.new_manager()
      assert IPC.list_pipes(mgr) == []
      assert IPC.list_message_queues(mgr) == []
      assert IPC.list_shared_regions(mgr) == []
    end

    test "updates lists after deletion" do
      mgr = IPC.new_manager()
      {mgr, h} = IPC.create_pipe(mgr)
      {mgr, _mq} = IPC.create_message_queue(mgr, "q")
      {mgr, _r} = IPC.create_shared_memory(mgr, "s", 100, 1)

      {:ok, mgr} = IPC.destroy_pipe(mgr, h.pipe_id)
      {:ok, mgr} = IPC.delete_message_queue(mgr, "q")
      {:ok, mgr} = IPC.delete_shared_memory(mgr, "s")

      assert IPC.list_pipes(mgr) == []
      assert IPC.list_message_queues(mgr) == []
      assert IPC.list_shared_regions(mgr) == []
    end
  end

  describe "IPCManager — integration" do
    test "supports full pipe lifecycle through manager" do
      mgr = IPC.new_manager()
      {mgr, handle} = IPC.create_pipe(mgr, 64)
      {:ok, pipe} = IPC.get_pipe(mgr, handle.pipe_id)

      {:ok, pipe, 16} = IPC.pipe_write(pipe, "integration test")
      mgr = IPC.update_pipe(mgr, handle.pipe_id, pipe)

      {:ok, pipe} = IPC.get_pipe(mgr, handle.pipe_id)
      {:ok, pipe, result} = IPC.pipe_read(pipe, 16)
      assert result == "integration test"

      mgr = IPC.update_pipe(mgr, handle.pipe_id, pipe)
      {:ok, mgr} = IPC.close_pipe_write(mgr, handle.pipe_id)
      {:ok, pipe} = IPC.get_pipe(mgr, handle.pipe_id)
      assert IPC.pipe_eof?(pipe) == true

      {:ok, _mgr} = IPC.destroy_pipe(mgr, handle.pipe_id)
    end

    test "supports message queue send/receive through manager" do
      mgr = IPC.new_manager()
      {mgr, mq} = IPC.create_message_queue(mgr, "work")

      {:ok, mq} = IPC.mq_send(mq, 1, "job-1")
      {:ok, mq} = IPC.mq_send(mq, 2, "job-2")
      mgr = IPC.update_message_queue(mgr, "work", mq)

      {:ok, mq} = IPC.get_message_queue(mgr, "work")
      {:ok, mq, msg2} = IPC.mq_receive(mq, 2)
      assert msg2.body == "job-2"

      {:ok, _mq, msg1} = IPC.mq_receive(mq, 1)
      assert msg1.body == "job-1"
    end

    test "supports shared memory multi-PID access through manager" do
      mgr = IPC.new_manager()
      {mgr, region} = IPC.create_shared_memory(mgr, "shared", 256, 1)

      {:ok, region} = IPC.shm_attach(region, 10)
      {:ok, region} = IPC.shm_attach(region, 20)

      {:ok, region, 11} = IPC.shm_write(region, 0, "shared data")
      mgr = IPC.update_shared_memory(mgr, "shared", region)

      {:ok, region} = IPC.get_shared_memory(mgr, "shared")
      {:ok, result} = IPC.shm_read(region, 0, 11)
      assert result == "shared data"

      {:ok, region} = IPC.shm_detach(region, 10)
      {:ok, region} = IPC.shm_detach(region, 20)
      assert IPC.shm_attached_count(region) == 0
    end
  end
end
