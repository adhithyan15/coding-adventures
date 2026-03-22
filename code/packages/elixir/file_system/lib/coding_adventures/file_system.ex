defmodule CodingAdventures.FileSystem do
  @moduledoc """
  # File System — An Inode-Based File System (ext2-Inspired)

  ## What Is a File System?

  A file system is the abstraction that turns a raw disk — billions of identical
  bytes with no structure — into the familiar world of files and directories.
  Without a file system, every program would need to remember "my data starts at
  byte 4,194,304 and is 8,192 bytes long." With a file system, you just say
  `open("/home/alice/notes.txt")` and the OS figures out the rest.

  **Analogy:** Think of a library. The *disk* is the building full of shelves.
  The *file system* is the cataloging system — the card catalog (inode table),
  the Dewey Decimal numbers (block pointers), the shelf labels (directories),
  and the checkout desk (file descriptors). Without the cataloging system, you
  would have a warehouse of unlabeled books with no way to find anything.

  ## Architecture

      VFS (Virtual File System)
      ├── Path Resolution    — "/" → inode 0 → "data" → inode 5 → "log.txt" → inode 23
      ├── Inode Table        — metadata for every file/directory
      ├── Block Bitmap       — which disk blocks are free/used
      ├── Open File Table    — system-wide table of open files
      ├── FD Table           — per-process mapping: local fd → open file entry
      └── Superblock         — file system metadata (sizes, counts, magic number)
  """

  # ============================================================================
  # Constants
  # ============================================================================

  # ## Disk Geometry Constants
  #
  # These constants define the physical layout of our simulated disk. We use
  # 512-byte blocks (the traditional hard disk sector size) and a total disk
  # size of 512 blocks = 256 KB. This is tiny by modern standards but large
  # enough to demonstrate every file system concept.
  @block_size 512
  @max_blocks 512
  @max_inodes 128
  @direct_blocks 12
  @root_inode 0
  @unallocated -1

  # The superblock magic number: 0x45585432 (ASCII 'EXT2')
  @magic 0x45585432

  # Public accessors for constants (useful in tests)
  def block_size, do: @block_size
  def max_blocks, do: @max_blocks
  def max_inodes, do: @max_inodes
  def direct_blocks_count, do: @direct_blocks
  def root_inode, do: @root_inode

  # ============================================================================
  # File Types
  # ============================================================================

  @typedoc """
  ## FileType — What Kind of Object Does an Inode Represent?

  In Unix-style file systems, "everything is a file" — but not all files are
  created equal. We use atoms to represent file types in Elixir, which is
  more idiomatic than integer enums.

      File Type Values
      ════════════════
        :regular      (1) — ordinary file containing user data
        :directory    (2) — contains directory entries
        :symlink      (3) — symbolic link (stores a path string)
        :char_device  (4) — character device (e.g., terminal)
        :block_device (5) — block device (e.g., disk)
        :pipe         (6) — named pipe / FIFO
        :socket       (7) — Unix domain socket
  """
  @type file_type ::
          :regular | :directory | :symlink | :char_device | :block_device | :pipe | :socket

  # ============================================================================
  # Open Flags and Seek Modes
  # ============================================================================

  # ## Open Flags — How Should the File Be Opened?
  #
  # We use atoms for flags in Elixir instead of bitmask integers. The VFS
  # accepts a list of flag atoms.
  #
  #     Flags:
  #       :rdonly  — read only
  #       :wronly  — write only
  #       :rdwr   — read and write
  #       :creat  — create if missing
  #       :trunc  — truncate to zero length
  #       :append — writes go to end of file

  # ## Seek Modes
  #
  #     :set — offset is absolute (from start of file)
  #     :cur — offset is relative to current position
  #     :seek_end — offset is relative to end of file
  #
  # Note: we use :seek_end instead of :end because `end` is a reserved word
  # in Elixir (it closes do/end blocks).

  # ============================================================================
  # Data Structures
  # ============================================================================

  defmodule Superblock do
    @moduledoc """
    ## Superblock — The File System's Identity Card

    The superblock is stored at block 0. It contains the metadata needed to
    mount the file system.

        Superblock Layout
        ═════════════════
          magic        — 0x45585432 ("EXT2") validates format
          block_size   — 512 bytes per block
          total_blocks — 512 blocks total on disk
          total_inodes — 128 inodes (max files/directories)
          free_blocks  — currently unallocated data blocks
          free_inodes  — currently unallocated inodes
    """
    defstruct magic: 0x45585432,
              block_size: 512,
              total_blocks: 512,
              total_inodes: 128,
              free_blocks: 0,
              free_inodes: 0
  end

  defmodule Inode do
    @moduledoc """
    ## Inode — The Heart of the File System

    An inode (index node) stores everything about a file *except its name*.
    Names live in directories, not in files. A file can have multiple names
    (hard links) pointing to the same inode.

        Inode Structure
        ═══════════════
          inode_number   — unique ID (0–127), inode 0 = root directory "/"
          file_type      — :regular, :directory, :symlink, etc.
          size           — file size in bytes
          permissions    — octal permission bits (e.g., 0o755)
          owner_pid      — PID of the creating process
          link_count     — number of directory entries pointing here
          direct_blocks  — list of 12 block numbers for file data
          indirect_block — block number of an indirect pointer block (-1 = none)
          created_at     — creation timestamp
          modified_at    — last modification timestamp
          accessed_at    — last access timestamp
    """
    defstruct inode_number: 0,
              file_type: :regular,
              size: 0,
              permissions: 0o755,
              owner_pid: 0,
              link_count: 1,
              direct_blocks: List.duplicate(-1, 12),
              indirect_block: -1,
              created_at: 0,
              modified_at: 0,
              accessed_at: 0
  end

  defmodule DirectoryEntry do
    @moduledoc """
    ## DirectoryEntry — Mapping Names to Inodes

    A directory's data blocks contain a list of these entries. Each entry
    maps a human-readable name to an inode number.

        Example: root directory "/"
          ┌───────────┬─────────────┐
          │ name      │ inode_number │
          ├───────────┼─────────────┤
          │ "."       │ 0           │
          │ ".."      │ 0           │
          │ "home"    │ 5           │
          └───────────┴─────────────┘
    """
    defstruct name: "", inode_number: 0
  end

  defmodule OpenFile do
    @moduledoc """
    ## OpenFile — A System-Wide Entry for an Open File

    Tracks the inode, current offset, access flags, and reference count
    for each open file. Multiple file descriptors can point to the same
    OpenFile entry (e.g., after dup or fork).
    """
    defstruct inode_number: 0, offset: 0, flags: :rdonly, ref_count: 1
  end

  # ============================================================================
  # Directory Entry Serialization
  # ============================================================================

  @doc """
  Serializes a list of DirectoryEntry structs into a binary.

  Each entry is encoded as `name <null byte> inode_number <newline>`.
  For example: `"." <0> "0" <\\n> ".." <0> "0" <\\n>`
  """
  def serialize_entries(entries) do
    entries
    |> Enum.map(fn %DirectoryEntry{name: entry_name, inode_number: inum} ->
      "#{entry_name}\0#{inum}\n"
    end)
    |> Enum.join("")
  end

  @doc """
  Deserializes a binary back into a list of DirectoryEntry structs.
  This is the inverse of `serialize_entries/1`.
  """
  def deserialize_entries(data) when is_binary(data) do
    data
    |> String.split("\n", trim: true)
    |> Enum.map(fn line ->
      case String.split(line, "\0", parts: 2) do
        [entry_name, inum_str] ->
          case Integer.parse(inum_str) do
            {inum, _} -> %DirectoryEntry{name: entry_name, inode_number: inum}
            :error -> nil
          end

        _ ->
          nil
      end
    end)
    |> Enum.reject(&is_nil/1)
  end

  # ============================================================================
  # VFS State
  # ============================================================================

  # ## VFS — The Virtual File System
  #
  # The VFS ties together all file system components. In Elixir, since data
  # is immutable, the VFS state is a map that gets threaded through all
  # operations. Each operation returns `{result, new_state}`.
  #
  # The state contains:
  # - `:blocks` — list of binaries (the simulated disk)
  # - `:superblock` — the Superblock struct
  # - `:inodes` — map of inode_number → Inode struct (nil = free)
  # - `:block_bitmap` — map of block_index → boolean (true = used)
  # - `:open_files` — map of global_index → OpenFile struct
  # - `:fd_tables` — map of pid → map of local_fd → global_index
  # - `:data_block_start` — index of the first data block
  # - `:total_data_blocks` — number of data blocks available
  # - `:next_open_file_id` — counter for allocating open file table entries

  @doc """
  ## format() — Initialize a Blank Disk

  Creates an empty file system with a root directory at inode 0.

      format()
      ════════
        1. Allocate MAX_BLOCKS blocks of BLOCK_SIZE bytes each
        2. Write the superblock to block 0
        3. Initialize the inode table (all free except inode 0)
        4. Initialize the block bitmap (all free except one for root)
        5. Create root directory with "." and ".." entries
  """
  def format do
    # Calculate disk layout
    inode_table_blocks = div(@max_inodes * 64 + @block_size - 1, @block_size)
    data_block_start = 1 + inode_table_blocks + 1
    total_data_blocks = @max_blocks - data_block_start

    # Initialize empty blocks
    blocks =
      for _ <- 0..(@max_blocks - 1) do
        :binary.copy(<<0>>, @block_size)
      end

    # Create root inode
    now = System.system_time(:millisecond)

    root_inode = %Inode{
      inode_number: @root_inode,
      file_type: :directory,
      size: 0,
      permissions: 0o755,
      owner_pid: 0,
      link_count: 2,
      direct_blocks: List.duplicate(@unallocated, @direct_blocks),
      indirect_block: @unallocated,
      created_at: now,
      modified_at: now,
      accessed_at: now
    }

    # Allocate data block 0 for root directory entries
    root_block_idx = 0
    root_inode = %{root_inode | direct_blocks: List.replace_at(root_inode.direct_blocks, 0, root_block_idx)}

    # Serialize root directory entries ("." and ".." both point to inode 0)
    root_entries = [
      %DirectoryEntry{name: ".", inode_number: @root_inode},
      %DirectoryEntry{name: "..", inode_number: @root_inode}
    ]

    root_data = serialize_entries(root_entries)
    root_inode = %{root_inode | size: byte_size(root_data)}

    # Write root data to the block
    block_absolute = data_block_start + root_block_idx
    padded = root_data <> :binary.copy(<<0>>, @block_size - byte_size(root_data))
    blocks = List.replace_at(blocks, block_absolute, padded)

    # Initialize bitmap (block 0 is used by root)
    block_bitmap = Map.new(0..(total_data_blocks - 1), fn i -> {i, i == 0} end)

    # Initialize inodes map (only inode 0 is used)
    inodes = Map.new(0..(@max_inodes - 1), fn i ->
      if i == 0, do: {i, root_inode}, else: {i, nil}
    end)

    %{
      blocks: blocks,
      superblock: %Superblock{
        magic: @magic,
        block_size: @block_size,
        total_blocks: @max_blocks,
        total_inodes: @max_inodes,
        free_blocks: total_data_blocks - 1,
        free_inodes: @max_inodes - 1
      },
      inodes: inodes,
      block_bitmap: block_bitmap,
      open_files: %{},
      fd_tables: %{},
      data_block_start: data_block_start,
      total_data_blocks: total_data_blocks,
      next_open_file_id: 3
    }
  end

  @doc "Returns the superblock from the VFS state."
  def get_superblock(state), do: state.superblock

  # ============================================================================
  # Block Bitmap Operations
  # ============================================================================

  # ## Block Bitmap Operations
  #
  # The block bitmap uses one bit per data block to track allocation.
  #
  #     Block Bitmap (one bit per data block)
  #     ═════════════════════════════════════
  #       Bit index:   0   1   2   3   4   5
  #       Value:       T   T   F   F   T   F
  #                    ▲   ▲           ▲
  #                  used used       used

  defp allocate_block(state) do
    case Enum.find(0..(state.total_data_blocks - 1), fn i ->
           not Map.get(state.block_bitmap, i, false)
         end) do
      nil ->
        {nil, state}

      block_idx ->
        new_bitmap = Map.put(state.block_bitmap, block_idx, true)
        {block_idx, %{state | block_bitmap: new_bitmap}}
    end
  end

  defp free_block(state, block_idx) when block_idx >= 0 do
    new_bitmap = Map.put(state.block_bitmap, block_idx, false)
    %{state | block_bitmap: new_bitmap}
  end

  defp free_block(state, _), do: state

  defp block_free_count(state) do
    Enum.count(state.block_bitmap, fn {_, used} -> not used end)
  end

  @doc "Checks whether a specific block is free."
  def is_block_free(state, block_idx) do
    not Map.get(state.block_bitmap, block_idx, true)
  end

  # ============================================================================
  # Inode Table Operations
  # ============================================================================

  # ## Inode Table Operations
  #
  # The inode table manages all 128 inodes. A nil value means the slot is free.

  defp allocate_inode(state, file_type) do
    case Enum.find(0..(@max_inodes - 1), fn i -> Map.get(state.inodes, i) == nil end) do
      nil ->
        {nil, state}

      idx ->
        now = System.system_time(:millisecond)

        inode = %Inode{
          inode_number: idx,
          file_type: file_type,
          size: 0,
          permissions: 0o755,
          owner_pid: 0,
          link_count: 1,
          direct_blocks: List.duplicate(@unallocated, @direct_blocks),
          indirect_block: @unallocated,
          created_at: now,
          modified_at: now,
          accessed_at: now
        }

        new_inodes = Map.put(state.inodes, idx, inode)
        {inode, %{state | inodes: new_inodes}}
    end
  end

  defp free_inode(state, inode_number) do
    new_inodes = Map.put(state.inodes, inode_number, nil)
    %{state | inodes: new_inodes}
  end

  @doc "Returns the inode at the given number, or nil if the slot is free."
  def get_inode(state, inode_number) do
    Map.get(state.inodes, inode_number)
  end

  defp update_inode(state, inode) do
    new_inodes = Map.put(state.inodes, inode.inode_number, inode)
    %{state | inodes: new_inodes}
  end

  # ============================================================================
  # Path Resolution
  # ============================================================================

  @doc """
  ## resolve_path() — Turn a Path String into an Inode Number

  Given a path like "/home/alice/notes.txt", walks the directory tree
  from root inode 0, looking up each component.

      resolve_path("/home/alice")
      ═══════════════════════════
        Component  │ Current Inode │ Action
        (start)    │ 0 (root)      │ Begin at root
        "home"     │ 0 → 5         │ Found "home" → inode 5
        "alice"    │ 5 → 12        │ Found "alice" → inode 12
  """
  def resolve_path(_state, "/"), do: @root_inode

  def resolve_path(state, path_str) do
    components =
      path_str
      |> String.split("/")
      |> Enum.reject(&(&1 == ""))

    walk_path(state, components, @root_inode)
  end

  defp walk_path(_state, [], current_inode), do: current_inode

  defp walk_path(state, [component | remaining], current_inode) do
    inode = get_inode(state, current_inode)

    cond do
      inode == nil -> nil
      inode.file_type != :directory -> nil
      true ->
        entries = read_directory_entries(state, inode)

        case Enum.find(entries, fn entry -> entry.name == component end) do
          nil -> nil
          found -> walk_path(state, remaining, found.inode_number)
        end
    end
  end

  # ============================================================================
  # Directory Operations
  # ============================================================================

  @doc """
  ## mkdir() — Create a New Directory

  Creates a new directory at the given path with "." and ".." entries.

      mkdir("/home/alice")
      ════════════════════
        1. Resolve parent "/home" → inode 5
        2. Allocate new inode (type=:directory) → inode 12
        3. Allocate data block for new directory
        4. Write "." (→12) and ".." (→5) entries
        5. Add "alice" (→12) entry to parent
        6. Increment parent's link_count (for ".." reference)
  """
  def mkdir(state, path_str) do
    {parent_path, dir_name} = split_path(path_str)

    if dir_name == "" do
      {:error, state}
    else
      parent_inode_num = resolve_path(state, parent_path)

      if parent_inode_num == nil do
        {:error, state}
      else
        parent_inode = get_inode(state, parent_inode_num)

        if parent_inode == nil or parent_inode.file_type != :directory do
          {:error, state}
        else
          # Check name doesn't exist
          parent_entries = read_directory_entries(state, parent_inode)

          if Enum.any?(parent_entries, fn entry -> entry.name == dir_name end) do
            {:error, state}
          else
            do_mkdir(state, parent_inode, parent_inode_num, dir_name)
          end
        end
      end
    end
  end

  defp do_mkdir(state, parent_inode, parent_inode_num, dir_name) do
    # Allocate inode for new directory
    {new_inode, state} = allocate_inode(state, :directory)

    if new_inode == nil do
      {:error, state}
    else
      # Allocate data block for directory entries
      {block_idx, state} = allocate_block(state)

      if block_idx == nil do
        state = free_inode(state, new_inode.inode_number)
        {:error, state}
      else
        # Set up new directory inode
        new_inode = %{new_inode |
          link_count: 2,
          direct_blocks: List.replace_at(new_inode.direct_blocks, 0, block_idx)
        }

        # Write "." and ".." entries
        entries = [
          %DirectoryEntry{name: ".", inode_number: new_inode.inode_number},
          %DirectoryEntry{name: "..", inode_number: parent_inode_num}
        ]

        dir_data = serialize_entries(entries)
        new_inode = %{new_inode | size: byte_size(dir_data)}

        # Write data to block
        block_absolute = state.data_block_start + block_idx
        padded = dir_data <> :binary.copy(<<0>>, @block_size - byte_size(dir_data))
        blocks = List.replace_at(state.blocks, block_absolute, padded)
        state = %{state | blocks: blocks}

        # Update new inode in table
        state = update_inode(state, new_inode)

        # Add entry in parent directory
        state = add_directory_entry(state, parent_inode, dir_name, new_inode.inode_number)

        # Increment parent link count (for ".." reference)
        updated_parent = get_inode(state, parent_inode_num)
        updated_parent = %{updated_parent | link_count: updated_parent.link_count + 1}
        state = update_inode(state, updated_parent)

        # Update superblock
        sb = %{state.superblock |
          free_inodes: state.superblock.free_inodes - 1,
          free_blocks: block_free_count(state)
        }

        {:ok, %{state | superblock: sb}}
      end
    end
  end

  @doc """
  ## readdir() — List Directory Contents

  Returns a list of DirectoryEntry structs for the directory at the given path.
  """
  def readdir(state, path_str) do
    inode_num = resolve_path(state, path_str)

    if inode_num == nil do
      nil
    else
      inode = get_inode(state, inode_num)

      if inode == nil or inode.file_type != :directory do
        nil
      else
        read_directory_entries(state, inode)
      end
    end
  end

  # ============================================================================
  # File Operations: open, close, read, write, lseek
  # ============================================================================

  @doc """
  ## open() — Open a File for Reading/Writing

  Resolves the path, optionally creates the file, and returns a file
  descriptor.

      open("/data/log.txt", [:rdwr, :creat])
      ═══════════════════════════════════════
        1. Resolve path → inode 23 (or create if :creat)
        2. Create OpenFile entry
        3. Allocate fd in process's FD table
        4. Return {fd, new_state}
  """
  def open(state, path_str, flags, pid \\ 0) do
    inode_num = resolve_path(state, path_str)

    {inode_num, state} =
      if inode_num == nil and :creat in flags do
        create_file(state, path_str)
      else
        {inode_num, state}
      end

    if inode_num == nil do
      {nil, state}
    else
      inode = get_inode(state, inode_num)

      if inode == nil do
        {nil, state}
      else
        # Handle O_TRUNC
        state =
          if :trunc in flags do
            truncate_file(state, inode)
          else
            state
          end

        # Determine access mode
        access_mode =
          cond do
            :rdwr in flags -> :rdwr
            :wronly in flags -> :wronly
            true -> :rdonly
          end

        # Create open file entry
        global_id = state.next_open_file_id

        initial_offset =
          if :append in flags do
            inode = get_inode(state, inode_num)
            inode.size
          else
            0
          end

        open_file = %OpenFile{
          inode_number: inode_num,
          offset: initial_offset,
          flags: access_mode,
          ref_count: 1
        }

        open_files = Map.put(state.open_files, global_id, open_file)

        # Allocate fd in process's FD table
        fd_table = Map.get(state.fd_tables, pid, %{})
        local_fd = next_available_fd(fd_table)
        fd_table = Map.put(fd_table, local_fd, global_id)
        fd_tables = Map.put(state.fd_tables, pid, fd_table)

        state = %{state |
          open_files: open_files,
          fd_tables: fd_tables,
          next_open_file_id: global_id + 1
        }

        {local_fd, state}
      end
    end
  end

  defp next_available_fd(fd_table) do
    # Find lowest available fd starting from 3
    used_fds = Map.keys(fd_table) |> MapSet.new()
    Enum.find(3..255, fn fd -> not MapSet.member?(used_fds, fd) end)
  end

  @doc """
  ## close() — Close a File Descriptor

  Frees the fd and decrements ref_count on the OpenFile entry.
  """
  def close(state, fd, pid \\ 0) do
    fd_table = Map.get(state.fd_tables, pid, %{})
    global_id = Map.get(fd_table, fd)

    if global_id == nil do
      {:error, state}
    else
      # Free the fd
      fd_table = Map.delete(fd_table, fd)
      fd_tables = Map.put(state.fd_tables, pid, fd_table)

      # Decrement ref count
      open_file = Map.get(state.open_files, global_id)

      open_files =
        if open_file != nil do
          new_ref = open_file.ref_count - 1

          if new_ref <= 0 do
            Map.delete(state.open_files, global_id)
          else
            Map.put(state.open_files, global_id, %{open_file | ref_count: new_ref})
          end
        else
          state.open_files
        end

      {:ok, %{state | fd_tables: fd_tables, open_files: open_files}}
    end
  end

  @doc """
  ## read() — Read Data from an Open File

  Reads up to `count` bytes starting at the current offset.

      Reading Algorithm
      ═════════════════
        1. Look up fd → OpenFile → inode
        2. Calculate which block holds current offset
        3. Read bytes from block, advance offset
        4. Repeat until count bytes read or end-of-file
  """
  def read(state, fd, count, pid \\ 0) do
    with {:ok, open_file, global_id} <- lookup_fd(state, fd, pid),
         false <- open_file.flags == :wronly,
         inode when inode != nil <- get_inode(state, open_file.inode_number) do
      bytes_available = inode.size - open_file.offset

      if bytes_available <= 0 do
        {<<>>, state}
      else
        bytes_to_read = min(count, bytes_available)
        {data, _bytes_read} = read_bytes(state, inode, open_file.offset, bytes_to_read)

        actual_read = byte_size(data)
        new_offset = open_file.offset + actual_read
        updated = %{open_file | offset: new_offset}
        open_files = Map.put(state.open_files, global_id, updated)

        # Update accessed_at
        now = System.system_time(:millisecond)
        updated_inode = %{inode | accessed_at: now}
        state = update_inode(%{state | open_files: open_files}, updated_inode)

        {data, state}
      end
    else
      _ -> {nil, state}
    end
  end

  @doc """
  ## write() — Write Data to an Open File

  Writes data at the current offset, allocating new blocks as needed.

      Writing Algorithm
      ═════════════════
        1. Look up fd → OpenFile → inode
        2. For each chunk:
           a. Calculate block_index from offset
           b. Allocate block if needed
           c. Write data to block
           d. Advance offset
        3. Update inode size if wrote past end
  """
  def write(state, fd, data, pid \\ 0) when is_binary(data) do
    with {:ok, open_file, global_id} <- lookup_fd(state, fd, pid),
         false <- open_file.flags == :rdonly,
         inode when inode != nil <- get_inode(state, open_file.inode_number) do
      {bytes_written, state, inode} =
        write_bytes(state, inode, open_file.offset, data)

      new_offset = open_file.offset + bytes_written

      # Update size if needed
      new_size = max(inode.size, new_offset)
      now = System.system_time(:millisecond)
      updated_inode = %{inode | size: new_size, modified_at: now}
      state = update_inode(state, updated_inode)

      # Update open file offset
      updated_of = %{open_file | offset: new_offset}
      open_files = Map.put(state.open_files, global_id, updated_of)

      # Update superblock free_blocks
      sb = %{state.superblock | free_blocks: block_free_count(state)}

      {bytes_written, %{state | open_files: open_files, superblock: sb}}
    else
      _ -> {nil, state}
    end
  end

  @doc """
  ## lseek() — Reposition the File Offset

      :set     — new_offset = offset (absolute)
      :cur     — new_offset = current + offset (relative)
      :seek_end — new_offset = file_size + offset (from end)

  Note: We use :seek_end instead of :end because `end` is a reserved word
  in Elixir.
  """
  def lseek(state, fd, offset, whence, pid \\ 0) do
    with {:ok, open_file, global_id} <- lookup_fd(state, fd, pid),
         inode when inode != nil <- get_inode(state, open_file.inode_number) do
      new_offset =
        case whence do
          :set -> offset
          :cur -> open_file.offset + offset
          :seek_end -> inode.size + offset
          _ -> nil
        end

      if new_offset == nil or new_offset < 0 do
        {nil, state}
      else
        updated = %{open_file | offset: new_offset}
        open_files = Map.put(state.open_files, global_id, updated)
        {new_offset, %{state | open_files: open_files}}
      end
    else
      _ -> {nil, state}
    end
  end

  # ============================================================================
  # stat and unlink
  # ============================================================================

  @doc """
  ## stat() — Get File Metadata

  Returns a map of inode metadata for the file at the given path.
  """
  def stat(state, path_str) do
    inode_num = resolve_path(state, path_str)

    if inode_num == nil do
      nil
    else
      inode = get_inode(state, inode_num)

      if inode == nil do
        nil
      else
        %{
          inode_number: inode.inode_number,
          file_type: inode.file_type,
          size: inode.size,
          permissions: inode.permissions,
          link_count: inode.link_count,
          created_at: inode.created_at,
          modified_at: inode.modified_at,
          accessed_at: inode.accessed_at
        }
      end
    end
  end

  @doc """
  ## unlink() — Remove a File

  Removes a directory entry and decrements link_count. When link_count
  reaches 0, frees the inode and all its data blocks.

      unlink("/data/log.txt")
      ═══════════════════════
        1. Resolve parent "/data" → inode 5
        2. Find "log.txt" in parent's entries → inode 23
        3. Remove "log.txt" entry from parent
        4. Decrement inode 23's link_count
        5. If link_count == 0: free blocks and inode
  """
  def unlink(state, path_str) do
    {parent_path, file_name} = split_path(path_str)

    if file_name == "" do
      {:error, state}
    else
      parent_inode_num = resolve_path(state, parent_path)

      if parent_inode_num == nil do
        {:error, state}
      else
        parent_inode = get_inode(state, parent_inode_num)

        if parent_inode == nil or parent_inode.file_type != :directory do
          {:error, state}
        else
          entries = read_directory_entries(state, parent_inode)
          target_entry = Enum.find(entries, fn entry -> entry.name == file_name end)

          if target_entry == nil do
            {:error, state}
          else
            target_inode = get_inode(state, target_entry.inode_number)

            if target_inode == nil or target_inode.file_type == :directory do
              {:error, state}
            else
              do_unlink(state, parent_inode, entries, file_name, target_inode)
            end
          end
        end
      end
    end
  end

  defp do_unlink(state, parent_inode, entries, file_name, target_inode) do
    # Remove entry from parent
    new_entries = Enum.reject(entries, fn entry -> entry.name == file_name end)
    state = write_directory_entries(state, parent_inode, new_entries)

    # Decrement link count
    new_link_count = target_inode.link_count - 1

    state =
      if new_link_count <= 0 do
        # Free blocks and inode
        state = free_inode_blocks(state, target_inode)
        state = free_inode(state, target_inode.inode_number)

        sb = %{state.superblock |
          free_inodes: state.superblock.free_inodes + 1,
          free_blocks: block_free_count(state)
        }

        %{state | superblock: sb}
      else
        updated = %{target_inode | link_count: new_link_count}
        update_inode(state, updated)
      end

    {:ok, state}
  end

  # ============================================================================
  # dup / dup2
  # ============================================================================

  @doc """
  ## dup() — Duplicate a File Descriptor

  Creates a new fd pointing to the same OpenFile entry. Both fds share
  the same offset.
  """
  def dup(state, fd, pid \\ 0) do
    with {:ok, open_file, global_id} <- lookup_fd(state, fd, pid) do
      # Increment ref count
      updated = %{open_file | ref_count: open_file.ref_count + 1}
      open_files = Map.put(state.open_files, global_id, updated)

      # Allocate new fd
      fd_table = Map.get(state.fd_tables, pid, %{})
      new_fd = next_available_fd(fd_table)
      fd_table = Map.put(fd_table, new_fd, global_id)
      fd_tables = Map.put(state.fd_tables, pid, fd_table)

      {new_fd, %{state | open_files: open_files, fd_tables: fd_tables}}
    else
      _ -> {nil, state}
    end
  end

  @doc """
  ## dup2() — Duplicate a File Descriptor to a Specific Number

  Like dup(), but the caller chooses the new fd number. If the target fd
  is already open, it is closed first.
  """
  def dup2(state, old_fd, new_fd, pid \\ 0) do
    with {:ok, open_file, global_id} <- lookup_fd(state, old_fd, pid) do
      fd_table = Map.get(state.fd_tables, pid, %{})

      # Close new_fd if it's already open
      state =
        if Map.has_key?(fd_table, new_fd) do
          {:ok, closed_state} = close(state, new_fd, pid)
          closed_state
        else
          state
        end

      # Increment ref count
      # Re-fetch open_file since state may have changed
      current_of = Map.get(state.open_files, global_id, open_file)
      updated = %{current_of | ref_count: current_of.ref_count + 1}
      open_files = Map.put(state.open_files, global_id, updated)

      # Map new_fd to the same global entry
      fd_table = Map.get(state.fd_tables, pid, %{})
      fd_table = Map.put(fd_table, new_fd, global_id)
      fd_tables = Map.put(state.fd_tables, pid, fd_table)

      {new_fd, %{state | open_files: open_files, fd_tables: fd_tables}}
    else
      _ -> {nil, state}
    end
  end

  # ============================================================================
  # Private Helpers
  # ============================================================================

  defp lookup_fd(state, fd, pid) do
    fd_table = Map.get(state.fd_tables, pid, %{})
    global_id = Map.get(fd_table, fd)

    if global_id == nil do
      :error
    else
      open_file = Map.get(state.open_files, global_id)

      if open_file == nil do
        :error
      else
        {:ok, open_file, global_id}
      end
    end
  end

  defp split_path(path_str) do
    parts =
      path_str
      |> String.split("/")
      |> Enum.reject(&(&1 == ""))

    case parts do
      [] -> {"/", ""}
      _ ->
        dir_name = List.last(parts)
        parent_parts = Enum.drop(parts, -1)
        parent_path = "/" <> Enum.join(parent_parts, "/")
        {parent_path, dir_name}
    end
  end

  defp read_directory_entries(state, inode) do
    data = read_all_blocks(state, inode)
    # Take only up to inode.size bytes
    relevant = binary_part(data, 0, min(byte_size(data), inode.size))
    deserialize_entries(relevant)
  end

  defp write_directory_entries(state, inode, entries) do
    data = serialize_entries(entries)
    {_, state, updated_inode} = write_raw_blocks(state, inode, 0, data)
    updated_inode = %{updated_inode | size: byte_size(data)}
    update_inode(state, updated_inode)
  end

  defp add_directory_entry(state, dir_inode, entry_name, inode_number) do
    entries = read_directory_entries(state, dir_inode)
    new_entries = entries ++ [%DirectoryEntry{name: entry_name, inode_number: inode_number}]
    write_directory_entries(state, dir_inode, new_entries)
  end

  defp read_all_blocks(state, inode) do
    total_blocks_needed = max(div(inode.size + @block_size - 1, @block_size), 0)

    0..(max(total_blocks_needed - 1, 0))
    |> Enum.reduce(<<>>, fn block_index, acc ->
      case resolve_block_number(state, inode, block_index) do
        nil -> acc
        block_num ->
          block_absolute = state.data_block_start + block_num
          block_data = Enum.at(state.blocks, block_absolute, :binary.copy(<<0>>, @block_size))
          acc <> block_data
      end
    end)
  end

  defp read_bytes(state, inode, start_offset, count) do
    # Read block by block
    {data, bytes_read} =
      read_bytes_loop(state, inode, start_offset, count, <<>>, 0)

    {data, bytes_read}
  end

  defp read_bytes_loop(_state, _inode, _offset, remaining, acc, bytes_read) when remaining <= 0 do
    {acc, bytes_read}
  end

  defp read_bytes_loop(state, inode, offset, remaining, acc, bytes_read) do
    block_index = div(offset, @block_size)
    byte_in_block = rem(offset, @block_size)

    case resolve_block_number(state, inode, block_index) do
      nil ->
        {acc, bytes_read}

      block_num ->
        block_absolute = state.data_block_start + block_num
        block_data = Enum.at(state.blocks, block_absolute, :binary.copy(<<0>>, @block_size))
        chunk_size = min(@block_size - byte_in_block, remaining)
        chunk = binary_part(block_data, byte_in_block, chunk_size)

        read_bytes_loop(
          state,
          inode,
          offset + chunk_size,
          remaining - chunk_size,
          acc <> chunk,
          bytes_read + chunk_size
        )
    end
  end

  defp write_bytes(state, inode, start_offset, data) do
    data_size = byte_size(data)
    write_bytes_loop(state, inode, start_offset, data, 0, data_size)
  end

  defp write_bytes_loop(state, inode, _offset, _data, bytes_written, total)
       when bytes_written >= total do
    {bytes_written, state, inode}
  end

  defp write_bytes_loop(state, inode, offset, data, bytes_written, total) do
    block_index = div(offset, @block_size)
    byte_in_block = rem(offset, @block_size)

    # Ensure block is allocated
    {block_num, state, inode} =
      case resolve_block_number(state, inode, block_index) do
        nil ->
          allocate_block_for_inode(state, inode, block_index)

        existing ->
          {existing, state, inode}
      end

    if block_num == nil do
      # Disk full
      {bytes_written, state, inode}
    else
      block_absolute = state.data_block_start + block_num
      block_data = Enum.at(state.blocks, block_absolute, :binary.copy(<<0>>, @block_size))

      chunk_size = min(@block_size - byte_in_block, total - bytes_written)
      chunk = binary_part(data, bytes_written, chunk_size)

      # Replace bytes in block
      prefix = binary_part(block_data, 0, byte_in_block)
      suffix_start = byte_in_block + chunk_size
      suffix_len = @block_size - suffix_start
      suffix = if suffix_len > 0, do: binary_part(block_data, suffix_start, suffix_len), else: <<>>

      new_block = prefix <> chunk <> suffix
      blocks = List.replace_at(state.blocks, block_absolute, new_block)
      state = %{state | blocks: blocks}

      write_bytes_loop(
        state,
        inode,
        offset + chunk_size,
        data,
        bytes_written + chunk_size,
        total
      )
    end
  end

  # Same as write_bytes but used for directory entry writing (starts from offset 0)
  defp write_raw_blocks(state, inode, start_offset, data) do
    write_bytes(state, inode, start_offset, data)
  end

  @doc """
  ## resolve_block_number() — Direct vs. Indirect Block Lookup

  Given a logical block index, returns the physical block number.

      Block Index Resolution
      ══════════════════════
        index < 12  → inode.direct_blocks[index]
        index >= 12 → read indirect block → pointers[index - 12]
  """
  def resolve_block_number(_state, inode, block_index) when block_index < @direct_blocks do
    block_num = Enum.at(inode.direct_blocks, block_index, @unallocated)
    if block_num == @unallocated, do: nil, else: block_num
  end

  def resolve_block_number(state, inode, block_index) do
    if inode.indirect_block == @unallocated do
      nil
    else
      indirect_absolute = state.data_block_start + inode.indirect_block
      indirect_data = Enum.at(state.blocks, indirect_absolute)
      pointer_index = block_index - @direct_blocks
      byte_offset = pointer_index * 4

      if byte_offset + 4 > @block_size do
        nil
      else
        <<_::binary-size(byte_offset), block_num::little-unsigned-32, _::binary>> = indirect_data
        if block_num == 0, do: nil, else: block_num
      end
    end
  end

  defp allocate_block_for_inode(state, inode, block_index) when block_index < @direct_blocks do
    {new_block, state} = allocate_block(state)

    if new_block == nil do
      {nil, state, inode}
    else
      new_directs = List.replace_at(inode.direct_blocks, block_index, new_block)
      inode = %{inode | direct_blocks: new_directs}
      state = update_inode(state, inode)
      {new_block, state, inode}
    end
  end

  defp allocate_block_for_inode(state, inode, block_index) do
    # Need indirect block
    {state, inode} =
      if inode.indirect_block == @unallocated do
        {indirect_block, state} = allocate_block(state)

        if indirect_block == nil do
          {state, inode}
        else
          # Clear the indirect block
          block_absolute = state.data_block_start + indirect_block
          blocks = List.replace_at(state.blocks, block_absolute, :binary.copy(<<0>>, @block_size))
          inode = %{inode | indirect_block: indirect_block}
          state = update_inode(%{state | blocks: blocks}, inode)
          {state, inode}
        end
      else
        {state, inode}
      end

    if inode.indirect_block == @unallocated do
      {nil, state, inode}
    else
      {new_block, state} = allocate_block(state)

      if new_block == nil do
        {nil, state, inode}
      else
        # Write pointer into indirect block
        indirect_absolute = state.data_block_start + inode.indirect_block
        indirect_data = Enum.at(state.blocks, indirect_absolute)
        pointer_index = block_index - @direct_blocks
        byte_offset = pointer_index * 4

        # Build new indirect block with the pointer written
        prefix = binary_part(indirect_data, 0, byte_offset)
        suffix_start = byte_offset + 4
        suffix_len = @block_size - suffix_start
        suffix = binary_part(indirect_data, suffix_start, suffix_len)
        new_indirect = prefix <> <<new_block::little-unsigned-32>> <> suffix

        blocks = List.replace_at(state.blocks, indirect_absolute, new_indirect)
        state = %{state | blocks: blocks}

        {new_block, state, inode}
      end
    end
  end

  defp create_file(state, path_str) do
    {parent_path, file_name} = split_path(path_str)

    if file_name == "" do
      {nil, state}
    else
      parent_inode_num = resolve_path(state, parent_path)

      if parent_inode_num == nil do
        {nil, state}
      else
        parent_inode = get_inode(state, parent_inode_num)

        if parent_inode == nil or parent_inode.file_type != :directory do
          {nil, state}
        else
          {new_inode, state} = allocate_inode(state, :regular)

          if new_inode == nil do
            {nil, state}
          else
            state = add_directory_entry(state, parent_inode, file_name, new_inode.inode_number)

            sb = %{state.superblock | free_inodes: state.superblock.free_inodes - 1}
            {new_inode.inode_number, %{state | superblock: sb}}
          end
        end
      end
    end
  end

  defp truncate_file(state, inode) do
    state = free_inode_blocks(state, inode)
    updated = %{inode | size: 0}
    state = update_inode(state, updated)
    sb = %{state.superblock | free_blocks: block_free_count(state)}
    %{state | superblock: sb}
  end

  defp free_inode_blocks(state, inode) do
    # Free direct blocks
    state =
      Enum.reduce(0..(@direct_blocks - 1), state, fn i, acc_state ->
        block_num = Enum.at(inode.direct_blocks, i, @unallocated)

        if block_num != @unallocated do
          free_block(acc_state, block_num)
        else
          acc_state
        end
      end)

    # Free indirect block and its pointers
    state =
      if inode.indirect_block != @unallocated do
        indirect_absolute = state.data_block_start + inode.indirect_block
        indirect_data = Enum.at(state.blocks, indirect_absolute)

        # Read all pointers (4 bytes each)
        state =
          Enum.reduce(0..(div(@block_size, 4) - 1), state, fn i, acc_state ->
            byte_offset = i * 4
            <<_::binary-size(byte_offset), block_num::little-unsigned-32, _::binary>> = indirect_data

            if block_num != 0 do
              free_block(acc_state, block_num)
            else
              acc_state
            end
          end)

        free_block(state, inode.indirect_block)
      else
        state
      end

    # Reset inode block pointers
    updated = %{inode |
      direct_blocks: List.duplicate(@unallocated, @direct_blocks),
      indirect_block: @unallocated
    }

    update_inode(state, updated)
  end
end
