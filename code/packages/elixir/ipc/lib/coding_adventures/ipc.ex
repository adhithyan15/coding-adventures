defmodule CodingAdventures.IPC do
  @moduledoc """
  # Inter-Process Communication (IPC)

  ## What Is IPC?

  Processes in an operating system are isolated by design. Each process has its
  own virtual address space, its own file descriptors, its own registers. This
  isolation is essential: a buggy program cannot corrupt another program's
  memory, and a malicious program cannot read another program's secrets.

  But isolation creates a problem: **how do processes collaborate?**

  - A web server might fork worker processes that share a request queue.
  - A shell pipeline like `ls | grep foo | wc -l` needs three processes to
    pass data in sequence.
  - A database might use shared memory so multiple query workers can read
    cached pages without copying.

  **Inter-Process Communication (IPC)** is the set of mechanisms the OS
  provides for processes to exchange data despite their isolation.

  This module implements three classic IPC mechanisms:

  1. **Pipes** — unidirectional byte streams (simplest)
  2. **Message Queues** — FIFO queues of typed messages (structured)
  3. **Shared Memory** — a region of memory mapped into multiple address
     spaces (fastest, zero-copy)

  ## Analogy

  Imagine two people in separate, soundproofed rooms:
  - A **pipe** is a pneumatic tube — stuff a message in one end, it comes
    out the other.
  - A **message queue** is a shared mailbox in the hallway — anyone can
    drop off or pick up labeled envelopes.
  - **Shared memory** is a window between rooms with a whiteboard visible
    to both — fastest, but you must take turns writing.
  """

  # ============================================================================
  # Default Constants
  # ============================================================================

  # Default pipe buffer size: one memory page (4096 bytes). This is the
  # traditional Unix pipe buffer size — a deliberate match to the page size
  # used by the virtual memory system.
  @default_pipe_capacity 4096

  # Message queue defaults. 256 messages max prevents a fast sender from
  # consuming all kernel memory. 4096 bytes per message matches one page.
  @default_max_messages 256
  @default_max_message_size 4096

  # Public accessors for constants (useful in tests).
  def default_pipe_capacity, do: @default_pipe_capacity
  def default_max_messages, do: @default_max_messages
  def default_max_message_size, do: @default_max_message_size

  # ============================================================================
  # Pipe — Unidirectional Byte Stream
  # ============================================================================

  @doc """
  ## Pipe: The Simplest IPC Mechanism

  A pipe is a unidirectional byte stream backed by a **circular buffer**.
  Data written to the write end appears at the read end, in order, exactly
  once. Think of it as a conveyor belt: items placed on one end come out
  the other in the same order, and once consumed they are gone.

  ### The Circular Buffer

  The buffer is a binary (Elixir's efficient byte container) with two
  position cursors:

      ┌───┬───┬───┬───┬───┬───┬───┬───┐
      │ h │ e │ l │ l │ o │   │   │   │   capacity = 8
      └───┴───┴───┴───┴───┴───┴───┴───┘
        ▲ read_pos=0          ▲ write_pos=5

  When write_pos reaches the end, it wraps to index 0 using modular
  arithmetic: `rem(write_pos + n, capacity)`.

  ### Reader/Writer Counts

  - **All writers closed + buffer empty** → EOF.
  - **All readers closed** → BrokenPipe error on write.

  ### Struct Fields

  | Field        | Description                                        |
  |--------------|----------------------------------------------------|
  | buffer       | Binary of `capacity` bytes (the circular buffer)   |
  | capacity     | Maximum bytes the buffer can hold                  |
  | read_pos     | Index of next byte to read                         |
  | write_pos    | Index of next byte to write                        |
  | count        | Bytes currently in the buffer                      |
  | reader_count | Number of open read-end file descriptors           |
  | writer_count | Number of open write-end file descriptors          |
  """
  defstruct buffer: <<>>,
            capacity: @default_pipe_capacity,
            read_pos: 0,
            write_pos: 0,
            count: 0,
            reader_count: 1,
            writer_count: 1

  @type t :: %__MODULE__{
          buffer: binary(),
          capacity: pos_integer(),
          read_pos: non_neg_integer(),
          write_pos: non_neg_integer(),
          count: non_neg_integer(),
          reader_count: non_neg_integer(),
          writer_count: non_neg_integer()
        }

  @doc """
  Create a new pipe with the given capacity (defaults to 4096).

  The buffer is initialized to all zeros — a convention matching how the
  OS kernel allocates zeroed pages for new pipe buffers.
  """
  @spec new_pipe(pos_integer()) :: t()
  def new_pipe(capacity \\ @default_pipe_capacity) do
    if capacity <= 0, do: raise("Pipe capacity must be positive")

    %__MODULE__{
      buffer: :binary.copy(<<0>>, capacity),
      capacity: capacity
    }
  end

  @doc """
  ## pipe_write(pipe, data)

  Write bytes into the pipe's circular buffer.

  ### Behavior Table

  | Readers alive? | Buffer has space? | Result                       |
  |----------------|-------------------|------------------------------|
  | No             | (any)             | {:error, :broken_pipe}       |
  | Yes            | Yes               | {:ok, pipe, bytes_written}   |
  | Yes            | No (full)         | {:ok, pipe, 0}               |
  | Yes            | Partial           | {:ok, pipe, partial_count}   |

  In a real OS, writing to a full pipe would block the process. Here we
  return the count of bytes actually written and let the caller retry.
  """
  @spec pipe_write(t(), binary()) :: {:ok, t(), non_neg_integer()} | {:error, :broken_pipe}
  def pipe_write(%__MODULE__{reader_count: 0}, _data), do: {:error, :broken_pipe}

  def pipe_write(%__MODULE__{} = pipe, data) when is_binary(data) do
    space_available = pipe.capacity - pipe.count
    bytes_to_write = min(byte_size(data), space_available)

    # Write bytes one at a time into the circular buffer, wrapping around.
    # We convert the binary to a list of bytes for indexed writing, then
    # reconstruct the binary. This is not the most efficient approach but
    # it is clear and correct — exactly what we want for educational code.
    updated_buffer =
      Enum.reduce(0..(bytes_to_write - 1)//1, pipe.buffer, fn i, buf ->
        pos = rem(pipe.write_pos + i, pipe.capacity)
        byte_val = :binary.at(data, i)
        replace_byte_at(buf, pos, byte_val)
      end)

    new_write_pos = rem(pipe.write_pos + bytes_to_write, pipe.capacity)

    updated_pipe = %{pipe |
      buffer: updated_buffer,
      write_pos: new_write_pos,
      count: pipe.count + bytes_to_write
    }

    {:ok, updated_pipe, bytes_to_write}
  end

  @doc """
  ## pipe_read(pipe, max_bytes)

  Read up to `max_bytes` bytes from the pipe's circular buffer.

  ### Behavior Table

  | Buffer has data? | Writers alive? | Result                    |
  |------------------|----------------|---------------------------|
  | Yes              | (any)          | {:ok, pipe, bytes}        |
  | No               | Yes            | {:ok, pipe, <<>>}  (block)|
  | No               | No             | {:ok, pipe, <<>>}  (EOF)  |
  """
  @spec pipe_read(t(), non_neg_integer()) :: {:ok, t(), binary()}
  def pipe_read(%__MODULE__{} = pipe, max_bytes) when max_bytes <= 0 do
    {:ok, pipe, <<>>}
  end

  def pipe_read(%__MODULE__{count: 0} = pipe, _max_bytes) do
    {:ok, pipe, <<>>}
  end

  def pipe_read(%__MODULE__{} = pipe, max_bytes) do
    bytes_to_read = min(max_bytes, pipe.count)

    # Extract bytes from the circular buffer by reading one at a time.
    result_bytes =
      for i <- 0..(bytes_to_read - 1) do
        pos = rem(pipe.read_pos + i, pipe.capacity)
        :binary.at(pipe.buffer, pos)
      end

    new_read_pos = rem(pipe.read_pos + bytes_to_read, pipe.capacity)

    updated_pipe = %{pipe |
      read_pos: new_read_pos,
      count: pipe.count - bytes_to_read
    }

    {:ok, updated_pipe, :binary.list_to_bin(result_bytes)}
  end

  @doc "Close the read end of the pipe. Decrements the reader count."
  @spec close_read(t()) :: t()
  def close_read(%__MODULE__{reader_count: 0} = pipe), do: pipe
  def close_read(%__MODULE__{} = pipe), do: %{pipe | reader_count: pipe.reader_count - 1}

  @doc "Close the write end of the pipe. Decrements the writer count."
  @spec close_write(t()) :: t()
  def close_write(%__MODULE__{writer_count: 0} = pipe), do: pipe
  def close_write(%__MODULE__{} = pipe), do: %{pipe | writer_count: pipe.writer_count - 1}

  @doc "Is the buffer empty?"
  @spec pipe_empty?(t()) :: boolean()
  def pipe_empty?(%__MODULE__{count: 0}), do: true
  def pipe_empty?(%__MODULE__{}), do: false

  @doc "Is the buffer completely full?"
  @spec pipe_full?(t()) :: boolean()
  def pipe_full?(%__MODULE__{count: count, capacity: cap}), do: count == cap

  @doc "Number of bytes available to read."
  @spec pipe_available(t()) :: non_neg_integer()
  def pipe_available(%__MODULE__{count: count}), do: count

  @doc "Number of bytes of free space for writing."
  @spec pipe_space(t()) :: non_neg_integer()
  def pipe_space(%__MODULE__{count: count, capacity: cap}), do: cap - count

  @doc """
  ## EOF Detection

  A pipe is at EOF when:
  1. All writers have closed (writer_count == 0), AND
  2. The buffer is empty (count == 0).

  If writers are closed but data remains, the pipe is NOT at EOF — the
  reader should consume the remaining data first.
  """
  @spec pipe_eof?(t()) :: boolean()
  def pipe_eof?(%__MODULE__{writer_count: 0, count: 0}), do: true
  def pipe_eof?(%__MODULE__{}), do: false

  @doc "Is writing broken (no readers)?"
  @spec pipe_broken?(t()) :: boolean()
  def pipe_broken?(%__MODULE__{reader_count: 0}), do: true
  def pipe_broken?(%__MODULE__{}), do: false

  # Replace a single byte in a binary at a given position.
  # This creates a new binary (Elixir binaries are immutable).
  defp replace_byte_at(bin, pos, byte_val) do
    <<before::binary-size(pos), _old::8, rest::binary>> = bin
    <<before::binary, byte_val::8, rest::binary>>
  end

  # ============================================================================
  # Message Queue — Structured Communication
  # ============================================================================

  # ## Message
  #
  # A message has three parts:
  #
  # | Field    | Purpose                                             |
  # |----------|-----------------------------------------------------|
  # | msg_type | Positive integer tag — receivers can filter by type  |
  # | body     | The payload as raw bytes (binary)                   |
  # | msg_size | Actual byte size of the body                        |
  defmodule Message do
    @moduledoc "A typed message in a message queue."
    defstruct [:msg_type, :body, :msg_size]

    @type t :: %__MODULE__{
            msg_type: pos_integer(),
            body: binary(),
            msg_size: non_neg_integer()
          }
  end

  # ## MessageQueue: FIFO of Typed Messages
  #
  # A message queue decouples senders and receivers. Unlike pipes:
  # - Any process can send to the queue.
  # - Messages have boundaries — you always get a complete message.
  # - Receivers can filter by message type.
  #
  # ### Capacity Limits
  #
  # - **max_messages (256):** Prevents memory exhaustion.
  # - **max_message_size (4096):** One page. Larger data should use shared memory.
  defmodule MessageQueue do
    @moduledoc "FIFO queue of typed messages with capacity limits."
    defstruct messages: :queue.new(),
              message_count: 0,
              max_messages: 256,
              max_message_size: 4096

    @type t :: %__MODULE__{
            messages: :queue.queue(),
            message_count: non_neg_integer(),
            max_messages: pos_integer(),
            max_message_size: pos_integer()
          }
  end

  @doc "Create a new message queue with the given capacity limits."
  @spec new_message_queue(pos_integer(), pos_integer()) :: MessageQueue.t()
  def new_message_queue(
        max_messages \\ @default_max_messages,
        max_message_size \\ @default_max_message_size
      ) do
    %MessageQueue{
      max_messages: max_messages,
      max_message_size: max_message_size
    }
  end

  @doc """
  ## mq_send(queue, msg_type, data)

  Send a message to the back of the queue.

  ### Validation

  1. `byte_size(data) <= max_message_size`? If too large → `:error`.
  2. `message_count < max_messages`? If full → `:error`.
  3. Push the message onto the FIFO.

  Returns `{:ok, updated_queue}` or `{:error, reason}`.
  """
  @spec mq_send(MessageQueue.t(), pos_integer(), binary()) ::
          {:ok, MessageQueue.t()} | {:error, :oversized | :full}
  def mq_send(%MessageQueue{} = mq, msg_type, data) when is_binary(data) do
    cond do
      byte_size(data) > mq.max_message_size ->
        {:error, :oversized}

      mq.message_count >= mq.max_messages ->
        {:error, :full}

      true ->
        msg = %Message{msg_type: msg_type, body: data, msg_size: byte_size(data)}
        updated = %{mq |
          messages: :queue.in(msg, mq.messages),
          message_count: mq.message_count + 1
        }
        {:ok, updated}
    end
  end

  @doc """
  ## mq_receive(queue, msg_type)

  Receive (dequeue) a message from the queue.

  ### Type Filtering

  | msg_type | Behavior                                       |
  |----------|------------------------------------------------|
  | 0        | Return the oldest message of ANY type           |
  | > 0      | Return the oldest message matching this type    |

  Non-matching messages are skipped but NOT removed.
  """
  @spec mq_receive(MessageQueue.t(), non_neg_integer()) ::
          {:ok, MessageQueue.t(), Message.t()} | {:error, :empty}
  def mq_receive(%MessageQueue{message_count: 0}, _msg_type), do: {:error, :empty}

  def mq_receive(%MessageQueue{} = mq, 0) do
    # Type 0 = "give me anything." Dequeue the oldest.
    {{:value, msg}, remaining} = :queue.out(mq.messages)
    updated = %{mq | messages: remaining, message_count: mq.message_count - 1}
    {:ok, updated, msg}
  end

  def mq_receive(%MessageQueue{} = mq, msg_type) do
    # Search for the first message matching the requested type.
    # We convert to a list, find the match, remove it, and rebuild.
    msg_list = :queue.to_list(mq.messages)

    case Enum.find_index(msg_list, fn m -> m.msg_type == msg_type end) do
      nil ->
        {:error, :empty}

      idx ->
        {msg, rest} = List.pop_at(msg_list, idx)
        updated = %{mq |
          messages: :queue.from_list(rest),
          message_count: mq.message_count - 1
        }
        {:ok, updated, msg}
    end
  end

  @doc "Is the queue empty?"
  @spec mq_empty?(MessageQueue.t()) :: boolean()
  def mq_empty?(%MessageQueue{message_count: 0}), do: true
  def mq_empty?(%MessageQueue{}), do: false

  @doc "Is the queue full?"
  @spec mq_full?(MessageQueue.t()) :: boolean()
  def mq_full?(%MessageQueue{message_count: c, max_messages: m}), do: c >= m

  # ============================================================================
  # Shared Memory — Zero-Copy Communication
  # ============================================================================

  # ## SharedMemoryRegion: The Fastest IPC
  #
  # Shared memory eliminates copying. Two processes map the same physical
  # pages into their virtual address spaces. A write by one process is
  # immediately visible to the other.
  #
  # **WARNING:** No built-in synchronization. Concurrent writes cause race
  # conditions. Real programs must use semaphores or mutexes.
  #
  # ### Struct Fields
  #
  # | Field         | Description                                       |
  # |---------------|---------------------------------------------------|
  # | region_name   | Human-readable name for this segment              |
  # | region_size   | Size in bytes                                     |
  # | data          | The shared bytes (binary)                         |
  # | owner_pid     | PID of the creator                                |
  # | attached_pids | MapSet of currently attached process IDs          |
  defmodule SharedMemoryRegion do
    @moduledoc "A named shared memory region with PID-based access control."
    defstruct [:region_name, :region_size, :data, :owner_pid, attached_pids: MapSet.new()]

    @type t :: %__MODULE__{
            region_name: String.t(),
            region_size: pos_integer(),
            data: binary(),
            owner_pid: non_neg_integer(),
            attached_pids: MapSet.t(non_neg_integer())
          }
  end

  @doc "Create a new shared memory region."
  @spec new_shared_memory(String.t(), pos_integer(), non_neg_integer()) :: SharedMemoryRegion.t()
  def new_shared_memory(region_name, region_size, owner_pid) do
    if region_size <= 0, do: raise("Shared memory size must be positive")

    %SharedMemoryRegion{
      region_name: region_name,
      region_size: region_size,
      data: :binary.copy(<<0>>, region_size),
      owner_pid: owner_pid
    }
  end

  @doc """
  Attach a process to the shared memory region.

  Returns `{:ok, updated_region}` if newly attached, `{:error, :already_attached}` otherwise.
  """
  @spec shm_attach(SharedMemoryRegion.t(), non_neg_integer()) ::
          {:ok, SharedMemoryRegion.t()} | {:error, :already_attached}
  def shm_attach(%SharedMemoryRegion{} = region, attached_pid) do
    if MapSet.member?(region.attached_pids, attached_pid) do
      {:error, :already_attached}
    else
      updated = %{region | attached_pids: MapSet.put(region.attached_pids, attached_pid)}
      {:ok, updated}
    end
  end

  @doc """
  Detach a process from the shared memory region.

  Returns `{:ok, updated_region}` if successfully detached, `{:error, :not_attached}` otherwise.
  """
  @spec shm_detach(SharedMemoryRegion.t(), non_neg_integer()) ::
          {:ok, SharedMemoryRegion.t()} | {:error, :not_attached}
  def shm_detach(%SharedMemoryRegion{} = region, detach_pid) do
    if MapSet.member?(region.attached_pids, detach_pid) do
      updated = %{region | attached_pids: MapSet.delete(region.attached_pids, detach_pid)}
      {:ok, updated}
    else
      {:error, :not_attached}
    end
  end

  @doc """
  ## shm_read(region, offset, byte_count)

  Read `byte_count` bytes starting at `offset`. Bounds checking is
  performed — reading past the end raises an error (analogous to a
  segfault).
  """
  @spec shm_read(SharedMemoryRegion.t(), non_neg_integer(), non_neg_integer()) ::
          {:ok, binary()} | {:error, :out_of_bounds}
  def shm_read(%SharedMemoryRegion{} = region, offset, byte_count) do
    if offset < 0 or byte_count < 0 or offset + byte_count > region.region_size do
      {:error, :out_of_bounds}
    else
      {:ok, binary_part(region.data, offset, byte_count)}
    end
  end

  @doc """
  ## shm_write(region, offset, write_data)

  Write bytes starting at `offset`. Bounds checking is performed.

  **WARNING:** No synchronization. Concurrent writes cause race conditions.
  """
  @spec shm_write(SharedMemoryRegion.t(), non_neg_integer(), binary()) ::
          {:ok, SharedMemoryRegion.t(), non_neg_integer()} | {:error, :out_of_bounds}
  def shm_write(%SharedMemoryRegion{} = region, offset, write_data) when is_binary(write_data) do
    write_len = byte_size(write_data)

    if offset < 0 or offset + write_len > region.region_size do
      {:error, :out_of_bounds}
    else
      <<before::binary-size(offset), _old::binary-size(write_len), rest::binary>> = region.data
      new_data = <<before::binary, write_data::binary, rest::binary>>
      {:ok, %{region | data: new_data}, write_len}
    end
  end

  @doc "Number of attached processes."
  @spec shm_attached_count(SharedMemoryRegion.t()) :: non_neg_integer()
  def shm_attached_count(%SharedMemoryRegion{attached_pids: pids}), do: MapSet.size(pids)

  @doc "Is a specific PID attached?"
  @spec shm_attached?(SharedMemoryRegion.t(), non_neg_integer()) :: boolean()
  def shm_attached?(%SharedMemoryRegion{attached_pids: pids}, check_pid) do
    MapSet.member?(pids, check_pid)
  end

  # ============================================================================
  # IPC Manager — The Kernel's IPC Coordinator
  # ============================================================================

  # ## PipeHandle
  #
  # Returned when creating a pipe. Contains the pipe ID and two file
  # descriptors (read and write).
  defmodule PipeHandle do
    @moduledoc "Handle returned by create_pipe with pipe_id, read_fd, and write_fd."
    defstruct [:pipe_id, :read_fd, :write_fd]

    @type t :: %__MODULE__{
            pipe_id: non_neg_integer(),
            read_fd: non_neg_integer(),
            write_fd: non_neg_integer()
          }
  end

  # ## IPCManager
  #
  # The kernel component that owns all IPC resources. It provides the
  # system call interface for creating, accessing, and destroying pipes,
  # message queues, and shared memory regions.
  #
  # ### Struct Fields
  #
  # | Field           | Description                                   |
  # |-----------------|-----------------------------------------------|
  # | pipes           | Map of pipe_id -> Pipe struct                 |
  # | message_queues  | Map of name -> MessageQueue struct             |
  # | shared_regions  | Map of name -> SharedMemoryRegion struct       |
  # | next_pipe_id    | Counter for unique pipe IDs                   |
  # | next_fd         | Counter for unique file descriptors            |
  defmodule Manager do
    @moduledoc "Kernel-level IPC resource coordinator."
    defstruct pipes: %{},
              message_queues: %{},
              shared_regions: %{},
              next_pipe_id: 0,
              next_fd: 100

    @type t :: %__MODULE__{
            pipes: %{non_neg_integer() => CodingAdventures.IPC.t()},
            message_queues: %{String.t() => CodingAdventures.IPC.MessageQueue.t()},
            shared_regions: %{String.t() => CodingAdventures.IPC.SharedMemoryRegion.t()},
            next_pipe_id: non_neg_integer(),
            next_fd: non_neg_integer()
          }
  end

  @doc "Create a new IPC manager."
  @spec new_manager() :: Manager.t()
  def new_manager, do: %Manager{}

  @doc "Create a new pipe. Returns {updated_manager, pipe_handle}."
  @spec create_pipe(Manager.t(), pos_integer()) :: {Manager.t(), PipeHandle.t()}
  def create_pipe(%Manager{} = mgr, capacity \\ @default_pipe_capacity) do
    pipe = new_pipe(capacity)
    pipe_id = mgr.next_pipe_id
    read_fd = mgr.next_fd
    write_fd = mgr.next_fd + 1

    handle = %PipeHandle{pipe_id: pipe_id, read_fd: read_fd, write_fd: write_fd}

    updated = %{mgr |
      pipes: Map.put(mgr.pipes, pipe_id, pipe),
      next_pipe_id: pipe_id + 1,
      next_fd: write_fd + 1
    }

    {updated, handle}
  end

  @doc "Get a pipe by ID. Returns {:ok, pipe} or {:error, :not_found}."
  @spec get_pipe(Manager.t(), non_neg_integer()) :: {:ok, t()} | {:error, :not_found}
  def get_pipe(%Manager{pipes: pipes}, pipe_id) do
    case Map.fetch(pipes, pipe_id) do
      {:ok, pipe} -> {:ok, pipe}
      :error -> {:error, :not_found}
    end
  end

  @doc "Update a pipe in the manager (after read/write operations)."
  @spec update_pipe(Manager.t(), non_neg_integer(), t()) :: Manager.t()
  def update_pipe(%Manager{} = mgr, pipe_id, %__MODULE__{} = pipe) do
    %{mgr | pipes: Map.put(mgr.pipes, pipe_id, pipe)}
  end

  @doc "Close the read end of a pipe."
  @spec close_pipe_read(Manager.t(), non_neg_integer()) ::
          {:ok, Manager.t()} | {:error, :not_found}
  def close_pipe_read(%Manager{} = mgr, pipe_id) do
    case get_pipe(mgr, pipe_id) do
      {:ok, pipe} ->
        updated_pipe = close_read(pipe)
        {:ok, update_pipe(mgr, pipe_id, updated_pipe)}

      err ->
        err
    end
  end

  @doc "Close the write end of a pipe."
  @spec close_pipe_write(Manager.t(), non_neg_integer()) ::
          {:ok, Manager.t()} | {:error, :not_found}
  def close_pipe_write(%Manager{} = mgr, pipe_id) do
    case get_pipe(mgr, pipe_id) do
      {:ok, pipe} ->
        updated_pipe = close_write(pipe)
        {:ok, update_pipe(mgr, pipe_id, updated_pipe)}

      err ->
        err
    end
  end

  @doc "Destroy a pipe, removing it from the manager."
  @spec destroy_pipe(Manager.t(), non_neg_integer()) ::
          {:ok, Manager.t()} | {:error, :not_found}
  def destroy_pipe(%Manager{} = mgr, pipe_id) do
    if Map.has_key?(mgr.pipes, pipe_id) do
      {:ok, %{mgr | pipes: Map.delete(mgr.pipes, pipe_id)}}
    else
      {:error, :not_found}
    end
  end

  @doc """
  Create a message queue with the given name. If a queue with this name
  already exists, return it (idempotent, like msgget).
  """
  @spec create_message_queue(Manager.t(), String.t(), pos_integer(), pos_integer()) ::
          {Manager.t(), MessageQueue.t()}
  def create_message_queue(
        %Manager{} = mgr,
        queue_name,
        max_messages \\ @default_max_messages,
        max_message_size \\ @default_max_message_size
      ) do
    case Map.fetch(mgr.message_queues, queue_name) do
      {:ok, existing} ->
        {mgr, existing}

      :error ->
        mq = new_message_queue(max_messages, max_message_size)
        updated = %{mgr | message_queues: Map.put(mgr.message_queues, queue_name, mq)}
        {updated, mq}
    end
  end

  @doc "Get a message queue by name."
  @spec get_message_queue(Manager.t(), String.t()) ::
          {:ok, MessageQueue.t()} | {:error, :not_found}
  def get_message_queue(%Manager{message_queues: queues}, queue_name) do
    case Map.fetch(queues, queue_name) do
      {:ok, mq} -> {:ok, mq}
      :error -> {:error, :not_found}
    end
  end

  @doc "Update a message queue in the manager (after send/receive)."
  @spec update_message_queue(Manager.t(), String.t(), MessageQueue.t()) :: Manager.t()
  def update_message_queue(%Manager{} = mgr, queue_name, %MessageQueue{} = mq) do
    %{mgr | message_queues: Map.put(mgr.message_queues, queue_name, mq)}
  end

  @doc "Delete a message queue by name."
  @spec delete_message_queue(Manager.t(), String.t()) ::
          {:ok, Manager.t()} | {:error, :not_found}
  def delete_message_queue(%Manager{} = mgr, queue_name) do
    if Map.has_key?(mgr.message_queues, queue_name) do
      {:ok, %{mgr | message_queues: Map.delete(mgr.message_queues, queue_name)}}
    else
      {:error, :not_found}
    end
  end

  @doc """
  Create a shared memory region. If one with this name already exists,
  return it (idempotent, like shmget).
  """
  @spec create_shared_memory(Manager.t(), String.t(), pos_integer(), non_neg_integer()) ::
          {Manager.t(), SharedMemoryRegion.t()}
  def create_shared_memory(%Manager{} = mgr, shm_name, shm_size, owner_pid) do
    case Map.fetch(mgr.shared_regions, shm_name) do
      {:ok, existing} ->
        {mgr, existing}

      :error ->
        region = new_shared_memory(shm_name, shm_size, owner_pid)
        updated = %{mgr | shared_regions: Map.put(mgr.shared_regions, shm_name, region)}
        {updated, region}
    end
  end

  @doc "Get a shared memory region by name."
  @spec get_shared_memory(Manager.t(), String.t()) ::
          {:ok, SharedMemoryRegion.t()} | {:error, :not_found}
  def get_shared_memory(%Manager{shared_regions: regions}, shm_name) do
    case Map.fetch(regions, shm_name) do
      {:ok, region} -> {:ok, region}
      :error -> {:error, :not_found}
    end
  end

  @doc "Update a shared memory region in the manager."
  @spec update_shared_memory(Manager.t(), String.t(), SharedMemoryRegion.t()) :: Manager.t()
  def update_shared_memory(%Manager{} = mgr, shm_name, %SharedMemoryRegion{} = region) do
    %{mgr | shared_regions: Map.put(mgr.shared_regions, shm_name, region)}
  end

  @doc "Delete a shared memory region by name."
  @spec delete_shared_memory(Manager.t(), String.t()) ::
          {:ok, Manager.t()} | {:error, :not_found}
  def delete_shared_memory(%Manager{} = mgr, shm_name) do
    if Map.has_key?(mgr.shared_regions, shm_name) do
      {:ok, %{mgr | shared_regions: Map.delete(mgr.shared_regions, shm_name)}}
    else
      {:error, :not_found}
    end
  end

  @doc "List all active pipe IDs."
  @spec list_pipes(Manager.t()) :: [non_neg_integer()]
  def list_pipes(%Manager{pipes: pipes}), do: Map.keys(pipes)

  @doc "List all message queue names."
  @spec list_message_queues(Manager.t()) :: [String.t()]
  def list_message_queues(%Manager{message_queues: queues}), do: Map.keys(queues)

  @doc "List all shared memory region names."
  @spec list_shared_regions(Manager.t()) :: [String.t()]
  def list_shared_regions(%Manager{shared_regions: regions}), do: Map.keys(regions)
end
