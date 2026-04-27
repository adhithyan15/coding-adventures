# frozen_string_literal: true

# ircd — wiring layer: connects irc_net_stdlib → irc_framing → irc_server
#
# == Architecture
#
# This file contains:
#   1. +DriverHandler+  — implements the +Handler+ protocol expected by
#                         +StdlibEventLoop+.  It owns per-connection framers
#                         and routes data through the IRC state machine.
#   2. +Config+         — plain struct holding all runtime configuration.
#   3. +parse_args+     — command-line argument parser (OptionParser).
#   4. +main+           — entry point that wires all components together.
#
# == Data flow
#
#   Network bytes
#     │  StdlibEventLoop calls on_data(conn_id, raw_bytes)
#     ▼
#   DriverHandler#on_data
#     │  feeds bytes into per-connection Framer
#     ▼
#   Framer#frames   → Array of complete IRC lines (CRLF stripped)
#     │
#     ▼
#   IrcProto.parse  → Message object
#     │
#     ▼
#   IRCServer#on_message → Array of [conn_id, Message] responses
#     │
#     ▼
#   DriverHandler serialises each response and calls loop.send_to(conn_id, wire)

require "optparse"
require "coding_adventures/irc_proto"
require "coding_adventures/irc_framing"
require "coding_adventures/irc_server"
require "coding_adventures/irc_net_stdlib"

# ── DriverHandler ────────────────────────────────────────────────────────────

# Bridges the network layer (+StdlibEventLoop+) to the IRC logic layer
# (+IRCServer+).
#
# For each connection it maintains a +Framer+ (from +irc_framing+) so that
# TCP byte-stream fragments are reassembled into complete IRC lines before
# dispatch.
#
# The +loop+ object (a +StdlibEventLoop+) is used to write response data back
# to the appropriate connections.
class DriverHandler
  include CodingAdventures::IrcNetStdlib::Handler

  # @param server [CodingAdventures::IrcServer::IRCServer]
  # @param loop   [CodingAdventures::IrcNetStdlib::StdlibEventLoop]
  def initialize(server, loop)
    @server  = server
    @loop    = loop
    @framers = {}  # conn_id → Framer
  end

  # Called when a new TCP connection arrives.
  #
  # Creates a +Framer+ for the connection and delegates to the IRC server.
  # Any immediate responses (none for IRC) are sent back.
  #
  # @param conn_id [Integer]
  # @param host    [String]
  def on_connect(conn_id, host)
    @framers[conn_id] = CodingAdventures::IrcFraming::Framer.new
    responses = @server.on_connect(conn_id, host)
    send_responses(responses)
  end

  # Called when raw bytes arrive from the network.
  #
  # Feeds the bytes into the connection's framer, then processes each
  # complete IRC line:
  #   1. Parse into a +Message+ (skip malformed lines gracefully).
  #   2. Dispatch to the IRC server.
  #   3. Send all response messages back via the event loop.
  #
  # @param conn_id [Integer]
  # @param data    [String] raw bytes from the socket
  def on_data(conn_id, data)
    framer = @framers[conn_id]
    return unless framer  # connection not yet registered — ignore

    framer.feed(data)

    framer.frames.each do |line|
      begin
        msg = CodingAdventures::IrcProto.parse(line)
      rescue CodingAdventures::IrcProto::ParseError
        # Silently skip malformed lines (empty line, prefix-only, etc.)
        next
      end

      responses = @server.on_message(conn_id, msg)
      send_responses(responses)
    end
  end

  # Called when a TCP connection closes.
  #
  # Removes the framer and delegates cleanup to the IRC server.
  # Any quit-broadcast messages are sent.
  #
  # @param conn_id [Integer]
  def on_disconnect(conn_id)
    @framers.delete(conn_id)
    responses = @server.on_disconnect(conn_id)
    send_responses(responses)
  end

  private

  # Serialise each [conn_id, Message] pair and write it to the network.
  #
  # Appends "\r\n" (the IRC message terminator) after the serialised line.
  def send_responses(responses)
    responses.each do |conn_id, msg|
      wire = CodingAdventures::IrcProto.serialize(msg) + "\r\n"
      @loop.send_to(conn_id, wire)
    end
  end
end

# ── Config ────────────────────────────────────────────────────────────────────

# Runtime configuration for the ircd program.
#
# All fields have sensible defaults so the server works out of the box
# without any command-line flags.
Config = Struct.new(
  :host,          # String  — bind address
  :port,          # Integer — TCP port
  :server_name,   # String  — advertised server hostname
  :motd,          # Array<String> — Message of the Day lines
  :oper_password, # String  — OPER command password (empty = disabled)
  keyword_init: true
) do
  # Return a Config with sensible production defaults.
  def self.default
    new(
      host:          "0.0.0.0",
      port:          6667,
      server_name:   "irc.local",
      motd:          ["Welcome."],
      oper_password: ""
    )
  end
end

# ── parse_args ────────────────────────────────────────────────────────────────

# Parse command-line arguments into a +Config+ struct.
#
# Supported flags:
#   --host         Bind address (default: 0.0.0.0)
#   --port         TCP port (default: 6667)
#   --server-name  Advertised server hostname (default: irc.local)
#   --motd         Single MOTD line (default: Welcome.)
#   --oper-password OPER command password (default: empty)
#
# @param argv [Array<String>]
# @return [Config]
def parse_args(argv)
  config = Config.default

  OptionParser.new do |opts|
    opts.banner = "Usage: ircd [options]"

    opts.on("--host HOST", "Bind address (default: 0.0.0.0)") do |v|
      config.host = v
    end

    opts.on("--port PORT", Integer, "TCP port (default: 6667)") do |v|
      config.port = v
    end

    opts.on("--server-name NAME", "Server hostname (default: irc.local)") do |v|
      config.server_name = v
    end

    opts.on("--motd LINE", "MOTD line (default: Welcome.)") do |v|
      config.motd = [v]
    end

    opts.on("--oper-password PASS", "OPER password") do |v|
      config.oper_password = v
    end
  end.parse!(argv)

  config
end

# ── main ──────────────────────────────────────────────────────────────────────

# Wire all components together and start the server.
#
# This is the entry point called from +bin/ircd+.
#
# @param argv [Array<String>, nil]  command-line arguments (defaults to ARGV)
def main(argv = nil)
  argv ||= ARGV.dup
  config = parse_args(argv)

  server = CodingAdventures::IrcServer::IRCServer.new(
    server_name:   config.server_name,
    motd:          config.motd,
    oper_password: config.oper_password
  )

  event_loop = CodingAdventures::IrcNetStdlib::StdlibEventLoop.new
  handler    = DriverHandler.new(server, event_loop)

  # Graceful shutdown on SIGINT (Ctrl-C) and SIGTERM (kill).
  Signal.trap("INT")  { event_loop.stop }
  Signal.trap("TERM") { event_loop.stop }

  $stderr.puts "ircd starting on #{config.host}:#{config.port} " \
               "(server: #{config.server_name})"

  event_loop.run(config.host, config.port, handler)
end
