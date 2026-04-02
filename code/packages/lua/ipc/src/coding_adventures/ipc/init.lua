-- ============================================================================
-- ipc — Inter-Process Communication
-- ============================================================================
--
-- Processes are isolated by design.  Each process has its own address space,
-- its own file descriptors, its own registers.  This isolation is essential:
-- a buggy program cannot corrupt another program's memory, and a malicious
-- program cannot read another program's secrets.
--
-- But isolation creates a problem: **how do processes collaborate?**
--
--   - A web server forks worker processes that all share a request queue.
--   - A shell pipeline `ls | grep foo | wc -l` links three processes in sequence.
--   - A database uses shared memory so multiple workers can read cached pages.
--
-- **IPC (Inter-Process Communication)** is the set of kernel mechanisms that
-- let isolated processes exchange data.
--
-- ## Three IPC Mechanisms
--
-- This module implements three classic mechanisms, ordered from simplest to
-- most powerful:
--
--   1. **Pipes**         — unidirectional byte streams
--   2. **Message Queues** — FIFO queues of typed messages
--   3. **Shared Memory** — a region mapped into multiple address spaces
--
-- ## Analogy: Two People in Soundproofed Rooms
--
--   Pipe:          A pneumatic tube — stuff bytes in one end, they come out the other.
--   Message Queue: A shared mailbox — labeled envelopes, picked up by type.
--   Shared Memory: A whiteboard visible through a window to both rooms —
--                  fastest, but you must take turns or get garbled text.
--
-- ## Module Structure
--
--   ipc
--   ├── Pipe              — circular buffer byte stream
--   ├── Message           — typed message in a queue
--   ├── MessageQueue      — FIFO of typed messages
--   ├── SharedMemory      — named memory region with byte read/write
--   └── Manager           — kernel IPC coordinator (pipes, queues, shm)
--
-- ============================================================================

local M = {}

-- Default constants (matching Elixir reference implementation)
M.DEFAULT_PIPE_CAPACITY    = 4096   -- one page
M.DEFAULT_MAX_MESSAGES     = 256
M.DEFAULT_MAX_MESSAGE_SIZE = 4096

-- ============================================================================
-- Pipe — Unidirectional Byte Stream
-- ============================================================================
--
-- A pipe is a circular buffer with two ends:
--   - Write end: one or more processes write bytes here.
--   - Read  end: one or more processes read bytes from here.
--
-- Data flows in one direction only.  To communicate in both directions,
-- create two pipes (one in each direction).
--
-- ### Circular Buffer Layout
--
--   ┌───┬───┬───┬───┬───┬───┬───┬───┐
--   │ h │ e │ l │ l │ o │   │   │   │   capacity = 8
--   └───┴───┴───┴───┴───┴───┴───┴───┘
--     ▲ read_pos=0          ▲ write_pos=5   count=5
--
-- After reading 3 bytes ("hel"):
--   read_pos=3, write_pos=5, count=2
--
-- After writing 4 more bytes ("worl") the write wraps around:
--   ┌───┬───┬───┬───┬───┬───┬───┬───┐
--   │ w │ o │ r │ l │ o │ l │ l │ o │  write_pos = (5+4) % 8 = 1
--   └───┴───┴───┴───┴───┴───┴───┴───┘
--
-- ### EOF and Broken Pipe
--
--   - All writers closed + buffer empty → pipe_read returns "eof"
--   - All readers closed → pipe_write returns "broken_pipe"
--
-- ### Fields
--
--   buffer        Table of bytes (1-indexed, capacity entries)
--   capacity      Maximum bytes the buffer can hold
--   read_pos      Next byte to read (0-based)
--   write_pos     Next byte to write (0-based)
--   count         Bytes currently buffered
--   reader_count  Number of open read-end file descriptors
--   writer_count  Number of open write-end file descriptors

M.Pipe = {}
M.Pipe.__index = M.Pipe

--- Create a new pipe with the given capacity.
-- @param capacity  Buffer size in bytes (default 4096)
function M.Pipe.new(capacity)
  capacity = capacity or M.DEFAULT_PIPE_CAPACITY
  if capacity <= 0 then error("Pipe capacity must be positive") end
  local buf = {}
  for i = 1, capacity do buf[i] = 0 end
  return setmetatable({
    buffer       = buf,
    capacity     = capacity,
    read_pos     = 0,
    write_pos    = 0,
    count        = 0,
    reader_count = 1,
    writer_count = 1,
  }, M.Pipe)
end

-- Internal: copy a pipe (immutable-style)
local function copy_pipe(p)
  local buf = {}
  for i = 1, p.capacity do buf[i] = p.buffer[i] end
  return setmetatable({
    buffer       = buf,
    capacity     = p.capacity,
    read_pos     = p.read_pos,
    write_pos    = p.write_pos,
    count        = p.count,
    reader_count = p.reader_count,
    writer_count = p.writer_count,
  }, M.Pipe)
end

--- Write bytes to the pipe.
-- @param bytes  String of bytes to write
-- @return "ok", updated_pipe, bytes_written
--      OR "full", updated_pipe, bytes_written  (partial write)
--      OR "broken_pipe", self, 0               (no readers)
function M.Pipe:write(bytes)
  if self.reader_count <= 0 then
    return "broken_pipe", self, 0
  end
  local p = copy_pipe(self)
  local written = 0
  for i = 1, #bytes do
    if p.count >= p.capacity then break end
    local b = string.byte(bytes, i)
    local idx = (p.write_pos % p.capacity) + 1
    p.buffer[idx] = b
    p.write_pos = p.write_pos + 1
    p.count = p.count + 1
    written = written + 1
  end
  if written < #bytes then
    return "full", p, written
  end
  return "ok", p, written
end

--- Read up to max_bytes bytes from the pipe.
-- @param max_bytes  Maximum number of bytes to read
-- @return "ok", updated_pipe, bytes_string
--      OR "eof", self, ""   (writers gone + buffer empty)
--      OR "empty", self, "" (buffer empty but writers still open)
function M.Pipe:read(max_bytes)
  if self.count == 0 then
    if self.writer_count <= 0 then
      return "eof", self, ""
    end
    return "empty", self, ""
  end
  local p = copy_pipe(self)
  local result = {}
  local to_read = math.min(max_bytes, p.count)
  for i = 1, to_read do
    local idx = (p.read_pos % p.capacity) + 1
    result[i] = string.char(p.buffer[idx])
    p.read_pos = p.read_pos + 1
    p.count = p.count - 1
  end
  return "ok", p, table.concat(result)
end

--- Close the read end of the pipe.
function M.Pipe:close_read()
  local p = copy_pipe(self)
  p.reader_count = math.max(0, p.reader_count - 1)
  return p
end

--- Close the write end of the pipe.
function M.Pipe:close_write()
  local p = copy_pipe(self)
  p.writer_count = math.max(0, p.writer_count - 1)
  return p
end

--- How many bytes are available to read?
function M.Pipe:available()
  return self.count
end

--- Is the buffer full?
function M.Pipe:is_full()
  return self.count >= self.capacity
end

--- Is the buffer empty?
function M.Pipe:is_empty()
  return self.count == 0
end

-- ============================================================================
-- Message / MessageQueue — Typed FIFO Queue
-- ============================================================================
--
-- Unlike pipes (byte streams), message queues preserve message boundaries.
-- Each message has:
--   - msg_type: a positive integer you choose (e.g., 1=command, 2=reply)
--   - body:     a string payload
--
-- Receivers can filter by type:
--   mq_receive(queue, 0)   → any message (oldest first)
--   mq_receive(queue, 2)   → oldest message of type 2
--
-- ### Capacity Limits
--
--   max_messages:     256   — prevents memory exhaustion by fast senders
--   max_message_size: 4096  — one page; larger data should use shared memory

--- Create a new typed message.
-- @param msg_type  Positive integer type tag
-- @param body      String payload
function M.message_new(msg_type, body)
  body = body or ""
  return {
    msg_type = msg_type,
    body     = body,
    msg_size = #body,
  }
end

M.MessageQueue = {}
M.MessageQueue.__index = M.MessageQueue

--- Create a new message queue.
-- @param max_messages      Maximum messages before queue is full (default 256)
-- @param max_message_size  Maximum bytes per message (default 4096)
function M.MessageQueue.new(max_messages, max_message_size)
  return setmetatable({
    messages         = {},            -- array acting as FIFO (front=index 1)
    message_count    = 0,
    max_messages     = max_messages     or M.DEFAULT_MAX_MESSAGES,
    max_message_size = max_message_size or M.DEFAULT_MAX_MESSAGE_SIZE,
  }, M.MessageQueue)
end

-- Internal: copy a queue
local function copy_mq(q)
  local msgs = {}
  for i, v in ipairs(q.messages) do msgs[i] = v end
  return setmetatable({
    messages         = msgs,
    message_count    = q.message_count,
    max_messages     = q.max_messages,
    max_message_size = q.max_message_size,
  }, M.MessageQueue)
end

--- Send a message to the queue.
-- @param msg_type  Message type tag (positive integer)
-- @param data      String payload
-- @return "ok", updated_queue  OR  "oversized", self  OR  "full", self
function M.MessageQueue:send(msg_type, data)
  data = data or ""
  if #data > self.max_message_size then
    return "oversized", self
  end
  if self.message_count >= self.max_messages then
    return "full", self
  end
  local q = copy_mq(self)
  local msg = M.message_new(msg_type, data)
  table.insert(q.messages, msg)
  q.message_count = q.message_count + 1
  return "ok", q
end

--- Receive a message from the queue.
-- @param msg_type  0 = any type; > 0 = specific type
-- @return "ok", updated_queue, message  OR  "empty", self, nil
function M.MessageQueue:receive(msg_type)
  if self.message_count == 0 then
    return "empty", self, nil
  end
  local q = copy_mq(self)
  if msg_type == 0 then
    -- Dequeue oldest message of any type
    local msg = table.remove(q.messages, 1)
    q.message_count = q.message_count - 1
    return "ok", q, msg
  else
    -- Find oldest message matching msg_type
    for i, msg in ipairs(q.messages) do
      if msg.msg_type == msg_type then
        table.remove(q.messages, i)
        q.message_count = q.message_count - 1
        return "ok", q, msg
      end
    end
    return "empty", self, nil
  end
end

--- Is the queue empty?
function M.MessageQueue:is_empty()
  return self.message_count == 0
end

--- Is the queue full?
function M.MessageQueue:is_full()
  return self.message_count >= self.max_messages
end

-- ============================================================================
-- SharedMemory — Zero-Copy Communication
-- ============================================================================
--
-- Shared memory is the fastest IPC mechanism because it eliminates copying.
-- Two processes map the same physical memory region into their virtual address
-- spaces.  A write by one process is immediately visible to the other.
--
-- **WARNING:** There is NO built-in synchronization.  Concurrent writes by
-- two processes cause race conditions.  Real programs must use semaphores or
-- mutexes to coordinate access.
--
-- ### Memory Layout
--
--   ┌──────────────────────────────────────────────────────────────┐
--   │  Shared Memory Region "shmem_buf" (size = 1024 bytes)        │
--   │  ┌──────────┬────────────────────────────────────────────┐   │
--   │  │ offset 0 │ 'H','e','l','l','o',' ','W','o','r','l','d'│   │
--   │  └──────────┴────────────────────────────────────────────┘   │
--   │                                                              │
--   │  Process A maps this region → reads/writes via shm_read/write│
--   │  Process B maps this region → sees the same bytes            │
--   └──────────────────────────────────────────────────────────────┘
--
-- ### Fields
--
--   region_name    Human-readable name (like a file path for mmap)
--   region_size    Total bytes in the region
--   data           Table of bytes (1-indexed)
--   owner_pid      PID of the process that created this region
--   attached_pids  Set of PIDs currently attached (shmat)

M.SharedMemory = {}
M.SharedMemory.__index = M.SharedMemory

--- Create a new shared memory region, zero-initialized.
-- @param region_name  Name for the region
-- @param region_size  Size in bytes (must be > 0)
-- @param owner_pid    PID of the creating process
function M.SharedMemory.new(region_name, region_size, owner_pid)
  if region_size <= 0 then error("Shared memory size must be positive") end
  local data = {}
  for i = 1, region_size do data[i] = 0 end
  local attached = {}
  return setmetatable({
    region_name   = region_name,
    region_size   = region_size,
    data          = data,
    owner_pid     = owner_pid,
    attached_pids = attached,
  }, M.SharedMemory)
end

-- Internal: copy shared memory
local function copy_shm(s)
  local data = {}
  for i = 1, s.region_size do data[i] = s.data[i] end
  local pids = {}
  for pid, _ in pairs(s.attached_pids) do pids[pid] = true end
  return setmetatable({
    region_name   = s.region_name,
    region_size   = s.region_size,
    data          = data,
    owner_pid     = s.owner_pid,
    attached_pids = pids,
  }, M.SharedMemory)
end

--- Attach a process to this shared memory region (like shmat).
-- @param pid  Process ID to attach
-- @return "ok", updated_region  OR  "already_attached", self
function M.SharedMemory:attach(pid)
  if self.attached_pids[pid] then
    return "already_attached", self
  end
  local s = copy_shm(self)
  s.attached_pids[pid] = true
  return "ok", s
end

--- Detach a process from the shared memory region (like shmdt).
-- @param pid  Process ID to detach
-- @return "ok", updated_region  OR  "not_attached", self
function M.SharedMemory:detach(pid)
  if not self.attached_pids[pid] then
    return "not_attached", self
  end
  local s = copy_shm(self)
  s.attached_pids[pid] = nil
  return "ok", s
end

--- Read bytes from the shared memory region.
-- @param offset      Start offset (0-based)
-- @param byte_count  Number of bytes to read
-- @return "ok", string  OR  "out_of_bounds", nil
function M.SharedMemory:read(offset, byte_count)
  if offset < 0 or byte_count < 0 or (offset + byte_count) > self.region_size then
    return "out_of_bounds", nil
  end
  local result = {}
  for i = 1, byte_count do
    result[i] = string.char(self.data[offset + i])
  end
  return "ok", table.concat(result)
end

--- Write bytes to the shared memory region.
-- @param offset  Start offset (0-based)
-- @param bytes   String of bytes to write
-- @return "ok", updated_region, bytes_written  OR  "out_of_bounds", self, 0
function M.SharedMemory:write(offset, bytes)
  local write_len = #bytes
  if offset < 0 or (offset + write_len) > self.region_size then
    return "out_of_bounds", self, 0
  end
  local s = copy_shm(self)
  for i = 1, write_len do
    s.data[offset + i] = string.byte(bytes, i)
  end
  return "ok", s, write_len
end

--- Number of currently attached processes.
function M.SharedMemory:attached_count()
  local n = 0
  for _ in pairs(self.attached_pids) do n = n + 1 end
  return n
end

--- Is a specific PID attached?
function M.SharedMemory:is_attached(pid)
  return self.attached_pids[pid] == true
end

-- ============================================================================
-- Manager — Kernel IPC Coordinator
-- ============================================================================
--
-- The Manager owns all IPC resources.  It provides the system call interface
-- for creating, accessing, and destroying pipes, message queues, and shared
-- memory regions.
--
-- Fields:
--   pipes          Map of pipe_id → Pipe
--   message_queues Map of name → MessageQueue
--   shared_regions Map of name → SharedMemory
--   next_pipe_id   Counter for unique pipe IDs
--   next_fd        Counter for unique file descriptors

M.Manager = {}
M.Manager.__index = M.Manager

--- Create a new IPC Manager.
function M.Manager.new()
  return setmetatable({
    pipes          = {},
    message_queues = {},
    shared_regions = {},
    next_pipe_id   = 0,
    next_fd        = 100,
  }, M.Manager)
end

-- Internal: copy manager shallowly (for immutable-style updates)
local function copy_mgr(m)
  local pipes = {}
  for k, v in pairs(m.pipes) do pipes[k] = v end
  local mqs = {}
  for k, v in pairs(m.message_queues) do mqs[k] = v end
  local shms = {}
  for k, v in pairs(m.shared_regions) do shms[k] = v end
  return setmetatable({
    pipes          = pipes,
    message_queues = mqs,
    shared_regions = shms,
    next_pipe_id   = m.next_pipe_id,
    next_fd        = m.next_fd,
  }, M.Manager)
end

--- Create a new pipe.
-- Returns updated_manager, pipe_handle
-- pipe_handle = { pipe_id, read_fd, write_fd }
function M.Manager:create_pipe(capacity)
  capacity = capacity or M.DEFAULT_PIPE_CAPACITY
  local mgr = copy_mgr(self)
  local pipe     = M.Pipe.new(capacity)
  local pipe_id  = mgr.next_pipe_id
  local read_fd  = mgr.next_fd
  local write_fd = mgr.next_fd + 1
  mgr.pipes[pipe_id] = pipe
  mgr.next_pipe_id = pipe_id + 1
  mgr.next_fd = write_fd + 1
  local handle = { pipe_id = pipe_id, read_fd = read_fd, write_fd = write_fd }
  return mgr, handle
end

--- Get a pipe by ID.
-- Returns "ok", pipe  OR  "not_found", nil
function M.Manager:get_pipe(pipe_id)
  local p = self.pipes[pipe_id]
  if p then return "ok", p end
  return "not_found", nil
end

--- Update (replace) a pipe in the manager after read/write.
function M.Manager:update_pipe(pipe_id, pipe)
  local mgr = copy_mgr(self)
  mgr.pipes[pipe_id] = pipe
  return mgr
end

--- Destroy a pipe.
-- Returns "ok", updated_manager  OR  "not_found", self
function M.Manager:destroy_pipe(pipe_id)
  if not self.pipes[pipe_id] then return "not_found", self end
  local mgr = copy_mgr(self)
  mgr.pipes[pipe_id] = nil
  return "ok", mgr
end

--- Create or get a message queue by name.
function M.Manager:create_message_queue(name, max_messages, max_message_size)
  local mgr = copy_mgr(self)
  if not mgr.message_queues[name] then
    mgr.message_queues[name] = M.MessageQueue.new(max_messages, max_message_size)
  end
  return mgr, mgr.message_queues[name]
end

--- Get a message queue by name.
-- Returns "ok", queue  OR  "not_found", nil
function M.Manager:get_message_queue(name)
  local q = self.message_queues[name]
  if q then return "ok", q end
  return "not_found", nil
end

--- Update a message queue in the manager.
function M.Manager:update_message_queue(name, queue)
  local mgr = copy_mgr(self)
  mgr.message_queues[name] = queue
  return mgr
end

--- Destroy a message queue.
function M.Manager:destroy_message_queue(name)
  if not self.message_queues[name] then return "not_found", self end
  local mgr = copy_mgr(self)
  mgr.message_queues[name] = nil
  return "ok", mgr
end

--- Create a shared memory region.
function M.Manager:create_shared_memory(name, size, owner_pid)
  local mgr = copy_mgr(self)
  if not mgr.shared_regions[name] then
    mgr.shared_regions[name] = M.SharedMemory.new(name, size, owner_pid)
  end
  return mgr, mgr.shared_regions[name]
end

--- Get a shared memory region by name.
function M.Manager:get_shared_memory(name)
  local s = self.shared_regions[name]
  if s then return "ok", s end
  return "not_found", nil
end

--- Update a shared memory region in the manager.
function M.Manager:update_shared_memory(name, region)
  local mgr = copy_mgr(self)
  mgr.shared_regions[name] = region
  return mgr
end

--- Destroy a shared memory region.
function M.Manager:destroy_shared_memory(name)
  if not self.shared_regions[name] then return "not_found", self end
  local mgr = copy_mgr(self)
  mgr.shared_regions[name] = nil
  return "ok", mgr
end

return M
