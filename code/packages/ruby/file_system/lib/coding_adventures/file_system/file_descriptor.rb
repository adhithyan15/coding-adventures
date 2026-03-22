# frozen_string_literal: true

# = File Descriptors: OpenFile, OpenFileTable, FileDescriptorTable
#
# When a process calls open(), it doesn't get back an inode or a block
# number. It gets a small integer — a file descriptor (fd). File descriptors
# are the process's handle to an open file.
#
# There are TWO levels of indirection, and understanding why both exist is
# crucial to understanding Unix I/O:
#
# == Level 1: OpenFileTable (system-wide)
#
# The system-wide open file table holds one entry per *opening* of a file.
# Each entry tracks:
#   - Which inode (file) is open
#   - The current read/write offset
#   - The access flags (read-only, write-only, read-write)
#   - A reference count (how many fd's point here)
#
# Why system-wide? Because when a process forks, the child inherits the
# parent's file descriptors, and both parent and child share the same
# offset. If the parent reads 10 bytes, the child's next read starts at
# byte 10. This sharing happens through the OpenFileTable.
#
# == Level 2: FileDescriptorTable (per-process)
#
# Each process has its own small table mapping integers (0, 1, 2, 3, ...)
# to entries in the system-wide OpenFileTable. This is why two processes
# can both have fd 3, but they refer to completely different files.
#
#   Process A's fd table        System-wide OpenFileTable
#   ┌───┬────────────┐         ┌───┬─────────────────────┐
#   │ 0 │ → entry 0  │────────>│ 0 │ inode=7, offset=0   │  (stdin)
#   │ 1 │ → entry 1  │────────>│ 1 │ inode=8, offset=0   │  (stdout)
#   │ 2 │ → entry 2  │────────>│ 2 │ inode=9, offset=0   │  (stderr)
#   │ 3 │ → entry 5  │────────>│ 5 │ inode=23, offset=42 │
#   └───┴────────────┘         └───┴─────────────────────┘

module CodingAdventures
  module FileSystem
    # An OpenFile represents one *opening* of a file. Multiple file
    # descriptors (across different processes) can point to the same
    # OpenFile, sharing the offset.
    class OpenFile
      # Which inode this open file refers to.
      attr_accessor :inode_number

      # Current byte offset within the file. Each read/write advances
      # this position.
      attr_accessor :offset

      # How the file was opened: O_RDONLY, O_WRONLY, or O_RDWR.
      # Determines which operations are permitted.
      attr_accessor :flags

      # Number of file descriptors pointing to this entry. When it drops
      # to 0, the entry can be freed.
      attr_accessor :ref_count

      def initialize(inode_number, flags)
        @inode_number = inode_number
        @offset = 0
        @flags = flags
        @ref_count = 1
      end

      # Can this open file be read from?
      #
      # Truth table:
      #   flags & 0x3  | readable?
      #   -------------|----------
      #   0 (O_RDONLY) |   true
      #   1 (O_WRONLY) |   false
      #   2 (O_RDWR)   |   true
      def readable?
        (flags & 0x3) != O_WRONLY
      end

      # Can this open file be written to?
      #
      # Truth table:
      #   flags & 0x3  | writable?
      #   -------------|----------
      #   0 (O_RDONLY) |   false
      #   1 (O_WRONLY) |   true
      #   2 (O_RDWR)   |   true
      def writable?
        (flags & 0x3) != O_RDONLY
      end
    end

    # The system-wide table of all open files. Shared across all processes.
    class OpenFileTable
      def initialize
        # Sparse array: index is the "global fd", value is an OpenFile.
        # nil entries are free slots.
        @entries = []
      end

      # Opens a file by creating a new entry in the table.
      #
      # @param inode_number [Integer] The inode of the file to open
      # @param flags [Integer] Access flags (O_RDONLY, O_WRONLY, O_RDWR)
      # @return [Integer] The index (global fd) of the new entry
      def open(inode_number, flags)
        entry = OpenFile.new(inode_number, flags)

        # Find the first free slot, or append
        @entries.each_with_index do |existing, index|
          if existing.nil?
            @entries[index] = entry
            return index
          end
        end

        @entries << entry
        @entries.length - 1
      end

      # Closes an open file entry by decrementing its reference count.
      # If the reference count reaches 0, the slot is freed.
      #
      # @param index [Integer] The global fd to close
      # @return [Boolean] true if the entry was fully closed (ref_count hit 0)
      def close(index)
        return false if index < 0 || index >= @entries.length
        entry = @entries[index]
        return false if entry.nil?

        entry.ref_count -= 1
        if entry.ref_count <= 0
          @entries[index] = nil
          true
        else
          false
        end
      end

      # Returns the OpenFile at the given index.
      #
      # @param index [Integer] The global fd
      # @return [OpenFile, nil] The entry, or nil if the slot is empty
      def get(index)
        return nil if index < 0 || index >= @entries.length

        @entries[index]
      end

      # Duplicates an open file entry by incrementing its reference count.
      # This is used by dup() — the new fd points to the same OpenFile.
      #
      # @param index [Integer] The global fd to duplicate
      # @return [Integer, nil] The same index (for convenience), or nil
      def dup(index)
        entry = get(index)
        return nil if entry.nil?

        entry.ref_count += 1
        index
      end
    end

    # Per-process file descriptor table. Maps local fd numbers (0, 1, 2, ...)
    # to indices in the system-wide OpenFileTable.
    #
    # Each process gets its own FileDescriptorTable. When a process forks,
    # the child gets a clone of the parent's table (but shares the same
    # OpenFileTable entries).
    class FileDescriptorTable
      def initialize
        # Hash mapping local_fd (Integer) → global_fd (Integer)
        @mapping = {}
        @next_fd = 0
      end

      # Allocates the lowest available local fd and maps it to a global fd.
      #
      # @param global_fd [Integer] Index in the OpenFileTable
      # @return [Integer] The local fd number assigned
      def allocate(global_fd)
        fd = lowest_free_fd
        @mapping[fd] = global_fd
        @next_fd = fd + 1 if fd >= @next_fd
        fd
      end

      # Removes a local fd mapping.
      #
      # @param local_fd [Integer] The local fd to close
      # @return [Integer, nil] The global fd it was mapped to, or nil
      def close(local_fd)
        @mapping.delete(local_fd)
      end

      # Looks up which global fd a local fd maps to.
      #
      # @param local_fd [Integer] The local fd to look up
      # @return [Integer, nil] The global fd, or nil if not mapped
      def get(local_fd)
        @mapping[local_fd]
      end

      # Duplicates a file descriptor. Finds the lowest free local fd and
      # maps it to the same global fd as the original.
      #
      # @param old_fd [Integer] The local fd to duplicate
      # @return [Integer, nil] The new local fd, or nil if old_fd invalid
      def dup_fd(old_fd)
        global = @mapping[old_fd]
        return nil if global.nil?

        new_fd = lowest_free_fd
        @mapping[new_fd] = global
        new_fd
      end

      # Duplicates a file descriptor to a specific number. If new_fd is
      # already open, it is closed first.
      #
      # @param old_fd [Integer] Source fd
      # @param new_fd [Integer] Destination fd number
      # @return [Integer, nil] new_fd on success, nil if old_fd invalid
      def dup2(old_fd, new_fd)
        global = @mapping[old_fd]
        return nil if global.nil?

        # If new_fd is already open, close it first
        @mapping.delete(new_fd) if @mapping.key?(new_fd)
        @mapping[new_fd] = global
        new_fd
      end

      # Creates a clone of this fd table (used by fork).
      #
      # @return [FileDescriptorTable] A new table with the same mappings
      def clone_table
        new_table = FileDescriptorTable.new
        @mapping.each { |local, global| new_table.set(local, global) }
        new_table
      end

      # Returns all active (local_fd, global_fd) pairs.
      #
      # @return [Hash{Integer => Integer}] Mapping of local to global fds
      def entries
        @mapping.dup
      end

      # Directly sets a mapping (used by clone_table).
      # @api private
      def set(local_fd, global_fd)
        @mapping[local_fd] = global_fd
      end

      private

      # Finds the lowest integer >= 0 not currently in use as a local fd.
      #
      # @return [Integer] The lowest free fd number
      def lowest_free_fd
        fd = 0
        fd += 1 while @mapping.key?(fd)
        fd
      end
    end
  end
end
