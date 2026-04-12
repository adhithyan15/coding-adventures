# frozen_string_literal: true

# irc_proto — pure IRC message parsing and serialisation (zero I/O)
#
# == Overview
#
# This package handles one job: converting between IRC wire-format strings
# and +Message+ objects.  It has no knowledge of sockets, threads, channels,
# or server state.
#
# == Wire format (RFC 1459 §2.3)
#
#   message    =  [ ":" prefix SPACE ] command [ params ] crlf
#   prefix     =  servername / ( nickname [ [ "!" user ] "@" host ] )
#   command    =  1*letter / 3digit
#   params     =  *14( SPACE middle ) [ SPACE ":" trailing ]
#                 =/ 14( SPACE middle ) [ SPACE [ ":" ] trailing ]
#   middle     =  nospcrlfcl *( ":" / nospcrlfcl )
#   trailing   =  *( ":" / " " / nospcrlfcl )
#
# == Parsing rules we implement
#
#  1. Strip the leading ":"  from the prefix to get the raw prefix string.
#  2. Normalise the command to upper-case for uniform dispatch.
#  3. Collect up to 15 params; the last one starting with ":" is the
#     *trailing* param and may contain spaces.  The leading ":" is stripped.
#  4. Raise +ParseError+ if the line is empty or has no command.
#
# == Serialisation rules we implement
#
#  1. Emit ":" + prefix + " " if prefix is present.
#  2. Emit command.
#  3. Emit each param separated by " ".  If the *last* param contains a
#     space (or is empty), prefix it with ":".

require_relative "irc_proto/version"
require_relative "irc_proto/parse_error"
require_relative "irc_proto/message"

module CodingAdventures
  module IrcProto
    # Parse a single IRC line (CRLF already stripped by the framing layer).
    #
    # @param line [String] a complete IRC message line, without CRLF
    # @return [Message]
    # @raise [ParseError] if the line is empty or has no command
    def self.parse(line)
      raise ParseError, "Empty line" if line.nil? || line.strip.empty?

      rest = line

      # ── Optional prefix ───────────────────────────────────────────────────
      # A prefix starts with a literal ":" at position 0.
      prefix = nil
      if rest.start_with?(":")
        space_idx = rest.index(" ")
        raise ParseError, "Prefix with no command: #{line.inspect}" if space_idx.nil?

        prefix = rest[1...space_idx]          # strip the leading ":"
        rest   = rest[(space_idx + 1)..]      # everything after the space
      end

      # ── Command ───────────────────────────────────────────────────────────
      # The command is the first whitespace-delimited token.
      parts   = rest.split(" ", 2)
      raise ParseError, "No command in line: #{line.inspect}" if parts.empty? || parts[0].empty?

      command = parts[0].upcase
      rest    = parts[1] || ""

      # ── Parameters ───────────────────────────────────────────────────────
      # We consume tokens until:
      #   - we've collected 14 non-trailing params, OR
      #   - the remaining string starts with ":"  (trailing param)
      #   - the remaining string is empty
      params = []

      until rest.empty?
        if rest.start_with?(":")
          # Trailing param: everything after the ":", spaces included.
          params << rest[1..]
          rest = ""
        else
          token_parts = rest.split(" ", 2)
          params << token_parts[0]
          rest = token_parts[1] || ""

          # RFC 1459 allows at most 15 params (14 middle + 1 trailing).
          # Once we have 14, treat the rest as a single trailing param.
          if params.length == 14 && !rest.empty?
            # Consume the rest as trailing (with or without leading ":").
            params << (rest.start_with?(":") ? rest[1..] : rest)
            rest = ""
          end
        end
      end

      Message.new(prefix: prefix, command: command, params: params)
    end

    # Serialise a +Message+ to an IRC wire-format line (without CRLF).
    #
    # The framing layer is responsible for appending "\r\n" before sending.
    #
    # @param msg [Message]
    # @return [String]
    def self.serialize(msg)
      parts = []

      # Emit ":prefix " if present.
      parts << ":#{msg.prefix}" if msg.prefix

      parts << msg.command

      msg.params.each_with_index do |param, idx|
        last = (idx == msg.params.length - 1)
        if last && (param.include?(" ") || param.empty?)
          parts << ":#{param}"
        else
          parts << param
        end
      end

      parts.join(" ")
    end
  end
end
