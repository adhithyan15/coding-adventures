# frozen_string_literal: true

module CodingAdventures
  module Actor
    # ═══════════════════════════════════════════════════════════════
    # ActorSystem — the runtime for managing actors and message delivery
    # ═══════════════════════════════════════════════════════════════
    #
    # The ActorSystem is the "office building" that actors live in. It has:
    #   - A directory: which actors exist and their addresses
    #   - A mail room: message routing between actors
    #   - Dead letters: messages that could not be delivered
    #   - Channels: named message logs for pub/sub communication
    #
    # The ActorSystem does NOT read messages — it only routes them.
    # It does NOT modify actor state — only behavior functions do that.
    # It is purely a lifecycle and routing manager.
    #
    # === Processing Model (V1)
    #
    # In V1, actors are processed one at a time in round-robin order.
    # This is sequential, not parallel. True parallelism (using threads
    # or processes) is a future enhancement. The sequential model is
    # simpler to test, debug, and reason about.
    #
    # === Dead Letters
    #
    # When a message cannot be delivered — because the target actor
    # does not exist or is stopped — the message goes to the dead
    # letters list. This is a debugging aid: you can inspect dead
    # letters to find routing errors, missing actors, or timing issues.
    #
    class ActorSystem
      attr_reader :dead_letters

      def initialize
        # Map of actor_id -> ActorEntity. This is the registry of all
        # living actors in the system.
        @actors = {}

        # Map of channel_id -> Channel. All channels in the system.
        @channels = {}

        # Messages that could not be delivered. Useful for debugging.
        @dead_letters = []

        # Monotonic clock counter. Provides ordering for events in
        # the system. Not used for wall-clock timing.
        @clock = 0
      end

      # ─── Actor Lifecycle ───────────────────────────────────────

      # Create a new actor and register it in the system.
      #
      # The actor starts in :idle status with an empty mailbox,
      # ready to receive messages.
      #
      # @param actor_id [String] Unique identifier for the actor.
      # @param initial_state [Object] The actor's starting state.
      # @param behavior [Proc] A callable: (state, message) -> ActorResult.
      # @return [String] The actor_id.
      # @raise [ArgumentError] If an actor with this ID already exists.
      def create_actor(actor_id, initial_state, behavior)
        if @actors.key?(actor_id)
          raise ArgumentError, "Actor with id '#{actor_id}' already exists"
        end

        @actors[actor_id] = ActorEntity.new(
          id: actor_id,
          state: initial_state,
          behavior: behavior
        )

        actor_id
      end

      # Stop an actor and drain its mailbox to dead letters.
      #
      # Once stopped, an actor will never process another message.
      # Any messages currently in its mailbox are moved to dead_letters
      # so they can be inspected for debugging.
      #
      # @param actor_id [String] The actor to stop.
      def stop_actor(actor_id)
        actor = @actors[actor_id]
        return unless actor

        drained = actor.stop!
        @dead_letters.concat(drained)
      end

      # Get the current status of an actor.
      #
      # @param actor_id [String] The actor to check.
      # @return [String] "idle", "processing", or "stopped".
      # @raise [ArgumentError] If the actor does not exist.
      def get_actor_status(actor_id)
        actor = @actors[actor_id]
        raise ArgumentError, "Actor '#{actor_id}' not found" unless actor

        actor.status.to_s
      end

      # ─── Messaging ─────────────────────────────────────────────

      # Send a message to an actor's mailbox.
      #
      # If the target actor does not exist or is stopped, the message
      # goes to dead_letters instead. This is "fire and forget" — the
      # sender does not get an error. Dead letters are inspected later
      # for debugging.
      #
      # @param target_id [String] The actor to deliver the message to.
      # @param message [Message] The message to deliver.
      def send_message(target_id, message)
        actor = @actors[target_id]

        if actor.nil? || actor.status == :stopped
          @dead_letters.push(message)
          return
        end

        actor.enqueue(message)
      end

      # ─── Processing ────────────────────────────────────────────

      # Process one message from an actor's mailbox.
      #
      # This is the core processing step:
      #   1. Dequeue the oldest message
      #   2. Call the behavior function
      #   3. Update state
      #   4. Deliver outbound messages
      #   5. Create any new actors
      #
      # If the behavior function raises an exception:
      #   - The message goes to dead_letters
      #   - The actor's state is unchanged
      #   - The actor returns to :idle
      #   - Processing continues with the next message
      #
      # @param actor_id [String] The actor to process.
      # @return [Boolean] true if a message was processed, false if mailbox empty.
      def process_next(actor_id)
        actor = @actors[actor_id]
        return false unless actor
        return false if actor.mailbox_empty?
        return false if actor.status == :stopped

        begin
          result = actor.process_next
          return false if result.nil?

          # Create any new actors FIRST — so that outbound messages
          # can be delivered to freshly spawned actors. Without this
          # ordering, a "create worker + send task to worker" pattern
          # would fail because the worker wouldn't exist yet when the
          # message is delivered.
          result.actors_to_create.each do |spec|
            create_actor(spec.actor_id, spec.initial_state, spec.behavior)
          end

          # Deliver outbound messages. Each entry is [target_id, message].
          result.messages_to_send.each do |target_id, msg|
            send_message(target_id, msg)
          end

          # If the actor requested to stop, drain remaining mailbox
          if result.stop
            # The ActorEntity#process_next already set status to :stopped,
            # but we need to drain remaining messages to dead_letters
            remaining = actor.stop!
            @dead_letters.concat(remaining)
          end

          true
        rescue BehaviorError => e
          # The behavior function threw an exception. The message is
          # dead-lettered, the actor continues processing next messages.
          @dead_letters.push(e.failed_message)
          true
        end
      end

      # Process all actors in round-robin until no work remains.
      #
      # "Round-robin" means we cycle through all actors, giving each
      # one a chance to process one message before moving to the next.
      # This prevents a single busy actor from starving others.
      #
      # The loop exits when no actor has any messages to process.
      #
      # @return [Hash] Statistics: messages_processed, actors_created.
      def run_until_idle
        stats = {messages_processed: 0, actors_created: 0}
        initial_actor_count = @actors.size

        loop do
          work_done = false

          # Snapshot the actor IDs to iterate over. New actors created
          # during this iteration will be picked up in the next pass.
          actor_ids_snapshot = @actors.keys.dup

          actor_ids_snapshot.each do |aid|
            actor = @actors[aid]
            next unless actor
            next if actor.status == :stopped
            next if actor.mailbox_empty?

            if process_next(aid)
              work_done = true
              stats[:messages_processed] += 1
            end
          end

          break unless work_done
        end

        stats[:actors_created] = @actors.size - initial_actor_count
        stats
      end

      # Process all actors until the system is completely quiet.
      #
      # This is like run_until_idle, but it keeps going even after
      # a quiet period — checking if any new messages were generated.
      # It only stops when run_until_idle returns with zero messages
      # processed (truly nothing left to do).
      #
      # @return [Hash] Statistics: messages_processed, actors_created.
      def run_until_done
        total_stats = {messages_processed: 0, actors_created: 0}

        loop do
          stats = run_until_idle
          total_stats[:messages_processed] += stats[:messages_processed]
          total_stats[:actors_created] += stats[:actors_created]

          break if stats[:messages_processed] == 0
        end

        total_stats
      end

      # ─── Channels ──────────────────────────────────────────────

      # Create and register a new channel.
      #
      # @param channel_id [String] Unique identifier for the channel.
      # @param name [String] Human-readable name.
      # @return [Channel] The newly created channel.
      def create_channel(channel_id, name)
        @channels[channel_id] = Channel.new(channel_id: channel_id, name: name)
      end

      # Retrieve a channel by its ID.
      #
      # @param channel_id [String] The channel to look up.
      # @return [Channel] The channel.
      # @raise [ArgumentError] If the channel does not exist.
      def get_channel(channel_id)
        channel = @channels[channel_id]
        raise ArgumentError, "Channel '#{channel_id}' not found" unless channel
        channel
      end

      # ─── Inspection ────────────────────────────────────────────

      # List all registered actor IDs.
      #
      # @return [Array<String>] All actor IDs in the system.
      def actor_ids
        @actors.keys
      end

      # Get the number of pending messages for an actor.
      #
      # @param actor_id [String] The actor to check.
      # @return [Integer] The mailbox depth.
      def mailbox_size(actor_id)
        actor = @actors[actor_id]
        return 0 unless actor
        actor.mailbox_size
      end

      # Shut down the entire system.
      #
      # Stops all actors and drains their mailboxes to dead_letters.
      #
      def shutdown
        @actors.each_key do |aid|
          stop_actor(aid)
        end
      end
    end
  end
end
