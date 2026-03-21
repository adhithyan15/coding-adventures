# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Synchronization primitives -- Fence, Semaphore, Event.
# ---------------------------------------------------------------------------
#
# === The Synchronization Problem ===
#
# CPUs and GPUs run asynchronously. When you submit a command buffer, the
# CPU doesn't wait -- it immediately returns and can do other work (or submit
# more GPU work). But at some point you need to know:
#
#     "Has the GPU finished yet?"
#
# That's what synchronization primitives solve.
#
# === Three Levels of Synchronization ===
#
#     +-------------------------------------------------------------+
#     |  FENCE (CPU <-> GPU)                                         |
#     |                                                              |
#     |  CPU submits work with a fence attached, then calls          |
#     |  fence.wait to block until the GPU signals it.               |
#     |                                                              |
#     |  CPU:  [submit(fence=F)]----------[F.wait]--[read results]   |
#     |  GPU:  ----------[execute]--[signal F]                       |
#     |                                                              |
#     |  Use case: "wait for my kernel to finish so I can read"      |
#     +-------------------------------------------------------------+
#
#     +-------------------------------------------------------------+
#     |  SEMAPHORE (GPU Queue <-> GPU Queue)                         |
#     |                                                              |
#     |  Queue A signals a semaphore when its CB completes.          |
#     |  Queue B waits on that semaphore before starting.            |
#     |                                                              |
#     |  Transfer Queue: [upload data]--[signal S]                   |
#     |  Compute Queue:  ---------------[wait S]--[run kernel]       |
#     |                                                              |
#     |  Use case: "compute waits for transfer to finish"            |
#     +-------------------------------------------------------------+
#
#     +-------------------------------------------------------------+
#     |  EVENT (GPU <-> GPU, fine-grained)                           |
#     |                                                              |
#     |  Set and waited on WITHIN command buffers.                   |
#     |                                                              |
#     |  CB: [dispatch A]--[set_event E]--[wait_event E]--[disp B]   |
#     |                                                              |
#     |  Use case: "barrier between two dispatches in the same CB"   |
#     +-------------------------------------------------------------+

module CodingAdventures
  module ComputeRuntime
    # =====================================================================
    # Fence -- CPU waits for GPU
    # =====================================================================
    #
    # === Fence Lifecycle ===
    #
    #     create_fence(signaled: false)
    #         |
    #         v
    #     [unsignaled] --submit(fence: f)---> [GPU working]
    #         ^                                    |
    #         |                              GPU finishes
    #         |                                    |
    #         +---- reset <-- [signaled] <---------+
    #                              |
    #                          wait returns
    #
    # You attach a fence to a queue submission. When the GPU finishes all
    # the command buffers in that submission, it signals the fence. The CPU
    # can then call wait to block until the signal arrives.
    #
    # Fences are reusable -- call reset to clear the signal, then attach
    # to another submission.
    class Fence
      @@next_id = 0

      attr_reader :fence_id, :wait_cycles

      def initialize(signaled: false)
        @fence_id = @@next_id
        @@next_id += 1
        @signaled = signaled
        @wait_cycles = 0
      end

      # Whether the GPU has signaled this fence.
      def signaled? = @signaled

      # Alias for compatibility with Python API style.
      def signaled = @signaled

      # Signal the fence (called by the runtime when GPU finishes).
      def signal
        @signaled = true
      end

      # Wait for the fence to be signaled.
      #
      # In a real system, this blocks the CPU thread. In our simulator,
      # the fence is either already signaled (because we run synchronously)
      # or it's not (which would be a programming error).
      #
      # @param timeout_cycles [Integer, nil] Maximum cycles to wait.
      # @return [Boolean] true if signaled, false if timeout expired.
      def wait(timeout_cycles: nil)
        @signaled
      end

      # Reset the fence to unsignaled state for reuse.
      def reset
        @signaled = false
        @wait_cycles = 0
      end
    end

    # =====================================================================
    # Semaphore -- GPU-to-GPU synchronization
    # =====================================================================
    #
    # === How Semaphores Differ from Fences ===
    #
    # Fences are for CPU <-> GPU synchronization (CPU blocks until GPU done).
    # Semaphores are for GPU <-> GPU synchronization between different queues.
    #
    # The CPU never waits on a semaphore -- they're entirely GPU-side.
    #
    # === Usage Pattern ===
    #
    #     # Transfer queue signals when upload is done
    #     transfer_queue.submit([upload_cb], signal_semaphores: [sem])
    #
    #     # Compute queue waits for upload before starting kernel
    #     compute_queue.submit([compute_cb], wait_semaphores: [sem])
    class Semaphore
      @@next_id = 0

      attr_reader :semaphore_id

      def initialize
        @semaphore_id = @@next_id
        @@next_id += 1
        @signaled = false
      end

      # Whether this semaphore has been signaled.
      def signaled? = @signaled

      # Alias for compatibility.
      def signaled = @signaled

      # Signal the semaphore (called by runtime after queue completes).
      def signal
        @signaled = true
      end

      # Reset to unsignaled (called by runtime when consumed by a wait).
      def reset
        @signaled = false
      end
    end

    # =====================================================================
    # Event -- fine-grained GPU-side synchronization
    # =====================================================================
    #
    # === Events vs Barriers ===
    #
    # Pipeline barriers are implicit -- they're executed inline in a command
    # buffer. Events are explicit -- you set them at one point and wait for
    # them at another, potentially in a different command buffer or even
    # from the CPU.
    #
    # === Usage Patterns ===
    #
    # GPU-side (in command buffer):
    #     cb.cmd_set_event(event, :compute)
    #     cb.cmd_wait_event(event, src_stage: :compute, dst_stage: :compute)
    #
    # CPU-side:
    #     event.set          # CPU signals
    #     event.status       # CPU checks without blocking
    #     event.reset        # CPU clears
    class Event
      @@next_id = 0

      attr_reader :event_id

      def initialize
        @event_id = @@next_id
        @@next_id += 1
        @signaled = false
      end

      # Whether this event has been signaled.
      def signaled? = @signaled

      # Alias for compatibility.
      def signaled = @signaled

      # Signal the event.
      def set
        @signaled = true
      end

      # Clear the event.
      def reset
        @signaled = false
      end

      # Check if signaled without blocking.
      def status
        @signaled
      end
    end
  end
end
