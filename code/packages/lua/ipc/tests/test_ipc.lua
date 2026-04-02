-- Tests for coding_adventures.ipc
-- Coverage target: 95%+

local IPC = require("coding_adventures.ipc")

-- ============================================================================
-- Constants
-- ============================================================================

describe("constants", function()
  it("DEFAULT_PIPE_CAPACITY is 4096", function()
    assert.are.equal(IPC.DEFAULT_PIPE_CAPACITY, 4096)
  end)
  it("DEFAULT_MAX_MESSAGES is 256", function()
    assert.are.equal(IPC.DEFAULT_MAX_MESSAGES, 256)
  end)
  it("DEFAULT_MAX_MESSAGE_SIZE is 4096", function()
    assert.are.equal(IPC.DEFAULT_MAX_MESSAGE_SIZE, 4096)
  end)
end)

-- ============================================================================
-- Pipe tests
-- ============================================================================

describe("Pipe", function()
  it("new creates pipe with defaults", function()
    local p = IPC.Pipe.new()
    assert.are.equal(p.capacity, 4096)
    assert.are.equal(p.count, 0)
    assert.are.equal(p.reader_count, 1)
    assert.are.equal(p.writer_count, 1)
    assert.is_true(p:is_empty())
    assert.is_false(p:is_full())
  end)

  it("new with custom capacity", function()
    local p = IPC.Pipe.new(8)
    assert.are.equal(p.capacity, 8)
  end)

  it("new rejects non-positive capacity", function()
    assert.has_error(function() IPC.Pipe.new(0) end)
  end)

  it("write and read round-trip", function()
    local p = IPC.Pipe.new(16)
    local status, p2, written = p:write("hello")
    assert.are.equal(status, "ok")
    assert.are.equal(written, 5)
    assert.are.equal(p2:available(), 5)

    local rs, p3, data = p2:read(5)
    assert.are.equal(rs, "ok")
    assert.are.equal(data, "hello")
    assert.are.equal(p3:available(), 0)
  end)

  it("read returns empty when buffer is empty and writers open", function()
    local p = IPC.Pipe.new(8)
    local status, _, data = p:read(4)
    assert.are.equal(status, "empty")
    assert.are.equal(data, "")
  end)

  it("read returns eof when writers closed and buffer empty", function()
    local p = IPC.Pipe.new(8)
    p = p:close_write()
    local status, _, data = p:read(4)
    assert.are.equal(status, "eof")
    assert.are.equal(data, "")
  end)

  it("write returns broken_pipe when all readers closed", function()
    local p = IPC.Pipe.new(8)
    p = p:close_read()
    local status, _, written = p:write("hi")
    assert.are.equal(status, "broken_pipe")
    assert.are.equal(written, 0)
  end)

  it("write returns full on partial write", function()
    local p = IPC.Pipe.new(4)
    local status, p2, written = p:write("hello world")  -- 11 bytes > 4
    assert.are.equal(status, "full")
    assert.are.equal(written, 4)
    assert.is_true(p2:is_full())
  end)

  it("circular buffer wraps around correctly", function()
    local p = IPC.Pipe.new(8)
    local _, p2, _ = p:write("abcde")    -- write 5
    local _, p3, data1 = p2:read(3)      -- read 3 → "abc", 2 remaining
    assert.are.equal(data1, "abc")
    local _, p4, _ = p3:write("fghij")   -- write 5 more (wraps)
    local _, p5, data2 = p4:read(7)      -- read 7
    assert.are.equal(data2, "defghij")
  end)

  it("partial read reads available bytes", function()
    local p = IPC.Pipe.new(8)
    local _, p2, _ = p:write("abc")
    local _, p3, data = p2:read(10)   -- ask for 10, get 3
    assert.are.equal(data, "abc")
    assert.are.equal(p3:available(), 0)
  end)

  it("immutability: write does not mutate original", function()
    local p1 = IPC.Pipe.new(8)
    local _, p2, _ = p1:write("hi")
    assert.are.equal(p1:available(), 0)
    assert.are.equal(p2:available(), 2)
  end)

  it("available returns byte count", function()
    local p = IPC.Pipe.new(16)
    assert.are.equal(p:available(), 0)
    local _, p2, _ = p:write("hello")
    assert.are.equal(p2:available(), 5)
  end)

  it("multiple writes and reads sequence", function()
    local p = IPC.Pipe.new(64)
    local _, p2, _ = p:write("first ")
    local _, p3, _ = p2:write("second")
    local _, p4, data = p3:read(12)
    assert.are.equal(data, "first second")
  end)
end)

-- ============================================================================
-- Message / MessageQueue tests
-- ============================================================================

describe("message_new", function()
  it("creates message with fields", function()
    local msg = IPC.message_new(1, "hello")
    assert.are.equal(msg.msg_type, 1)
    assert.are.equal(msg.body, "hello")
    assert.are.equal(msg.msg_size, 5)
  end)

  it("defaults body to empty string", function()
    local msg = IPC.message_new(1)
    assert.are.equal(msg.body, "")
    assert.are.equal(msg.msg_size, 0)
  end)
end)

describe("MessageQueue", function()
  it("new creates empty queue", function()
    local q = IPC.MessageQueue.new()
    assert.is_true(q:is_empty())
    assert.is_false(q:is_full())
    assert.are.equal(q.message_count, 0)
  end)

  it("new with custom limits", function()
    local q = IPC.MessageQueue.new(10, 128)
    assert.are.equal(q.max_messages, 10)
    assert.are.equal(q.max_message_size, 128)
  end)

  it("send and receive round-trip", function()
    local q = IPC.MessageQueue.new()
    local status, q2 = q:send(1, "hello")
    assert.are.equal(status, "ok")
    assert.are.equal(q2.message_count, 1)

    local rs, q3, msg = q2:receive(0)
    assert.are.equal(rs, "ok")
    assert.are.equal(msg.body, "hello")
    assert.are.equal(msg.msg_type, 1)
    assert.is_true(q3:is_empty())
  end)

  it("FIFO order preserved", function()
    local q = IPC.MessageQueue.new()
    local _, q2 = q:send(1, "first")
    local _, q3 = q2:send(1, "second")
    local _, q4 = q3:send(1, "third")

    local _, q5, m1 = q4:receive(0)
    local _, q6, m2 = q5:receive(0)
    local _, _, m3 = q6:receive(0)

    assert.are.equal(m1.body, "first")
    assert.are.equal(m2.body, "second")
    assert.are.equal(m3.body, "third")
  end)

  it("receive with type filter", function()
    local q = IPC.MessageQueue.new()
    local _, q2 = q:send(1, "type1")
    local _, q3 = q2:send(2, "type2")
    local _, q4 = q3:send(1, "type1-again")

    -- Receive type 2 specifically
    local status, q5, msg = q4:receive(2)
    assert.are.equal(status, "ok")
    assert.are.equal(msg.body, "type2")
    assert.are.equal(q5.message_count, 2)

    -- Type 3 not present
    local rs, _, m = q5:receive(3)
    assert.are.equal(rs, "empty")
    assert.is_nil(m)
  end)

  it("receive from empty queue returns empty", function()
    local q = IPC.MessageQueue.new()
    local status, _, msg = q:receive(0)
    assert.are.equal(status, "empty")
    assert.is_nil(msg)
  end)

  it("send returns oversized when body too large", function()
    local q = IPC.MessageQueue.new(10, 5)
    local status, q2 = q:send(1, "toolarge")
    assert.are.equal(status, "oversized")
    assert.are.equal(q2.message_count, 0)
  end)

  it("send returns full when queue is at capacity", function()
    local q = IPC.MessageQueue.new(2, 4096)
    local _, q2 = q:send(1, "a")
    local _, q3 = q2:send(1, "b")
    assert.is_true(q3:is_full())
    local status, q4 = q3:send(1, "c")
    assert.are.equal(status, "full")
    assert.are.equal(q4.message_count, 2)
  end)

  it("immutability: send does not mutate original", function()
    local q = IPC.MessageQueue.new()
    local _, q2 = q:send(1, "hi")
    assert.are.equal(q.message_count, 0)
    assert.are.equal(q2.message_count, 1)
  end)

  it("multiple types coexist", function()
    local q = IPC.MessageQueue.new()
    local _, q2 = q:send(1, "cmd1")
    local _, q3 = q2:send(2, "reply1")
    local _, q4 = q3:send(1, "cmd2")

    local _, q5, r1 = q4:receive(1)
    assert.are.equal(r1.body, "cmd1")
    local _, _, r2 = q5:receive(1)
    assert.are.equal(r2.body, "cmd2")
  end)
end)

-- ============================================================================
-- SharedMemory tests
-- ============================================================================

describe("SharedMemory", function()
  it("new creates zero-initialized region", function()
    local s = IPC.SharedMemory.new("seg1", 16, 100)
    assert.are.equal(s.region_name, "seg1")
    assert.are.equal(s.region_size, 16)
    assert.are.equal(s.owner_pid, 100)
    assert.are.equal(s:attached_count(), 0)
  end)

  it("new rejects non-positive size", function()
    assert.has_error(function() IPC.SharedMemory.new("x", 0, 1) end)
  end)

  it("write and read round-trip", function()
    local s = IPC.SharedMemory.new("test", 32, 1)
    local status, s2, n = s:write(0, "hello")
    assert.are.equal(status, "ok")
    assert.are.equal(n, 5)

    local rs, data = s2:read(0, 5)
    assert.are.equal(rs, "ok")
    assert.are.equal(data, "hello")
  end)

  it("write at offset", function()
    local s = IPC.SharedMemory.new("test", 32, 1)
    local _, s2, _ = s:write(10, "world")
    local _, data = s2:read(10, 5)
    assert.are.equal(data, "world")
  end)

  it("read out of bounds returns error", function()
    local s = IPC.SharedMemory.new("test", 8, 1)
    local status, data = s:read(6, 5)  -- 6+5=11 > 8
    assert.are.equal(status, "out_of_bounds")
    assert.is_nil(data)
  end)

  it("write out of bounds returns error", function()
    local s = IPC.SharedMemory.new("test", 8, 1)
    local status, _, n = s:write(6, "toolong")  -- 6+7=13 > 8
    assert.are.equal(status, "out_of_bounds")
    assert.are.equal(n, 0)
  end)

  it("attach and detach", function()
    local s = IPC.SharedMemory.new("seg", 64, 1)
    local st1, s2 = s:attach(200)
    assert.are.equal(st1, "ok")
    assert.is_true(s2:is_attached(200))
    assert.are.equal(s2:attached_count(), 1)

    local st2, s3 = s2:detach(200)
    assert.are.equal(st2, "ok")
    assert.is_false(s3:is_attached(200))
    assert.are.equal(s3:attached_count(), 0)
  end)

  it("attach returns already_attached if pid already attached", function()
    local s = IPC.SharedMemory.new("seg", 64, 1)
    local _, s2 = s:attach(200)
    local status, s3 = s2:attach(200)
    assert.are.equal(status, "already_attached")
    assert.are.equal(s3:attached_count(), 1)
  end)

  it("detach returns not_attached for unknown pid", function()
    local s = IPC.SharedMemory.new("seg", 64, 1)
    local status, _ = s:detach(999)
    assert.are.equal(status, "not_attached")
  end)

  it("multiple pids can attach", function()
    local s = IPC.SharedMemory.new("seg", 64, 1)
    local _, s2 = s:attach(10)
    local _, s3 = s2:attach(20)
    local _, s4 = s3:attach(30)
    assert.are.equal(s4:attached_count(), 3)
    assert.is_true(s4:is_attached(10))
    assert.is_true(s4:is_attached(20))
    assert.is_true(s4:is_attached(30))
  end)

  it("immutability: write does not mutate original", function()
    local s = IPC.SharedMemory.new("seg", 32, 1)
    local _, s2, _ = s:write(0, "hi")
    local _, d1 = s:read(0, 2)
    local _, d2 = s2:read(0, 2)
    -- original was zero-initialized
    assert.are.equal(string.byte(d1, 1), 0)
    assert.are.equal(d2, "hi")
  end)

  it("read returns correct data at boundary", function()
    local s = IPC.SharedMemory.new("seg", 4, 1)
    local _, s2, _ = s:write(0, "abcd")
    local _, data = s2:read(0, 4)
    assert.are.equal(data, "abcd")
    -- read exactly at end
    local st, _ = s2:read(4, 0)
    assert.are.equal(st, "ok")
  end)
end)

-- ============================================================================
-- Manager tests
-- ============================================================================

describe("Manager", function()
  it("new creates empty manager", function()
    local m = IPC.Manager.new()
    assert.are.equal(m.next_pipe_id, 0)
    assert.are.equal(m.next_fd, 100)
  end)

  it("create_pipe returns manager and handle", function()
    local m = IPC.Manager.new()
    local m2, handle = m:create_pipe()
    assert.are.equal(handle.pipe_id, 0)
    assert.are.equal(handle.read_fd, 100)
    assert.are.equal(handle.write_fd, 101)
    assert.are.equal(m2.next_pipe_id, 1)
    assert.are.equal(m2.next_fd, 102)
  end)

  it("create_pipe multiple times assigns unique IDs", function()
    local m = IPC.Manager.new()
    local m2, h1 = m:create_pipe()
    local m3, h2 = m2:create_pipe()
    assert.are_not.equal(h1.pipe_id, h2.pipe_id)
    assert.are_not.equal(h1.read_fd, h2.read_fd)
  end)

  it("get_pipe returns pipe by ID", function()
    local m = IPC.Manager.new()
    local m2, handle = m:create_pipe()
    local status, pipe = m2:get_pipe(handle.pipe_id)
    assert.are.equal(status, "ok")
    assert.is_not_nil(pipe)
  end)

  it("get_pipe returns not_found for unknown ID", function()
    local m = IPC.Manager.new()
    local status, _ = m:get_pipe(999)
    assert.are.equal(status, "not_found")
  end)

  it("update_pipe replaces pipe", function()
    local m = IPC.Manager.new()
    local m2, handle = m:create_pipe()
    local _, pipe = m2:get_pipe(handle.pipe_id)
    local _, pipe2, _ = pipe:write("hello")
    local m3 = m2:update_pipe(handle.pipe_id, pipe2)
    local _, p3 = m3:get_pipe(handle.pipe_id)
    assert.are.equal(p3:available(), 5)
  end)

  it("destroy_pipe removes pipe", function()
    local m = IPC.Manager.new()
    local m2, handle = m:create_pipe()
    local st, m3 = m2:destroy_pipe(handle.pipe_id)
    assert.are.equal(st, "ok")
    local rs, _ = m3:get_pipe(handle.pipe_id)
    assert.are.equal(rs, "not_found")
  end)

  it("destroy_pipe returns not_found for missing pipe", function()
    local m = IPC.Manager.new()
    local st, _ = m:destroy_pipe(42)
    assert.are.equal(st, "not_found")
  end)

  it("create_message_queue creates and returns queue", function()
    local m = IPC.Manager.new()
    local m2, q = m:create_message_queue("worker_queue")
    assert.is_not_nil(q)
    assert.is_true(q:is_empty())
    local st, q2 = m2:get_message_queue("worker_queue")
    assert.are.equal(st, "ok")
    assert.is_not_nil(q2)
  end)

  it("create_message_queue is idempotent", function()
    local m = IPC.Manager.new()
    local m2, _ = m:create_message_queue("q")
    local _, q2 = m2:get_message_queue("q")
    local _, q2send = q2:send(1, "msg")
    local m3 = m2:update_message_queue("q", q2send)
    -- create again should not overwrite
    local m4, q3 = m3:create_message_queue("q")
    assert.are.equal(q3.message_count, 1)
  end)

  it("get_message_queue returns not_found for unknown name", function()
    local m = IPC.Manager.new()
    local st, _ = m:get_message_queue("no_such")
    assert.are.equal(st, "not_found")
  end)

  it("destroy_message_queue removes it", function()
    local m = IPC.Manager.new()
    local m2, _ = m:create_message_queue("q")
    local st, m3 = m2:destroy_message_queue("q")
    assert.are.equal(st, "ok")
    local rs, _ = m3:get_message_queue("q")
    assert.are.equal(rs, "not_found")
  end)

  it("create_shared_memory creates region", function()
    local m = IPC.Manager.new()
    local m2, s = m:create_shared_memory("seg1", 64, 1)
    assert.are.equal(s.region_size, 64)
    local st, s2 = m2:get_shared_memory("seg1")
    assert.are.equal(st, "ok")
    assert.are.equal(s2.region_size, 64)
  end)

  it("get_shared_memory returns not_found for unknown name", function()
    local m = IPC.Manager.new()
    local st, _ = m:get_shared_memory("no_seg")
    assert.are.equal(st, "not_found")
  end)

  it("update_shared_memory replaces region", function()
    local m = IPC.Manager.new()
    local m2, s = m:create_shared_memory("seg", 32, 1)
    local _, s2, _ = s:write(0, "data")
    local m3 = m2:update_shared_memory("seg", s2)
    local _, s3 = m3:get_shared_memory("seg")
    local _, data = s3:read(0, 4)
    assert.are.equal(data, "data")
  end)

  it("destroy_shared_memory removes region", function()
    local m = IPC.Manager.new()
    local m2, _ = m:create_shared_memory("seg", 32, 1)
    local st, m3 = m2:destroy_shared_memory("seg")
    assert.are.equal(st, "ok")
    local rs, _ = m3:get_shared_memory("seg")
    assert.are.equal(rs, "not_found")
  end)

  it("end-to-end pipe workflow", function()
    local m = IPC.Manager.new()
    local m2, handle = m:create_pipe(64)

    local st1, pipe = m2:get_pipe(handle.pipe_id)
    assert.are.equal(st1, "ok")

    local _, pipe2, _ = pipe:write("ping")
    local m3 = m2:update_pipe(handle.pipe_id, pipe2)

    local _, pipe3 = m3:get_pipe(handle.pipe_id)
    local _, pipe4, data = pipe3:read(4)
    assert.are.equal(data, "ping")

    local m4 = m3:update_pipe(handle.pipe_id, pipe4)
    local _, pipe5 = m4:get_pipe(handle.pipe_id)
    assert.are.equal(pipe5:available(), 0)
  end)
end)
