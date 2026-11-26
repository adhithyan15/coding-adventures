# frozen_string_literal: true

module UnixHistory
  Inode = Data.define(:number, :data, :entries)
  Descriptor = Data.define(:inode, :offset)

  # Directory entries name inodes. Byte reads use the inode and a range.
  class FileSystem
    def initialize
      @inodes = {
        1 => Inode.new(1, nil, {"hello" => 2, "empty" => 3, "etc" => 4}),
        2 => Inode.new(2, "Hello, Unix!\n", {}),
        3 => Inode.new(3, "", {}),
        4 => Inode.new(4, nil, {"notes" => 5}),
        5 => Inode.new(5, "A directory maps a name to an inode.\n" \
                          "An open descriptor tracks a byte offset.\n", {})
      }
    end

    def resolve(path)
      return "EINVAL" if path.empty? || path.include?("\0")

      root = @inodes.fetch(1)
      node = root
      parents = []
      path.split("/").reject(&:empty?).each do |name|
        return "ENOTDIR" if node.data

        case name
        when "."
          next
        when ".."
          node = parents.pop || root
        else
          entry = node.entries[name]
          return "ENOENT" unless entry

          parents << node
          node = @inodes.fetch(entry)
        end
      end
      (path.end_with?("/") && node.data) ? "ENOTDIR" : node
    end

    def read(inode, offset, count)
      @inodes.fetch(inode).data.byteslice(offset, count) || ""
    end
  end

  # Slot ownership and offset tracking are independent of filesystem lookup.
  class DescriptorTable
    def initialize(capacity = 4)
      raise ArgumentError, "descriptor slots must be from 1 through 1024" unless (1..1024).cover?(capacity)

      @slots = Array.new(capacity)
    end

    def get(fd)
      @slots[fd] if fd >= 0
    end

    def open(inode)
      fd = @slots.index(nil)
      @slots[fd] = Descriptor.new(inode, 0) if fd
      fd
    end

    def advance(fd, count)
      slot = get(fd)
      @slots[fd] = slot.with(offset: slot.offset + count) if slot && count >= 0
    end

    def close(fd)
      return false unless get(fd)

      @slots[fd] = nil
      true
    end
  end

  class Kernel
    def initialize
      @fs = FileSystem.new
      @descriptors = DescriptorTable.new
    end

    def open(path)
      event = {call: "open", path: path}
      node = @fs.resolve(path)
      return event.merge(error: node) if node.is_a?(String)
      return event.merge(error: "EISDIR") unless node.data

      fd = @descriptors.open(node.number)
      return event.merge(error: "EMFILE") unless fd

      event.merge(inode: node.number, fd: fd, offset: 0)
    end

    def read(fd, count)
      event = {call: "read", fd: fd}
      slot = @descriptors.get(fd)
      return [event.merge(error: "EBADF"), ""] unless slot
      return [event.merge(error: "EINVAL"), ""] if count < 0

      data = @fs.read(slot.inode, slot.offset, count)
      @descriptors.advance(fd, data.bytesize)
      [event.merge(
        inode: slot.inode,
        offset: slot.offset,
        bytes: data.bytesize,
        nextOffset: slot.offset + data.bytesize
      ), data]
    end

    def close(fd)
      event = {call: "close", fd: fd}
      @descriptors.close(fd) ? event : event.merge(error: "EBADF")
    end

    def list(path)
      event = {call: "list", path: path}
      node = @fs.resolve(path)
      return event.merge(error: node) if node.is_a?(String)
      return event.merge(error: "ENOTDIR") if node.data

      event.merge(inode: node.number, names: node.entries.keys)
    end
  end

  def self.run_command(command)
    words = command.split(/[ \t\n\r\f\v]+/).reject(&:empty?)
    kernel = Kernel.new
    events = []
    output = +""

    case words
    in ["cat", path]
      opened = kernel.open(path)
      events << opened
      if (fd = opened[:fd])
        loop do
          event, data = kernel.read(fd, 8)
          events << event
          break if data.empty?

          output << data
        end
        events << kernel.close(fd)
      end
    in ["ls"] | ["ls", _]
      listed = kernel.list(words[1] || "/")
      events << listed
      listed.fetch(:names, []).each { |name| output << "#{name}\n" }
    in []
      # An empty line has no kernel operations.
    else
      output << "usage: cat PATH or ls [PATH]\n"
    end
    events.each do |event|
      output << "error: #{event[:error]}\n" if event[:error]
    end
    {output: output, events: events}
  end
end
