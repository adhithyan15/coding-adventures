# frozen_string_literal: true

# IPC Manager -- the kernel component that owns all IPC resources.
#
# In a real OS kernel, IPC resources (pipes, message queues, shared memory
# segments) are not owned by individual processes -- they are global kernel
# objects managed by a central authority. The IPCManager is that authority.
#
# When a process calls pipe(), msgget(), or shmget(), the kernel dispatches
# to the IPCManager, which creates the resource and returns an identifier
# (file descriptor for pipes, queue ID for message queues, segment ID for
# shared memory).
#
# === Resource Lifecycle ===
#
#   +----------+     create_pipe()      +--------+
#   | Process  | ---------------------> | Pipe   |
#   |          | <-- (pipe_id, r, w) -- |        |
#   +----------+                        +--------+
#        |
#        | close_pipe_read(pipe_id)
#        v
#   Reader count decremented. If both reader and writer counts hit 0,
#   the pipe is eligible for cleanup.
#
# === Identifier Scheme ===
#
# Each resource type has its own ID counter, starting from 0. Pipe IDs,
# queue IDs, and shared memory IDs are independent -- pipe 0 and queue 0
# are different resources.
#
# Pipes are identified by integer IDs. When created, the manager returns
# a triple: (pipe_id, read_fd, write_fd). The read_fd and write_fd are
# also integer identifiers (simulating file descriptors) that the process
# uses for read() and write() calls.
#
# Message queues and shared memory are identified by **name** (a string key).
# This allows unrelated processes to find the same resource by agreeing on
# a name, much like a phone number.

module CodingAdventures
  module Ipc
    class IpcManager
      def initialize
        # Pipes are stored by integer ID. Each pipe gets a unique ID from
        # the @next_pipe_id counter.
        @pipes = {}
        @next_pipe_id = 0

        # File descriptor table: maps fd numbers to {pipe_id, mode} pairs.
        # Mode is :read or :write, indicating which end of the pipe this
        # fd represents.
        @fd_table = {}
        @next_fd = 0

        # Message queues are stored by name (string key). The name is how
        # unrelated processes find the same queue.
        @message_queues = {}

        # Shared memory regions are stored by name (string key), similar
        # to message queues.
        @shared_memory = {}
      end

      # ---------------------------------------------------------------
      # Pipe operations
      # ---------------------------------------------------------------

      # Create a new pipe and return (pipe_id, read_fd, write_fd).
      #
      # This is the kernel's implementation of the pipe() system call.
      # It creates a Pipe object and two file descriptors: one for reading
      # and one for writing. The process receives these fds and uses them
      # with read() and write().
      def create_pipe(capacity: DEFAULT_PIPE_CAPACITY)
        pipe_id = @next_pipe_id
        @next_pipe_id += 1

        pipe = Pipe.new(capacity: capacity)
        @pipes[pipe_id] = pipe

        # Allocate two file descriptors: one for each end of the pipe.
        read_fd = allocate_fd(pipe_id, :read)
        write_fd = allocate_fd(pipe_id, :write)

        [pipe_id, read_fd, write_fd]
      end

      # Get the Pipe object by its ID. Returns nil if not found.
      def get_pipe(pipe_id)
        @pipes[pipe_id]
      end

      # Close the read end of a pipe (via file descriptor).
      #
      # In a real kernel, close(fd) looks up what the fd points to. If it's
      # the read end of a pipe, decrement the pipe's reader_count.
      def close_pipe_read(pipe_id)
        pipe = @pipes[pipe_id]
        return unless pipe

        pipe.close_read
      end

      # Close the write end of a pipe.
      def close_pipe_write(pipe_id)
        pipe = @pipes[pipe_id]
        return unless pipe

        pipe.close_write
      end

      # List all active pipe IDs.
      def list_pipes
        @pipes.keys
      end

      # ---------------------------------------------------------------
      # Message queue operations
      # ---------------------------------------------------------------

      # Create (or retrieve) a message queue by name.
      #
      # If a queue with this name already exists, return it (idempotent).
      # This mirrors shmget() / msgget() behavior in System V IPC: the key
      # is a rendezvous point for unrelated processes.
      def create_message_queue(name, max_messages: DEFAULT_MAX_MESSAGES,
        max_message_size: DEFAULT_MAX_MESSAGE_SIZE)
        @message_queues[name] ||= MessageQueue.new(
          max_messages: max_messages,
          max_message_size: max_message_size
        )
      end

      # Get a message queue by name. Returns nil if not found.
      def get_message_queue(name)
        @message_queues[name]
      end

      # Delete a message queue by name.
      #
      # Any unread messages in the queue are lost. In a real OS, this
      # would wake up any blocked receivers with an error.
      def delete_message_queue(name)
        @message_queues.delete(name)
      end

      # List all message queue names.
      def list_message_queues
        @message_queues.keys
      end

      # ---------------------------------------------------------------
      # Shared memory operations
      # ---------------------------------------------------------------

      # Create (or retrieve) a shared memory region by name.
      #
      # Parameters:
      #   name      - string key for finding this region
      #   size      - region size in bytes (only used when creating)
      #   owner_pid - the process ID of the creator
      #
      # If a region with this name already exists, return it (ignoring
      # the size and owner_pid parameters).
      def create_shared_memory(name, size:, owner_pid:)
        @shared_memory[name] ||= SharedMemoryRegion.new(
          name: name,
          size: size,
          owner_pid: owner_pid
        )
      end

      # Get a shared memory region by name. Returns nil if not found.
      def get_shared_memory(name)
        @shared_memory[name]
      end

      # Delete a shared memory region by name.
      #
      # In a real OS, the region is only truly freed when the last process
      # detaches. We simplify by removing it immediately.
      def delete_shared_memory(name)
        @shared_memory.delete(name)
      end

      # List all shared memory region names.
      def list_shared_memory
        @shared_memory.keys
      end

      private

      # Allocate a file descriptor number and record what it points to.
      def allocate_fd(pipe_id, mode)
        fd = @next_fd
        @next_fd += 1
        @fd_table[fd] = {pipe_id: pipe_id, mode: mode}
        fd
      end
    end
  end
end
