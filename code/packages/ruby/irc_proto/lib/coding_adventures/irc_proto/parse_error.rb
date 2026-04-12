# frozen_string_literal: true

module CodingAdventures
  module IrcProto
    # Raised when a line cannot be parsed as a valid IRC message.
    #
    # The IRC protocol specifies that a message is:
    #
    #   [:prefix] command [params...]
    #
    # where the command must be present.  We raise ParseError for:
    #   - empty lines (nothing to parse)
    #   - lines that contain only a prefix but no command
    class ParseError < StandardError; end
  end
end
