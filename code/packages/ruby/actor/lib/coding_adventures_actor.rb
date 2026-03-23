# frozen_string_literal: true

# Entry point for the coding_adventures_actor gem.
#
# This gem implements the Actor model — a mathematical framework for
# concurrent computation invented by Carl Hewitt in 1973. It defines
# computation in terms of **actors** — independent entities that
# communicate exclusively through **messages**. No shared memory.
# No locks. No mutexes.
#
# The package provides three primitives:
#
#   1. Message  — immutable, typed, binary-native, serializable
#   2. Channel  — one-way, append-only, persistent message log
#   3. Actor    — isolated computation with mailbox and behavior function
#
# Plus an ActorSystem that manages lifecycle, routing, and channels.
#
# Usage:
#   require "coding_adventures_actor"
#
#   system = CodingAdventures::Actor::ActorSystem.new
#
#   # Create an echo actor
#   echo = ->(state, msg) {
#     reply = CodingAdventures::Actor::Message.text(
#       sender_id: "echo",
#       payload: "echo: #{msg.payload_text}"
#     )
#     CodingAdventures::Actor::ActorResult.new(
#       new_state: state,
#       messages_to_send: [[msg.sender_id, reply]]
#     )
#   }
#
#   system.create_actor("echo", nil, echo)
#   msg = CodingAdventures::Actor::Message.text(sender_id: "user", payload: "hello")
#   system.send_message("echo", msg)
#   system.process_next("echo")

require_relative "coding_adventures/actor/version"
require_relative "coding_adventures/actor/message"
require_relative "coding_adventures/actor/channel"
require_relative "coding_adventures/actor/actor"
require_relative "coding_adventures/actor/actor_system"
