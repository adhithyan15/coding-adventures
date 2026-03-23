# frozen_string_literal: true

# = DirectoryEntry
#
# A directory in Unix is just a special kind of file. Its data blocks don't
# contain text or binary data — they contain a list of DirectoryEntry records,
# each mapping a name to an inode number.
#
# == Analogy
#
# Think of a directory as a phone book. Each entry has a name ("Alice") and
# a number (inode 23). The phone book doesn't contain Alice herself — it
# just tells you how to find her. Multiple phone books can list the same
# person (hard links), and the person exists independently of any listing.
#
# == Structure
#
#   DirectoryEntry
#   +-----------+---------------+
#   | name      | inode_number  |
#   +-----------+---------------+
#   | "."       |      5        |   ← points to self
#   | ".."      |      0        |   ← points to parent
#   | "notes"   |     23        |   ← a file
#   | "photos"  |     41        |   ← a subdirectory
#   +-----------+---------------+
#
# Every directory always contains at least two entries:
#   "."  — points to the directory's own inode (self-reference)
#   ".." — points to the parent directory's inode
#
# For the root directory ("/"), both "." and ".." point to inode 0 (the root
# is its own parent — there's nothing above it).
#
# == Serialization
#
# Directory entries are serialized into data blocks as length-prefixed
# records. Each entry is stored as:
#   [name_length (1 byte)] [name (variable)] [inode_number (4 bytes)]
#
# This format lets us store variable-length names efficiently while still
# being able to parse the entries sequentially.

module CodingAdventures
  module FileSystem
    class DirectoryEntry
      # The name of this file or directory. Up to 255 characters.
      # No slashes ("/") or null bytes allowed.
      attr_reader :name

      # The inode number this name refers to.
      attr_accessor :inode_number

      # Creates a new directory entry.
      #
      # @param name [String] The file/directory name (max 255 chars)
      # @param inode_number [Integer] The inode this entry points to
      # @raise [ArgumentError] If name is too long or contains invalid chars
      def initialize(name, inode_number)
        raise ArgumentError, "Name cannot be empty" if name.nil? || name.empty?
        raise ArgumentError, "Name too long (max #{MAX_NAME_LENGTH})" if name.length > MAX_NAME_LENGTH
        raise ArgumentError, "Name cannot contain '/'" if name.include?("/")

        @name = name
        @inode_number = inode_number
      end

      # Serializes this entry to bytes for storage in a data block.
      #
      # Format: [name_length:1 byte][name:variable][inode_number:4 bytes big-endian]
      #
      # @return [String] Binary string of the serialized entry
      def serialize
        name_bytes = @name.encode("UTF-8")
        [name_bytes.length].pack("C") + name_bytes + [@inode_number].pack("N")
      end

      # Deserializes a directory entry from a binary string at the given offset.
      #
      # @param data [String] The binary data containing serialized entries
      # @param offset [Integer] Byte offset to start reading from
      # @return [Array(DirectoryEntry, Integer)] The entry and the next offset,
      #   or nil if there is no valid entry at this offset
      def self.deserialize(data, offset)
        return nil if offset >= data.length

        name_length = data.getbyte(offset)
        return nil if name_length.nil? || name_length == 0
        return nil if offset + 1 + name_length + 4 > data.length

        name = data[(offset + 1)...(offset + 1 + name_length)]
        inode_bytes = data[(offset + 1 + name_length)...(offset + 1 + name_length + 4)]
        return nil if inode_bytes.nil? || inode_bytes.length < 4

        inode_number = inode_bytes.unpack1("N")
        entry = new(name, inode_number)
        next_offset = offset + 1 + name_length + 4
        [entry, next_offset]
      end

      # Serializes an array of directory entries into a single binary string.
      #
      # @param entries [Array<DirectoryEntry>] Entries to serialize
      # @return [String] Binary string of all serialized entries
      def self.serialize_all(entries)
        entries.map(&:serialize).join
      end

      # Deserializes all directory entries from a binary string.
      #
      # @param data [String] Binary data containing serialized entries
      # @return [Array<DirectoryEntry>] All entries found in the data
      def self.deserialize_all(data)
        entries = []
        offset = 0
        while offset < data.length
          result = deserialize(data, offset)
          break if result.nil?

          entry, offset = result
          entries << entry
        end
        entries
      end
    end
  end
end
