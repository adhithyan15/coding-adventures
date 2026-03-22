# frozen_string_literal: true

# Entry point for the coding_adventures_ipc gem.
#
# This gem implements three classic Inter-Process Communication (IPC)
# mechanisms that operating systems use to let isolated processes exchange
# data:
#
#   1. Pipes           - unidirectional byte streams (like pneumatic tubes)
#   2. Message Queues  - FIFO queues of typed messages (like a shared mailbox)
#   3. Shared Memory   - a memory region visible to multiple processes
#                        (like a shared whiteboard)
#
# Plus an IPCManager that acts as the kernel component owning all IPC
# resources.
#
# Usage:
#   require "coding_adventures_ipc"
#
#   # Create a pipe
#   pipe = CodingAdventures::Ipc::Pipe.new
#   pipe.write([72, 101, 108, 108, 111])  # "Hello"
#   pipe.read(5)  # => [72, 101, 108, 108, 111]
#
#   # Create a message queue
#   mq = CodingAdventures::Ipc::MessageQueue.new
#   mq.send(1, [65, 66, 67])  # type=1, body="ABC"
#   msg = mq.receive(1)       # => Message(msg_type=1, body=[65, 66, 67])
#
#   # Create shared memory
#   shm = CodingAdventures::Ipc::SharedMemoryRegion.new(
#     name: "my_region", size: 1024, owner_pid: 1
#   )
#   shm.attach(1)
#   shm.write(0, [1, 2, 3])
#   shm.read(0, 3)  # => [1, 2, 3]

require "set"

require_relative "coding_adventures/ipc/version"
require_relative "coding_adventures/ipc/pipe"
require_relative "coding_adventures/ipc/message_queue"
require_relative "coding_adventures/ipc/shared_memory"
require_relative "coding_adventures/ipc/ipc_manager"
