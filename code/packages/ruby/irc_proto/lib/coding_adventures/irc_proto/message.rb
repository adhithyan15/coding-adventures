# frozen_string_literal: true

module CodingAdventures
  module IrcProto
    # Represents a single parsed IRC message.
    #
    # An IRC message has three parts (RFC 1459 §2.3):
    #
    #   [:prefix] command [param param ... [:trailing]]
    #
    # == Fields
    #
    # +prefix+  — Optional origin identifier.  For server-generated messages
    #             this is the server name (e.g. "irc.example.com").  For
    #             client-relayed messages it is the nick!user@host mask.
    #             +nil+ if absent.
    #
    # +command+ — The IRC verb, always stored as an upper-case String
    #             (e.g. "NICK", "001", "PRIVMSG").
    #
    # +params+  — An Array<String> of parameters, trailing colon stripped.
    #             RFC 1459 allows at most 15 params; we don't enforce the
    #             limit on serialisation but do honour it on parsing.
    class Message
      attr_reader :prefix, :command, :params

      # @param prefix  [String, nil]
      # @param command [String]
      # @param params  [Array<String>]
      def initialize(prefix: nil, command:, params: [])
        @prefix  = prefix
        @command = command
        @params  = Array(params)
      end

      # Two messages are equal if all three fields are equal.
      def ==(other)
        other.is_a?(Message) &&
          prefix  == other.prefix  &&
          command == other.command &&
          params  == other.params
      end

      # Human-readable representation for debugging.
      def inspect
        "#<Message prefix=#{prefix.inspect} command=#{command.inspect}" \
          " params=#{params.inspect}>"
      end
    end
  end
end
