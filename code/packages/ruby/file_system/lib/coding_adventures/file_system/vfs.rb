# frozen_string_literal: true

# = VFS (Virtual File System)
#
# The VFS is the grand unifier — the layer that ties together inodes,
# directories, block bitmaps, file descriptors, and raw block storage into
# the familiar file system API that every program uses: open, read, write,
# close, mkdir, etc.
#
# == Analogy
#
# If the file system were a restaurant:
#   - The *disk* (block storage) is the kitchen pantry
#   - The *block bitmap* is the inventory checklist
#   - The *inode table* is the recipe index
#   - The *directories* are the menu sections (appetizers, mains, desserts)
#   - The *file descriptors* are order tickets
#   - The *VFS* is the head chef who coordinates everything
#
# The head chef (VFS) receives order tickets (open/read/write), looks up
# recipes (inodes), checks inventory (bitmap), and manages the pantry
# (block storage). No one else interacts with the pantry directly.
#
# == Disk Layout
#
# After formatting, the disk is organized as:
#
#   Block 0:           Superblock (magic number, counts, sizes)
#   Blocks 1..N:       Inode table (holds MAX_INODES inodes)
#   Block N+1:         Block bitmap (1 bit per data block)
#   Blocks N+2..511:   Data blocks (file contents, directory entries)
#
# We calculate N from the inode size and block size. In our in-memory
# implementation, we don't actually serialize inodes to disk blocks — we
# keep them as Ruby objects. The block storage is used only for data blocks.

module CodingAdventures
  module FileSystem
    class VFS
      # The superblock describing this file system's geometry.
      attr_reader :superblock

      # The inode table holding all file/directory metadata.
      attr_reader :inode_table

      # The block bitmap tracking free/used data blocks.
      attr_reader :block_bitmap

      # The system-wide open file table.
      attr_reader :open_file_table

      # The per-process file descriptor table. In a real OS, each process
      # would have its own. We use one for simplicity.
      attr_reader :fd_table

      # Creates a new VFS instance. Call #format to initialize the file system.
      def initialize
        @superblock = nil
        @inode_table = nil
        @block_bitmap = nil
        @open_file_table = OpenFileTable.new
        @fd_table = FileDescriptorTable.new

        # In-memory block storage. Each element is a binary string of
        # BLOCK_SIZE bytes. This simulates the raw disk.
        @blocks = []
      end

      # ======================================================================
      # format — Initialize a Blank Disk
      # ======================================================================
      #
      # Formatting creates the empty file system structure:
      #   1. Write the superblock with file system metadata
      #   2. Initialize all inodes as free
      #   3. Initialize the block bitmap with all blocks free
      #   4. Create the root directory (inode 0) with "." and ".." entries
      #
      # After formatting, the disk is ready for use. The root directory
      # exists at "/" and you can start creating files and subdirectories.
      #
      # @param total_blocks [Integer] Total blocks on disk (default: 512)
      # @param total_inodes [Integer] Max inodes (default: 128)
      def format(total_blocks: MAX_BLOCKS, total_inodes: MAX_INODES)
        # Calculate how many blocks are reserved for metadata.
        # We reserve: 1 superblock + inode_table_blocks + 1 bitmap block.
        # The remaining blocks are available for data.
        inode_table_blocks = (total_inodes + (BLOCK_SIZE / 64) - 1) / (BLOCK_SIZE / 64)
        metadata_blocks = 1 + inode_table_blocks + 1
        data_block_count = total_blocks - metadata_blocks

        # Initialize the superblock
        @superblock = Superblock.new(total_blocks: total_blocks, total_inodes: total_inodes)

        # Initialize the inode table (all inodes start as free)
        @inode_table = InodeTable.new(total_inodes)

        # Initialize the block bitmap with only data blocks tracked
        @block_bitmap = BlockBitmap.new(data_block_count)

        # Initialize in-memory block storage (all zeros)
        @blocks = Array.new(data_block_count) { "\x00".b * BLOCK_SIZE }

        # Create the root directory at inode 0
        root_inode = @inode_table.allocate(FILE_TYPE_DIRECTORY)
        root_inode.link_count = 2  # "." and ".." both point to root

        # Allocate one data block for the root directory's entries
        root_block = @block_bitmap.allocate
        root_inode.direct_blocks[0] = root_block

        # Create the "." and ".." entries (both point to inode 0 for root)
        dot = DirectoryEntry.new(".", ROOT_INODE)
        dotdot = DirectoryEntry.new("..", ROOT_INODE)
        dir_data = DirectoryEntry.serialize_all([dot, dotdot])
        root_inode.size = dir_data.length

        # Write the directory entries to the data block
        write_block(root_block, dir_data)

        # Update superblock free counts
        @superblock.free_blocks = @block_bitmap.free_count
        @superblock.free_inodes = @inode_table.free_count
      end

      # ======================================================================
      # open — Open a File
      # ======================================================================
      #
      # Opening a file involves:
      #   1. Resolve the path to find the inode
      #   2. If O_CREAT is set and the file doesn't exist, create it
      #   3. If O_TRUNC is set, truncate the file to zero length
      #   4. Create an entry in the OpenFileTable
      #   5. Allocate a local fd in the FileDescriptorTable
      #
      # @param path [String] Absolute path to the file (e.g., "/data/log.txt")
      # @param flags [Integer] Open flags (O_RDONLY, O_WRONLY | O_CREAT, etc.)
      # @return [Integer] The file descriptor, or -1 on error
      def open(path, flags = O_RDONLY)
        inode_number = resolve_path(path)

        if inode_number.nil?
          # File does not exist. Create it if O_CREAT is set.
          if (flags & O_CREAT) != 0
            inode_number = create_file(path, FILE_TYPE_REGULAR)
            return -1 if inode_number.nil?
          else
            return -1  # File not found and O_CREAT not set
          end
        end

        # If O_TRUNC is set, truncate the file to zero length
        if (flags & O_TRUNC) != 0
          truncate_inode(inode_number)
        end

        # Create an entry in the system-wide open file table
        global_fd = @open_file_table.open(inode_number, flags)

        # If O_APPEND is set, seek to end
        if (flags & O_APPEND) != 0
          entry = @open_file_table.get(global_fd)
          inode = @inode_table.get(inode_number)
          entry.offset = inode.size
        end

        # Allocate a local fd for this process
        @fd_table.allocate(global_fd)
      end

      # ======================================================================
      # close — Close a File Descriptor
      # ======================================================================
      #
      # Closing involves:
      #   1. Look up the global fd from the local fd
      #   2. Remove the local fd mapping
      #   3. Decrement ref_count on the OpenFile entry
      #   4. If ref_count hits 0, free the OpenFile slot
      #
      # @param fd [Integer] The local file descriptor to close
      # @return [Integer] 0 on success, -1 on error
      def close(fd)
        global_fd = @fd_table.get(fd)
        return -1 if global_fd.nil?

        @fd_table.close(fd)
        @open_file_table.close(global_fd)
        0
      end

      # ======================================================================
      # read — Read Bytes from an Open File
      # ======================================================================
      #
      # Reading a file involves:
      #   1. Look up the fd → OpenFile → inode
      #   2. Calculate which block(s) contain the requested bytes
      #   3. Read from direct or indirect blocks as needed
      #   4. Advance the offset
      #   5. Return the bytes read
      #
      # @param fd [Integer] The file descriptor to read from
      # @param count [Integer] Maximum number of bytes to read
      # @return [String, nil] The bytes read (binary string), or nil on error
      def read(fd, count)
        global_fd = @fd_table.get(fd)
        return nil if global_fd.nil?

        entry = @open_file_table.get(global_fd)
        return nil if entry.nil?
        return nil unless entry.readable?

        inode = @inode_table.get(entry.inode_number)
        return nil if inode.nil?

        # Don't read past the end of the file
        available = inode.size - entry.offset
        return "".b if available <= 0

        bytes_to_read = [count, available].min
        result = "".b
        remaining = bytes_to_read

        while remaining > 0
          # Calculate which block contains the current offset
          block_index = entry.offset / BLOCK_SIZE
          offset_in_block = entry.offset % BLOCK_SIZE

          # Get the actual block number from the inode
          block_number = get_block_number(inode, block_index)
          break if block_number.nil?

          # Read from this block
          block_data = read_block(block_number)
          bytes_from_block = [remaining, BLOCK_SIZE - offset_in_block].min
          result << block_data[offset_in_block, bytes_from_block]

          entry.offset += bytes_from_block
          remaining -= bytes_from_block
        end

        inode.accessed_at = Time.now
        result
      end

      # ======================================================================
      # write — Write Bytes to an Open File
      # ======================================================================
      #
      # Writing involves:
      #   1. Look up the fd → OpenFile → inode
      #   2. For each chunk of data:
      #      a. Calculate which block to write to
      #      b. Allocate a new block if needed
      #      c. Read the existing block (for partial writes)
      #      d. Overwrite the relevant bytes
      #      e. Write the block back
      #   3. Update the file size if we wrote past the old end
      #   4. Advance the offset
      #
      # @param fd [Integer] The file descriptor to write to
      # @param data [String] The bytes to write
      # @return [Integer] Number of bytes written, or -1 on error
      def write(fd, data)
        global_fd = @fd_table.get(fd)
        return -1 if global_fd.nil?

        entry = @open_file_table.get(global_fd)
        return -1 if entry.nil?
        return -1 unless entry.writable?

        # If O_APPEND, seek to end before writing
        inode = @inode_table.get(entry.inode_number)
        return -1 if inode.nil?

        if (entry.flags & O_APPEND) != 0
          entry.offset = inode.size
        end

        data = data.b  # Ensure binary encoding
        bytes_written = 0
        remaining = data.length

        while remaining > 0
          block_index = entry.offset / BLOCK_SIZE
          offset_in_block = entry.offset % BLOCK_SIZE

          # Get or allocate the block
          block_number = get_block_number(inode, block_index)
          if block_number.nil?
            # Need to allocate a new block
            block_number = allocate_block_for_inode(inode, block_index)
            return bytes_written if block_number.nil?  # Disk full
          end

          # Read existing block data (for partial writes)
          block_data = read_block(block_number).dup

          # Calculate how many bytes to write to this block
          bytes_to_write = [remaining, BLOCK_SIZE - offset_in_block].min

          # Overwrite the relevant portion
          block_data[offset_in_block, bytes_to_write] = data[bytes_written, bytes_to_write]

          # Write the block back
          write_block(block_number, block_data)

          entry.offset += bytes_to_write
          bytes_written += bytes_to_write
          remaining -= bytes_to_write

          # Update file size if we wrote past the end
          inode.size = entry.offset if entry.offset > inode.size
        end

        inode.modified_at = Time.now
        @superblock.free_blocks = @block_bitmap.free_count
        bytes_written
      end

      # ======================================================================
      # lseek — Reposition the File Offset
      # ======================================================================
      #
      # lseek changes where the next read or write will occur within a file.
      #
      # @param fd [Integer] The file descriptor
      # @param offset [Integer] The new offset (interpretation depends on whence)
      # @param whence [Integer] SEEK_SET, SEEK_CUR, or SEEK_END
      # @return [Integer] The new absolute offset, or -1 on error
      def lseek(fd, offset, whence)
        global_fd = @fd_table.get(fd)
        return -1 if global_fd.nil?

        entry = @open_file_table.get(global_fd)
        return -1 if entry.nil?

        inode = @inode_table.get(entry.inode_number)
        return -1 if inode.nil?

        new_offset = case whence
        when SEEK_SET
          offset
        when SEEK_CUR
          entry.offset + offset
        when SEEK_END
          inode.size + offset
        else
          return -1
        end

        return -1 if new_offset < 0

        entry.offset = new_offset
        new_offset
      end

      # ======================================================================
      # stat — Get File Metadata
      # ======================================================================
      #
      # Returns the inode for a given path, providing access to all metadata
      # (type, size, permissions, timestamps, link count).
      #
      # @param path [String] Absolute path to stat
      # @return [Inode, nil] The inode, or nil if not found
      def stat(path)
        inode_number = resolve_path(path)
        return nil if inode_number.nil?

        @inode_table.get(inode_number)
      end

      # ======================================================================
      # mkdir — Create a Directory
      # ======================================================================
      #
      # Creating a directory involves:
      #   1. Resolve the parent directory
      #   2. Allocate a new inode (type = DIRECTORY)
      #   3. Allocate a data block for "." and ".." entries
      #   4. Add an entry in the parent directory pointing to the new inode
      #   5. Increment the parent's link_count (because ".." points to it)
      #
      # @param path [String] Absolute path of the directory to create
      # @return [Integer] 0 on success, -1 on error
      def mkdir(path)
        # Split path into parent and name
        parent_path, name = split_path(path)
        return -1 if name.nil? || name.empty?

        # Resolve the parent directory
        parent_inode_num = resolve_path(parent_path)
        return -1 if parent_inode_num.nil?

        parent_inode = @inode_table.get(parent_inode_num)
        return -1 unless parent_inode&.directory?

        # Check if name already exists in parent
        return -1 unless resolve_in_directory(parent_inode, name).nil?

        # Allocate a new inode for the directory
        new_inode = @inode_table.allocate(FILE_TYPE_DIRECTORY)
        return -1 if new_inode.nil?

        new_inode.link_count = 2  # "." and ".." both reference directories

        # Allocate a data block for the directory entries
        block_num = @block_bitmap.allocate
        if block_num.nil?
          @inode_table.free(new_inode.inode_number)
          return -1
        end
        new_inode.direct_blocks[0] = block_num

        # Write "." and ".." entries
        dot = DirectoryEntry.new(".", new_inode.inode_number)
        dotdot = DirectoryEntry.new("..", parent_inode_num)
        dir_data = DirectoryEntry.serialize_all([dot, dotdot])
        new_inode.size = dir_data.length
        write_block(block_num, dir_data)

        # Add entry in parent directory
        add_directory_entry(parent_inode, name, new_inode.inode_number)

        # Increment parent's link_count (because ".." points to parent)
        parent_inode.link_count += 1

        # Update superblock
        @superblock.free_blocks = @block_bitmap.free_count
        @superblock.free_inodes = @inode_table.free_count

        0
      end

      # ======================================================================
      # readdir — List Directory Entries
      # ======================================================================
      #
      # Returns all directory entries for the given path.
      #
      # @param path [String] Absolute path of the directory
      # @return [Array<DirectoryEntry>, nil] Array of entries, or nil on error
      def readdir(path)
        inode_number = resolve_path(path)
        return nil if inode_number.nil?

        inode = @inode_table.get(inode_number)
        return nil unless inode&.directory?

        read_directory_entries(inode)
      end

      # ======================================================================
      # unlink — Remove a Directory Entry
      # ======================================================================
      #
      # Unlinking removes a name from a directory and decrements the target
      # inode's link_count. If link_count reaches 0, the inode and all its
      # data blocks are freed.
      #
      # This is how file deletion works in Unix: you don't delete files,
      # you remove names. A file is only truly deleted when the last name
      # pointing to it is removed AND no process has it open.
      #
      # @param path [String] Absolute path to unlink
      # @return [Integer] 0 on success, -1 on error
      def unlink(path)
        parent_path, name = split_path(path)
        return -1 if name.nil? || name.empty?

        parent_inode_num = resolve_path(parent_path)
        return -1 if parent_inode_num.nil?

        parent_inode = @inode_table.get(parent_inode_num)
        return -1 unless parent_inode&.directory?

        # Find the entry in the parent
        target_inode_num = resolve_in_directory(parent_inode, name)
        return -1 if target_inode_num.nil?

        target_inode = @inode_table.get(target_inode_num)
        return -1 if target_inode.nil?

        # Don't unlink directories (use rmdir for that)
        return -1 if target_inode.directory?

        # Remove the entry from the parent directory
        remove_directory_entry(parent_inode, name)

        # Decrement link count
        target_inode.link_count -= 1

        # If link_count reaches 0, free the inode and its blocks
        if target_inode.link_count <= 0
          free_inode_blocks(target_inode)
          @inode_table.free(target_inode_num)
        end

        @superblock.free_blocks = @block_bitmap.free_count
        @superblock.free_inodes = @inode_table.free_count

        0
      end

      # ======================================================================
      # resolve_path — Walk the Directory Tree
      # ======================================================================
      #
      # Path resolution is the algorithm that turns a string like
      # "/home/alice/notes.txt" into an inode number. It works by:
      #
      #   1. Starting at the root inode (inode 0)
      #   2. Splitting the path by "/"
      #   3. For each component, looking it up in the current directory
      #   4. Moving to the inode the entry points to
      #   5. Repeating until all components are consumed
      #
      # @param path [String] Absolute path to resolve
      # @return [Integer, nil] The inode number, or nil if not found
      def resolve_path(path)
        return ROOT_INODE if path == "/"

        components = path.split("/").reject(&:empty?)
        return ROOT_INODE if components.empty?

        current_inode_num = ROOT_INODE

        components.each do |component|
          inode = @inode_table.get(current_inode_num)
          return nil if inode.nil? || !inode.directory?

          next_inode = resolve_in_directory(inode, component)
          return nil if next_inode.nil?

          current_inode_num = next_inode
        end

        current_inode_num
      end

      # ======================================================================
      # dup_fd / dup2_fd — Duplicate File Descriptors
      # ======================================================================

      # Duplicates a file descriptor. The new fd is the lowest available
      # number and shares the same OpenFile entry (including offset).
      #
      # @param old_fd [Integer] The fd to duplicate
      # @return [Integer, nil] The new fd, or nil on error
      def dup_fd(old_fd)
        global_fd = @fd_table.get(old_fd)
        return nil if global_fd.nil?

        @open_file_table.dup(global_fd)
        new_fd = @fd_table.dup_fd(old_fd)
        new_fd
      end

      # Duplicates a file descriptor to a specific number.
      # If new_fd is already open, it is closed first.
      #
      # @param old_fd [Integer] Source fd
      # @param new_fd [Integer] Destination fd number
      # @return [Integer, nil] new_fd on success, nil on error
      def dup2_fd(old_fd, new_fd)
        global_fd = @fd_table.get(old_fd)
        return nil if global_fd.nil?

        # If new_fd is open, close it first
        existing_global = @fd_table.get(new_fd)
        if existing_global
          @open_file_table.close(existing_global)
        end

        @open_file_table.dup(global_fd)
        @fd_table.dup2(old_fd, new_fd)
      end

      private

      # ======================================================================
      # Private Helper Methods
      # ======================================================================

      # Reads a data block from in-memory storage.
      #
      # @param block_number [Integer] The block to read
      # @return [String] Binary string of BLOCK_SIZE bytes
      def read_block(block_number)
        @blocks[block_number] || ("\x00".b * BLOCK_SIZE)
      end

      # Writes data to a data block in in-memory storage. Pads with zeros
      # if data is shorter than BLOCK_SIZE.
      #
      # @param block_number [Integer] The block to write to
      # @param data [String] The data to write (up to BLOCK_SIZE bytes)
      def write_block(block_number, data)
        padded = data.b.ljust(BLOCK_SIZE, "\x00")
        @blocks[block_number] = padded[0, BLOCK_SIZE]
      end

      # Splits an absolute path into (parent_path, name).
      # Example: "/home/alice/notes.txt" → ["/home/alice", "notes.txt"]
      #          "/notes.txt"           → ["/", "notes.txt"]
      #
      # @param path [String] Absolute path
      # @return [Array(String, String)] Parent path and file/dir name
      def split_path(path)
        parts = path.split("/").reject(&:empty?)
        return ["/", nil] if parts.empty?

        name = parts.pop
        parent = parts.empty? ? "/" : "/#{parts.join("/")}"
        [parent, name]
      end

      # Looks up a name in a directory's entries.
      #
      # @param dir_inode [Inode] The directory inode to search
      # @param name [String] The name to look for
      # @return [Integer, nil] The inode number if found, nil otherwise
      def resolve_in_directory(dir_inode, name)
        entries = read_directory_entries(dir_inode)
        entry = entries.find { |e| e.name == name }
        entry&.inode_number
      end

      # Reads all directory entries from a directory inode's data blocks.
      #
      # @param dir_inode [Inode] The directory inode
      # @return [Array<DirectoryEntry>] All entries in the directory
      def read_directory_entries(dir_inode)
        data = "".b
        block_index = 0

        while block_index < DIRECT_BLOCKS
          block_num = dir_inode.direct_blocks[block_index]
          break if block_num.nil?

          block_data = read_block(block_num)
          data << block_data
          block_index += 1
        end

        # Trim to actual directory size
        data = data[0, dir_inode.size] if dir_inode.size < data.length

        DirectoryEntry.deserialize_all(data)
      end

      # Adds a new entry to a directory.
      #
      # @param dir_inode [Inode] The directory inode to add to
      # @param name [String] The name for the new entry
      # @param inode_number [Integer] The inode the entry points to
      def add_directory_entry(dir_inode, name, inode_number)
        # Read existing entries
        entries = read_directory_entries(dir_inode)
        entries << DirectoryEntry.new(name, inode_number)

        # Serialize all entries
        data = DirectoryEntry.serialize_all(entries)
        dir_inode.size = data.length

        # Write back to blocks, allocating new ones if needed
        write_data_to_inode(dir_inode, data)
      end

      # Removes a named entry from a directory.
      #
      # @param dir_inode [Inode] The directory inode to modify
      # @param name [String] The name to remove
      def remove_directory_entry(dir_inode, name)
        entries = read_directory_entries(dir_inode)
        entries.reject! { |e| e.name == name }

        data = DirectoryEntry.serialize_all(entries)
        dir_inode.size = data.length

        write_data_to_inode(dir_inode, data)
      end

      # Writes data across an inode's data blocks, allocating new blocks
      # as needed.
      #
      # @param inode [Inode] The inode to write data for
      # @param data [String] The binary data to write
      def write_data_to_inode(inode, data)
        block_index = 0
        offset = 0

        while offset < data.length
          block_num = inode.direct_blocks[block_index]
          if block_num.nil?
            block_num = @block_bitmap.allocate
            return if block_num.nil?  # Disk full

            inode.direct_blocks[block_index] = block_num
          end

          chunk = data[offset, BLOCK_SIZE] || "".b
          write_block(block_num, chunk)

          offset += BLOCK_SIZE
          block_index += 1
        end
      end

      # Creates a new file at the given path.
      #
      # @param path [String] Absolute path for the new file
      # @param file_type [Integer] The file type (REGULAR, DIRECTORY, etc.)
      # @return [Integer, nil] The new inode number, or nil on error
      def create_file(path, file_type)
        parent_path, name = split_path(path)
        return nil if name.nil? || name.empty?

        parent_inode_num = resolve_path(parent_path)
        return nil if parent_inode_num.nil?

        parent_inode = @inode_table.get(parent_inode_num)
        return nil unless parent_inode&.directory?

        # Allocate a new inode
        new_inode = @inode_table.allocate(file_type)
        return nil if new_inode.nil?

        new_inode.link_count = 1

        # Add entry in parent directory
        add_directory_entry(parent_inode, name, new_inode.inode_number)

        @superblock.free_inodes = @inode_table.free_count
        new_inode.inode_number
      end

      # Truncates a file to zero length, freeing all its data blocks.
      #
      # @param inode_number [Integer] The inode to truncate
      def truncate_inode(inode_number)
        inode = @inode_table.get(inode_number)
        return if inode.nil?

        free_inode_blocks(inode)
        inode.size = 0
        inode.modified_at = Time.now
        @superblock.free_blocks = @block_bitmap.free_count
      end

      # Frees all data blocks owned by an inode (direct and indirect).
      #
      # @param inode [Inode] The inode whose blocks to free
      def free_inode_blocks(inode)
        # Free direct blocks
        DIRECT_BLOCKS.times do |i|
          block_num = inode.direct_blocks[i]
          if block_num
            @block_bitmap.free(block_num)
            inode.direct_blocks[i] = nil
          end
        end

        # Free indirect block and all blocks it points to
        if inode.indirect_block
          indirect_data = read_block(inode.indirect_block)
          # Each pointer is 4 bytes (big-endian)
          (BLOCK_SIZE / 4).times do |i|
            ptr_bytes = indirect_data[i * 4, 4]
            break if ptr_bytes.nil?

            ptr = ptr_bytes.unpack1("N")
            @block_bitmap.free(ptr) if ptr > 0
          end
          @block_bitmap.free(inode.indirect_block)
          inode.indirect_block = nil
        end
      end

      # Returns the block number for a given logical block index in an inode.
      # For indices 0..11, uses direct blocks. For index 12+, uses the
      # indirect block.
      #
      # @param inode [Inode] The inode to look up
      # @param block_index [Integer] Logical block index within the file
      # @return [Integer, nil] The data block number, or nil if not allocated
      def get_block_number(inode, block_index)
        if block_index < DIRECT_BLOCKS
          inode.direct_blocks[block_index]
        elsif inode.indirect_block
          indirect_data = read_block(inode.indirect_block)
          indirect_index = block_index - DIRECT_BLOCKS
          ptr_offset = indirect_index * 4
          return nil if ptr_offset + 4 > BLOCK_SIZE

          ptr = indirect_data[ptr_offset, 4].unpack1("N")
          ptr > 0 ? ptr : nil
        end
      end

      # Allocates a data block for a given logical block index in an inode.
      # Handles both direct and indirect block allocation.
      #
      # @param inode [Inode] The inode to allocate for
      # @param block_index [Integer] Logical block index
      # @return [Integer, nil] The allocated block number, or nil if disk full
      def allocate_block_for_inode(inode, block_index)
        if block_index < DIRECT_BLOCKS
          block_num = @block_bitmap.allocate
          return nil if block_num.nil?

          inode.direct_blocks[block_index] = block_num
          block_num
        else
          # Need indirect block — allocate it if not present
          if inode.indirect_block.nil?
            indirect_num = @block_bitmap.allocate
            return nil if indirect_num.nil?

            inode.indirect_block = indirect_num
            # Initialize indirect block to all zeros
            write_block(indirect_num, "\x00".b * BLOCK_SIZE)
          end

          # Allocate the actual data block
          indirect_index = block_index - DIRECT_BLOCKS
          # Check if the indirect index exceeds the capacity of one
          # indirect block (BLOCK_SIZE / 4 pointers).
          max_indirect_entries = BLOCK_SIZE / 4
          return nil if indirect_index >= max_indirect_entries

          block_num = @block_bitmap.allocate
          return nil if block_num.nil?

          # Write the pointer into the indirect block
          indirect_data = read_block(inode.indirect_block).dup
          indirect_data[indirect_index * 4, 4] = [block_num].pack("N")
          write_block(inode.indirect_block, indirect_data)

          block_num
        end
      end
    end
  end
end
