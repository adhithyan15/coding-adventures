# frozen_string_literal: true

module CodingAdventures
  module Actor
    # ═══════════════════════════════════════════════════════════════
    # ActorResult — the return value from an actor's behavior function
    # ═══════════════════════════════════════════════════════════════
    #
    # When an actor processes a message, its behavior function returns
    # an ActorResult that describes what should happen next:
    #
    #   - new_state:        The actor's updated internal state
    #   - messages_to_send: List of [target_id, message] pairs to deliver
    #   - actors_to_create: List of ActorSpec objects for new actors to spawn
    #   - stop:             If true, the actor halts permanently
    #
    # This is a value object — it simply holds the result data.
    # The ActorSystem reads it and carries out the instructions.
    #
    ActorResult = Struct.new(:new_state, :messages_to_send, :actors_to_create, :stop, keyword_init: true) do
      def initialize(new_state:, messages_to_send: nil, actors_to_create: nil, stop: false)
        super(
          new_state: new_state,
          messages_to_send: messages_to_send || [],
          actors_to_create: actors_to_create || [],
          stop: stop
        )
      end
    end

    # ═══════════════════════════════════════════════════════════════
    # ActorSpec — specification for creating a new actor
    # ═══════════════════════════════════════════════════════════════
    #
    # An ActorSpec describes an actor that should be created. It's
    # returned inside ActorResult.actors_to_create when an actor wants
    # to spawn new actors. Think of it as a blueprint.
    #
    #   ActorSpec.new(
    #     actor_id: "worker_1",
    #     initial_state: 0,
    #     behavior: ->(state, msg) { ... }
    #   )
    #
    ActorSpec = Struct.new(:actor_id, :initial_state, :behavior, keyword_init: true)

    # ═══════════════════════════════════════════════════════════════
    # ActorEntity — an isolated unit of computation
    # ═══════════════════════════════════════════════════════════════
    #
    # An actor is a person sitting alone in a soundproofed room with a
    # mail slot in the door. Letters (messages) come in through the slot
    # and pile up in a tray (mailbox). The person reads one letter at a
    # time, thinks about it, possibly writes reply letters and slides
    # them out, and possibly rearranges things on their desk (state).
    # They never leave the room. They never look into anyone else's room.
    #
    # === The Four Things an Actor Can Do
    #
    # 1. Receive a message (from the mailbox)
    # 2. Send messages to other actors it knows about
    # 3. Create new actors
    # 4. Change its own internal state
    #
    # === The Three Things an Actor CANNOT Do
    #
    # 1. Access another actor's internal state
    # 2. Share memory with another actor
    # 3. Communicate except through messages
    #
    # === Processing Guarantees
    #
    # - Sequential: one message at a time, no concurrency within a single actor
    # - At-most-once delivery: a message is processed exactly once or lost
    # - FIFO ordering: messages from the same sender arrive in send order
    #
    # We name this class ActorEntity (not Actor) because CodingAdventures::Actor
    # is the module namespace. The ActorSystem exposes this transparently.
    #
    class ActorEntity
      # The three possible states an actor can be in:
      #   :idle       — waiting for messages, ready to process
      #   :processing — currently handling a message
      #   :stopped    — permanently halted, no more messages accepted
      STATUSES = %i[idle processing stopped].freeze

      attr_reader :id, :status
      attr_accessor :state

      # Create a new actor.
      #
      # @param id [String] Unique identifier — this is the actor's "address".
      # @param state [Object] Initial internal state (can be any type).
      # @param behavior [Proc, Lambda] A callable: (state, message) -> ActorResult.
      def initialize(id:, state:, behavior:)
        @id = id
        @state = state
        @behavior = behavior
        @status = :idle

        # The mailbox is a simple Array used as a FIFO queue.
        # Messages are pushed to the end (enqueue) and shifted from
        # the front (dequeue). In a production system, this would be
        # a thread-safe queue, but for our single-threaded educational
        # implementation, an array is sufficient.
        @mailbox = []
      end

      # Add a message to this actor's mailbox.
      #
      # Messages accumulate in FIFO order. They are not processed
      # immediately — the ActorSystem decides when to call process_next.
      #
      # @param message [Message] The message to enqueue.
      def enqueue(message)
        @mailbox.push(message)
      end

      # Return the number of messages waiting in the mailbox.
      #
      # @return [Integer] The mailbox depth.
      def mailbox_size
        @mailbox.length
      end

      # Check if the mailbox has any messages waiting.
      #
      # @return [Boolean] true if the mailbox is empty.
      def mailbox_empty?
        @mailbox.empty?
      end

      # Process the next message in the mailbox.
      #
      # This is the core of the actor processing loop:
      #   1. Dequeue the oldest message (FIFO)
      #   2. Set status to :processing
      #   3. Call the behavior function with (state, message)
      #   4. Update state from the result
      #   5. Return the result so the ActorSystem can deliver outbound
      #      messages and create new actors
      #
      # If the behavior function raises an exception:
      #   - The actor's state is NOT changed (no partial updates)
      #   - The message is returned via the exception for dead-lettering
      #   - The actor returns to :idle and continues with the next message
      #
      # @return [ActorResult, nil] The behavior's result, or nil if mailbox empty.
      # @raise [BehaviorError] If the behavior function raises an exception.
      def process_next
        return nil if @mailbox.empty?
        return nil if @status == :stopped

        message = @mailbox.shift
        @status = :processing

        begin
          result = @behavior.call(@state, message)
          @state = result.new_state

          if result.stop
            @status = :stopped
          else
            @status = :idle
          end

          result
        rescue => e
          # On exception: state is unchanged, actor goes back to idle,
          # and we wrap the error so the ActorSystem can handle it.
          @status = :idle
          raise BehaviorError.new(e, message)
        end
      end

      # Stop this actor permanently.
      #
      # Once stopped, an actor cannot process any more messages.
      # Remaining messages in the mailbox are drained and returned
      # so the ActorSystem can add them to dead_letters.
      #
      # @return [Array<Message>] All messages that were in the mailbox.
      def stop!
        @status = :stopped
        drained = @mailbox.dup
        @mailbox.clear
        drained
      end
    end

    # Raised when an actor's behavior function throws an exception.
    # Wraps the original error and the message that caused it, so
    # the ActorSystem can log the failure and dead-letter the message.
    class BehaviorError < StandardError
      attr_reader :original_error, :failed_message

      def initialize(original_error, failed_message)
        @original_error = original_error
        @failed_message = failed_message
        super("Actor behavior raised #{original_error.class}: #{original_error.message}")
      end
    end
  end
end
