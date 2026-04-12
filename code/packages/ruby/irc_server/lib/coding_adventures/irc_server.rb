# frozen_string_literal: true

# irc_server — IRC server state machine (channels, nicks, command dispatch)
#
# This package is the brain of an IRC server.  It knows nothing about sockets,
# threads, or I/O — it is a *pure state machine* that consumes +Message+ values
# (from +irc_proto+) and produces Arrays of +[conn_id, Message]+ pairs that the
# transport layer should forward to the appropriate connections.
#
# == Architecture overview
#
# An IRC server manages three kinds of mutable state:
#
# 1. *Clients* — each TCP connection is represented by a +Client+ object
#    keyed by a +conn_id+ (an integer the transport layer assigns and owns).
#    A client starts in the *unregistered* state and becomes *registered*
#    once it has supplied both a NICK and a USER command.
#
# 2. *Channels* — a +Channel+ groups a set of registered clients.  The
#    first client to join a channel automatically becomes its *operator*.
#
# 3. *Nick index* — a +Hash+ mapping lowercase nick names to conn_ids,
#    enabling O(1) uniqueness checks and direct-message delivery.
#
# == Public interface
#
# The public interface (+IRCServer+) has exactly three methods:
#
# * +on_connect(conn_id, host)+ — a new TCP connection arrived.
# * +on_message(conn_id, msg)+ — a parsed message arrived from a client.
# * +on_disconnect(conn_id)+ — the TCP connection closed.
#
# Each method returns an Array of +[conn_id, Message]+ pairs.

require "set"
require "coding_adventures/irc_proto"

require_relative "irc_server/version"

module CodingAdventures
  module IrcServer
    # ─── Convenience alias ────────────────────────────────────────────────────
    Message = CodingAdventures::IrcProto::Message

    # ─── Nick validation ──────────────────────────────────────────────────────

    # RFC 1459 §2.3.1 defines which characters may appear in a nickname.
    #
    # A valid nick:
    #   - is 1–9 characters long
    #   - starts with a letter OR one of: [ ] \ ` ^ { | } _
    #   - subsequent characters may additionally include digits and the hyphen
    #
    # We compile the regex once at module load time.
    NICK_RE = /\A[a-zA-Z\[\]\\`^{|}_][a-zA-Z0-9\[\]\\`^{|}_-]{0,8}\z/

    # @param nick [String]
    # @return [Boolean]
    def self.valid_nick?(nick)
      !!(NICK_RE =~ nick)
    end

    # ─── IRC numeric reply constants ──────────────────────────────────────────
    # Named constants make the handler code self-documenting.

    RPL_WELCOME        = "001"
    RPL_YOURHOST       = "002"
    RPL_CREATED        = "003"
    RPL_MYINFO         = "004"
    RPL_LUSERCLIENT    = "251"
    RPL_AWAY           = "301"
    RPL_UNAWAY         = "305"
    RPL_NOWAWAY        = "306"
    RPL_WHOISUSER      = "311"
    RPL_WHOISSERVER    = "312"
    RPL_ENDOFWHOIS     = "318"
    RPL_WHOISCHANNELS  = "319"
    RPL_LISTSTART      = "321"
    RPL_LIST           = "322"
    RPL_LISTEND        = "323"
    RPL_CHANNELMODEIS  = "324"
    RPL_NOTOPIC        = "331"
    RPL_TOPIC          = "332"
    RPL_INVITING       = "341"
    RPL_ENDOFWHO       = "315"
    RPL_WHOREPLY       = "352"
    RPL_NAMREPLY       = "353"
    RPL_ENDOFNAMES     = "366"
    RPL_MOTDSTART      = "375"
    RPL_MOTD           = "372"
    RPL_ENDOFMOTD      = "376"
    RPL_YOUREOPER      = "381"

    ERR_NOSUCHNICK        = "401"
    ERR_NOSUCHCHANNEL     = "403"
    ERR_NOTEXTTOSEND      = "412"
    ERR_UNKNOWNCOMMAND    = "421"
    ERR_NONICKNAMEGIVEN   = "431"
    ERR_ERRONEUSNICKNAME  = "432"
    ERR_NICKNAMEINUSE     = "433"
    ERR_USERNOTINCHANNEL  = "441"
    ERR_NOTONCHANNEL      = "442"
    ERR_NOTREGISTERED     = "451"
    ERR_NEEDMOREPARAMS    = "461"
    ERR_PASSWDMISMATCH    = "464"
    ERR_CHANOPRIVSNEEDED  = "482"

    # ─── State model ──────────────────────────────────────────────────────────

    # All server-side state for one TCP connection.
    #
    # A freshly-connected client is *unregistered*: +registered: false+,
    # +nick: nil+, +username: nil+, +realname: nil+.  The client transitions
    # to *registered* once both +NICK+ and +USER+ have been processed.
    #
    # +channels+ tracks lowercase channel names the client has joined,
    # giving O(1) membership tests and efficient cleanup on disconnect.
    Client = Struct.new(
      :id,           # Integer conn_id (transport-layer assigned)
      :nick,         # String or nil
      :username,     # String or nil
      :realname,     # String or nil
      :hostname,     # String — peer host from transport layer
      :registered,   # Boolean
      :channels,     # Set<String> — lowercase channel names
      :away_message, # String or nil
      :is_oper,      # Boolean
      keyword_init: true
    ) do
      # Return the +nick!user@host+ mask used as a message prefix.
      #
      # This is the standard IRC identity string.  Other clients see this
      # in the prefix of any message we relay on behalf of this client.
      #
      # Example: <tt>"alice!alice@192.168.1.1"</tt>
      def mask
        n = nick || "*"
        u = username || "*"
        "#{n}!#{u}@#{hostname}"
      end
    end

    # Per-membership metadata for a client inside a channel.
    #
    # +is_operator+ — True if this client is a channel operator (@).
    #                 The first member of a newly-created channel gets this.
    # +has_voice+   — True if this client has voice privilege (+v).
    ChannelMember = Struct.new(:client, :is_operator, :has_voice, keyword_init: true)

    # All server-side state for one IRC channel.
    #
    # Channel names are always stored and compared in lowercase.
    #
    # +members+  — Hash of conn_id → ChannelMember
    # +modes+    — Set of single-character mode letters currently active
    # +ban_list+ — Array of ban mask patterns (stored, not enforced in v1)
    Channel = Struct.new(:name, :topic, :members, :modes, :ban_list, keyword_init: true)

    # ─── IRCServer ────────────────────────────────────────────────────────────

    # Pure IRC server state machine.
    #
    # This class contains the complete server state (clients, channels, nick
    # index) and the logic for every IRC command.  It never touches the network
    # — the transport layer calls +on_connect+, +on_message+, and
    # +on_disconnect+, and the server returns Arrays of +[conn_id, Message]+
    # pairs that the transport should deliver.
    #
    # *Concurrency note*: this class is intentionally NOT thread-safe.  If the
    # transport layer is multi-threaded, it must serialize calls to these three
    # methods (e.g., with a Mutex).
    #
    # Usage example:
    #
    #   server = CodingAdventures::IrcServer::IRCServer.new(
    #     server_name: "irc.example.com"
    #   )
    #
    #   responses = server.on_connect(1, "192.168.1.10")
    #   responses = server.on_message(1, IrcProto.parse("NICK alice"))
    #   responses = server.on_message(1, IrcProto.parse("USER alice 0 * :Alice Smith"))
    #   # → responses now contains the 001–376 welcome sequence
    class IRCServer
      # @param server_name   [String]       hostname this server advertises
      # @param version       [String]       software version string
      # @param motd          [Array<String>] Message of the Day lines
      # @param oper_password [String]       password for OPER command
      def initialize(server_name:, version: "1.0", motd: [], oper_password: "")
        @server_name   = server_name
        @version       = version
        @motd          = motd || []
        @oper_password = oper_password

        # All known clients keyed by conn_id (Integer).
        @clients = {}

        # All active channels keyed by lowercase name (including '#').
        @channels = {}

        # Nick → conn_id index.  All nicks stored in lowercase for case-
        # insensitive uniqueness checks.
        @nicks = {}
      end

      # ── Public API ─────────────────────────────────────────────────────────

      # Register a new TCP connection.
      #
      # Creates a +Client+ record for the connection but does not send
      # anything — IRC clients are expected to initiate registration by
      # sending +CAP+, +NICK+, and +USER+.
      #
      # @param conn_id [Integer]
      # @param host    [String]
      # @return [Array] empty array
      def on_connect(conn_id, host)
        @clients[conn_id] = Client.new(
          id:           conn_id,
          nick:         nil,
          username:     nil,
          realname:     nil,
          hostname:     host,
          registered:   false,
          channels:     Set.new,
          away_message: nil,
          is_oper:      false
        )
        []
      end

      # Dispatch an inbound IRC message and return the resulting responses.
      #
      # This is the central dispatch method.  It:
      # 1. Looks up the client record for +conn_id+.
      # 2. Routes the message to the appropriate +handle_*+ method.
      # 3. Returns the Array of +[conn_id, Message]+ pairs to send.
      #
      # A client that has not completed the NICK+USER handshake may only
      # send: +NICK+, +USER+, +CAP+, +QUIT+, and +PASS+.  Any other command
      # gets +451 ERR_NOTREGISTERED+.
      #
      # @param conn_id [Integer]
      # @param msg     [Message]
      # @return [Array<[Integer, Message]>]
      def on_message(conn_id, msg)
        client = @clients[conn_id]
        return [] if client.nil?

        # Commands permitted before registration is complete.
        pre_reg_allowed = %w[NICK USER CAP QUIT PASS].to_set

        # Map command strings to handler methods.
        handlers = {
          "NICK"    => method(:handle_nick),
          "USER"    => method(:handle_user),
          "CAP"     => method(:handle_cap),
          "QUIT"    => method(:handle_quit),
          "PASS"    => method(:handle_pass),
          "JOIN"    => method(:handle_join),
          "PART"    => method(:handle_part),
          "PRIVMSG" => method(:handle_privmsg),
          "NOTICE"  => method(:handle_notice),
          "NAMES"   => method(:handle_names),
          "LIST"    => method(:handle_list),
          "TOPIC"   => method(:handle_topic),
          "KICK"    => method(:handle_kick),
          "INVITE"  => method(:handle_invite),
          "MODE"    => method(:handle_mode),
          "PING"    => method(:handle_ping),
          "PONG"    => method(:handle_pong),
          "AWAY"    => method(:handle_away),
          "WHOIS"   => method(:handle_whois),
          "WHO"     => method(:handle_who),
          "OPER"    => method(:handle_oper)
        }

        # Gate: reject post-registration commands from unregistered clients.
        unless client.registered || pre_reg_allowed.include?(msg.command)
          return [[conn_id,
                   make_msg(ERR_NOTREGISTERED, "*", "You have not registered")]]
        end

        handler = handlers[msg.command]
        if handler.nil?
          return [[conn_id,
                   make_msg(ERR_UNKNOWNCOMMAND,
                            client.nick || "*", msg.command,
                            "Unknown command")]]
        end

        handler.call(client, msg)
      end

      # Clean up state after a TCP connection closes.
      #
      # 1. Broadcast QUIT to all channel members who shared a channel.
      # 2. Remove the client from every channel; destroy empty channels.
      # 3. Remove the nick from the nick index.
      # 4. Remove the client record.
      #
      # @param conn_id [Integer]
      # @return [Array<[Integer, Message]>]
      def on_disconnect(conn_id)
        client = @clients[conn_id]
        return [] if client.nil?

        responses = []

        # Broadcast QUIT to all unique channel peers (excluding the quitter).
        if client.registered && client.nick
          quit_msg = Message.new(
            prefix:  client.mask,
            command: "QUIT",
            params:  ["Connection closed"]
          )
          unique_channel_peers(client).each do |peer_id|
            responses << [peer_id, quit_msg]
          end
        end

        # Remove from every channel; destroy empty ones.
        client.channels.each do |chan_name|
          channel = @channels[chan_name]
          next unless channel

          channel.members.delete(conn_id)
          @channels.delete(chan_name) if channel.members.empty?
        end

        # Remove from nick index.
        @nicks.delete(client.nick.downcase) if client.nick

        # Remove client record.
        @clients.delete(conn_id)

        responses
      end

      private

      # ── Helpers ───────────────────────────────────────────────────────────

      # Build a Message whose prefix is the server name.
      #
      # Strips leading colons from params (the colon is a wire-format artefact;
      # +irc_proto.serialize+ adds it back automatically for trailing params).
      def make_msg(command, *params)
        cleaned = params.map { |p| p.start_with?(":") ? p[1..] : p }
        Message.new(prefix: @server_name, command: command, params: cleaned)
      end

      # Build a server message addressed to +client+'s nick.
      #
      # Like +make_msg+ but automatically inserts the client's nick (or +*+
      # if not yet set) as the first parameter.
      def client_msg(client, command, *params)
        nick = client.nick || "*"
        make_msg(command, nick, *params)
      end

      # Return the set of conn_ids that share at least one channel with +client+.
      # The client itself is excluded from the returned set.
      def unique_channel_peers(client)
        peers = Set.new
        client.channels.each do |chan_name|
          channel = @channels[chan_name]
          next unless channel

          channel.members.each_key do |cid|
            peers.add(cid) unless cid == client.id
          end
        end
        peers
      end

      # Build 353 (NAMREPLY) + 366 (ENDOFNAMES) responses for +channel+.
      def names_replies(channel, requesting_nick)
        conn_id = @nicks[requesting_nick.downcase]
        return [] if conn_id.nil?

        # Build the space-separated list of prefixed nicks.
        parts = channel.members.values.map do |m|
          if m.is_operator
            "@#{m.client.nick}"
          elsif m.has_voice
            "+#{m.client.nick}"
          else
            m.client.nick.to_s
          end
        end

        [
          [conn_id, make_msg(RPL_NAMREPLY, requesting_nick, "=",
                             channel.name, parts.join(" "))],
          [conn_id, make_msg(RPL_ENDOFNAMES, requesting_nick,
                             channel.name, "End of /NAMES list")]
        ]
      end

      # Send the RFC 1459 welcome sequence to a newly-registered client.
      def welcome(client)
        nick = client.nick || "*"
        host = @server_name
        ver  = @version
        user_count = @clients.values.count(&:registered)

        responses = [
          [client.id, client_msg(client, RPL_WELCOME,
                                 "Welcome to the IRC Network, #{client.mask}")],
          [client.id, client_msg(client, RPL_YOURHOST,
                                 "Your host is #{host}, running version #{ver}")],
          [client.id, client_msg(client, RPL_CREATED,
                                 "This server was created today")],
          [client.id, make_msg(RPL_MYINFO, nick, host, ver, "o", "o")],
          [client.id, client_msg(client, RPL_LUSERCLIENT,
                                 "There are #{user_count} users on 1 server")],
          [client.id, client_msg(client, RPL_MOTDSTART,
                                 "- #{host} Message of the Day -")]
        ]

        @motd.each do |line|
          responses << [client.id, client_msg(client, RPL_MOTD, "- #{line} -")]
        end

        responses << [client.id, client_msg(client, RPL_ENDOFMOTD,
                                            "End of /MOTD command.")]
        responses
      end

      # ── Command handlers ─────────────────────────────────────────────────
      # Each handler receives (client, msg) and returns Array<[conn_id, Message]>.

      def handle_cap(client, _msg)
        # Modern IRC clients send CAP LS to discover capabilities before
        # NICK/USER.  We acknowledge all CAP requests with an empty ACK so
        # clients that wait for a response don't hang.
        [[client.id,
          Message.new(prefix: @server_name, command: "CAP",
                      params: ["*", "ACK", ""])]]
      end

      def handle_pass(_client, _msg)
        # Silently accept PASS (no connection passwords in v1).
        []
      end

      def handle_nick(client, msg)
        # Validate params: we need at least one (the desired nick).
        if msg.params.empty?
          return [[client.id,
                   make_msg(ERR_NONICKNAMEGIVEN, client.nick || "*",
                            "No nickname given")]]
        end

        new_nick = msg.params[0]

        # Validate the nick against RFC 1459 character rules.
        unless IrcServer.valid_nick?(new_nick)
          return [[client.id,
                   make_msg(ERR_ERRONEUSNICKNAME, client.nick || "*",
                            new_nick, "Erroneous nickname")]]
        end

        # Reject if nick is already in use (case-insensitive).
        if @nicks.key?(new_nick.downcase) && @nicks[new_nick.downcase] != client.id
          return [[client.id,
                   make_msg(ERR_NICKNAMEINUSE, client.nick || "*",
                            new_nick, "Nickname is already in use")]]
        end

        old_nick = client.nick

        # Update the nick index: remove old entry, add new one.
        @nicks.delete(old_nick.downcase) if old_nick
        @nicks[new_nick.downcase] = client.id

        # Store the canonical (mixed-case) nick on the client record.
        client.nick = new_nick

        # If still unregistered, try to complete registration.
        if !client.registered && client.username
          client.registered = true
          return welcome(client)
        end

        # If already registered, broadcast the nick change to channel peers.
        if client.registered && old_nick && old_nick != new_nick
          nick_msg = Message.new(
            prefix:  "#{old_nick}!#{client.username}@#{client.hostname}",
            command: "NICK",
            params:  [new_nick]
          )
          peers = unique_channel_peers(client)
          responses = peers.map { |pid| [pid, nick_msg] }
          responses << [client.id, nick_msg]
          return responses
        end

        []
      end

      def handle_user(client, msg)
        # USER requires at least 4 params: username mode unused realname
        if msg.params.length < 4
          return [[client.id,
                   client_msg(client, ERR_NEEDMOREPARAMS,
                              "USER", "Not enough parameters")]]
        end

        # Ignore USER if already registered (re-registration not allowed).
        return [] if client.registered

        client.username = msg.params[0]
        client.realname = msg.params[3]

        # If we already have a nick, complete registration.
        if client.nick
          client.registered = true
          return welcome(client)
        end

        []
      end

      def handle_quit(client, msg)
        reason = msg.params.first || "Quit"
        quit_msg = Message.new(
          prefix:  client.mask,
          command: "QUIT",
          params:  [reason]
        )

        responses = []

        # Broadcast the quit to all channel peers.
        unique_channel_peers(client).each do |peer_id|
          responses << [peer_id, quit_msg]
        end

        # Inform the client itself (ERROR message then disconnect).
        responses << [client.id,
                      make_msg("ERROR", "Closing Link: #{client.hostname} (#{reason})")]

        responses
      end

      def handle_join(client, msg)
        if msg.params.empty?
          return [[client.id,
                   client_msg(client, ERR_NEEDMOREPARAMS,
                              "JOIN", "Not enough parameters")]]
        end

        # Clients may join multiple channels separated by commas.
        channel_names = msg.params[0].split(",")
        responses = []

        channel_names.each do |chan_name|
          key = chan_name.downcase

          # Create the channel if it doesn't exist.
          channel = @channels[key] ||= Channel.new(
            name:     chan_name,
            topic:    nil,
            members:  {},
            modes:    Set.new,
            ban_list: []
          )

          # Skip if already in the channel.
          next if channel.members.key?(client.id)

          # The first member automatically becomes channel operator.
          first_member = channel.members.empty?
          channel.members[client.id] = ChannelMember.new(
            client:      client,
            is_operator: first_member,
            has_voice:   false
          )
          client.channels.add(key)

          # Broadcast JOIN to all members (including the joiner).
          join_msg = Message.new(
            prefix:  client.mask,
            command: "JOIN",
            params:  [chan_name]
          )
          channel.members.each_key do |cid|
            responses << [cid, join_msg]
          end

          # Send TOPIC (or RPL_NOTOPIC) to the joiner.
          if channel.topic
            responses << [client.id,
                          make_msg(RPL_TOPIC, client.nick, chan_name, channel.topic)]
          else
            responses << [client.id,
                          make_msg(RPL_NOTOPIC, client.nick, chan_name,
                                   "No topic is set")]
          end

          # Send NAMES reply.
          responses.concat(names_replies(channel, client.nick))
        end

        responses
      end

      def handle_part(client, msg)
        if msg.params.empty?
          return [[client.id,
                   client_msg(client, ERR_NEEDMOREPARAMS,
                              "PART", "Not enough parameters")]]
        end

        channel_names = msg.params[0].split(",")
        reason = msg.params[1] || client.nick
        responses = []

        channel_names.each do |chan_name|
          key = chan_name.downcase
          channel = @channels[key]

          unless channel&.members&.key?(client.id)
            responses << [client.id,
                          make_msg(ERR_NOTONCHANNEL, client.nick, chan_name,
                                   "You're not on that channel")]
            next
          end

          part_msg = Message.new(
            prefix:  client.mask,
            command: "PART",
            params:  [chan_name, reason]
          )

          # Broadcast PART to all members including the departing client.
          channel.members.each_key do |cid|
            responses << [cid, part_msg]
          end

          channel.members.delete(client.id)
          client.channels.delete(key)
          @channels.delete(key) if channel.members.empty?
        end

        responses
      end

      def handle_privmsg(client, msg)
        handle_message_command(client, msg, "PRIVMSG")
      end

      def handle_notice(client, msg)
        handle_message_command(client, msg, "NOTICE")
      end

      # Shared logic for PRIVMSG and NOTICE.
      #
      # Target can be a nick (direct message) or a channel (#name).
      def handle_message_command(client, msg, command)
        if msg.params.length < 2
          return [[client.id,
                   client_msg(client, ERR_NEEDMOREPARAMS,
                              command, "Not enough parameters")]]
        end

        target  = msg.params[0]
        text    = msg.params[1]

        if text.empty?
          return [[client.id,
                   client_msg(client, ERR_NOTEXTTOSEND, "No text to send")]]
        end

        relay = Message.new(
          prefix:  client.mask,
          command: command,
          params:  [target, text]
        )

        # Channel message: relay to all members except the sender.
        if target.start_with?("#", "&")
          key     = target.downcase
          channel = @channels[key]
          unless channel
            return [[client.id,
                     make_msg(ERR_NOSUCHCHANNEL, client.nick, target,
                              "No such channel")]]
          end

          responses = []
          channel.members.each_key do |cid|
            responses << [cid, relay] unless cid == client.id
          end
          return responses
        end

        # Direct message to a nick.
        target_id = @nicks[target.downcase]
        unless target_id
          return [[client.id,
                   make_msg(ERR_NOSUCHNICK, client.nick, target,
                            "No such nick/channel")]]
        end

        [[target_id, relay]]
      end

      def handle_names(client, msg)
        responses = []

        if msg.params.empty?
          # List names for all channels.
          @channels.each_value do |channel|
            responses.concat(names_replies(channel, client.nick))
          end
        else
          chan_name = msg.params[0]
          channel = @channels[chan_name.downcase]
          if channel
            responses.concat(names_replies(channel, client.nick))
          else
            responses << [client.id,
                          make_msg(ERR_NOSUCHCHANNEL, client.nick, chan_name,
                                   "No such channel")]
          end
        end

        responses
      end

      def handle_list(client, _msg)
        responses = [[client.id,
                      make_msg(RPL_LISTSTART, client.nick, "Channel", "Users  Name")]]

        @channels.each_value do |channel|
          responses << [client.id,
                        make_msg(RPL_LIST, client.nick, channel.name,
                                 channel.members.size.to_s,
                                 channel.topic || "")]
        end

        responses << [client.id,
                      make_msg(RPL_LISTEND, client.nick, "End of /LIST")]
        responses
      end

      def handle_topic(client, msg)
        if msg.params.empty?
          return [[client.id,
                   client_msg(client, ERR_NEEDMOREPARAMS,
                              "TOPIC", "Not enough parameters")]]
        end

        chan_name = msg.params[0]
        key = chan_name.downcase
        channel = @channels[key]

        unless channel
          return [[client.id,
                   make_msg(ERR_NOSUCHCHANNEL, client.nick, chan_name,
                            "No such channel")]]
        end

        unless channel.members.key?(client.id)
          return [[client.id,
                   make_msg(ERR_NOTONCHANNEL, client.nick, chan_name,
                            "You're not on that channel")]]
        end

        # If no new topic provided, return the current topic.
        if msg.params.length == 1
          if channel.topic
            return [[client.id,
                     make_msg(RPL_TOPIC, client.nick, chan_name, channel.topic)]]
          else
            return [[client.id,
                     make_msg(RPL_NOTOPIC, client.nick, chan_name, "No topic is set")]]
          end
        end

        # Check channel operator status before allowing topic change.
        member = channel.members[client.id]
        unless member&.is_operator || client.is_oper
          return [[client.id,
                   make_msg(ERR_CHANOPRIVSNEEDED, client.nick, chan_name,
                            "You're not channel operator")]]
        end

        # Set the new topic.
        channel.topic = msg.params[1]
        topic_msg = Message.new(
          prefix:  client.mask,
          command: "TOPIC",
          params:  [chan_name, channel.topic]
        )

        channel.members.each_key.map { |cid| [cid, topic_msg] }
      end

      def handle_kick(client, msg)
        if msg.params.length < 2
          return [[client.id,
                   client_msg(client, ERR_NEEDMOREPARAMS,
                              "KICK", "Not enough parameters")]]
        end

        chan_name  = msg.params[0]
        target_nick = msg.params[1]
        reason      = msg.params[2] || client.nick
        key         = chan_name.downcase
        channel     = @channels[key]

        unless channel
          return [[client.id,
                   make_msg(ERR_NOSUCHCHANNEL, client.nick, chan_name,
                            "No such channel")]]
        end

        member = channel.members[client.id]
        unless member&.is_operator || client.is_oper
          return [[client.id,
                   make_msg(ERR_CHANOPRIVSNEEDED, client.nick, chan_name,
                            "You're not channel operator")]]
        end

        target_id = @nicks[target_nick.downcase]
        unless target_id && channel.members.key?(target_id)
          return [[client.id,
                   make_msg(ERR_USERNOTINCHANNEL, client.nick, target_nick,
                            chan_name, "They aren't on that channel")]]
        end

        kick_msg = Message.new(
          prefix:  client.mask,
          command: "KICK",
          params:  [chan_name, target_nick, reason]
        )

        responses = channel.members.each_key.map { |cid| [cid, kick_msg] }

        target_client = @clients[target_id]
        if target_client
          channel.members.delete(target_id)
          target_client.channels.delete(key)
          @channels.delete(key) if channel.members.empty?
        end

        responses
      end

      def handle_invite(client, msg)
        if msg.params.length < 2
          return [[client.id,
                   client_msg(client, ERR_NEEDMOREPARAMS,
                              "INVITE", "Not enough parameters")]]
        end

        target_nick = msg.params[0]
        chan_name   = msg.params[1]
        target_id   = @nicks[target_nick.downcase]

        unless target_id
          return [[client.id,
                   make_msg(ERR_NOSUCHNICK, client.nick, target_nick,
                            "No such nick")]]
        end

        invite_msg = Message.new(
          prefix:  client.mask,
          command: "INVITE",
          params:  [target_nick, chan_name]
        )

        [
          [client.id,
           make_msg(RPL_INVITING, client.nick, target_nick, chan_name)],
          [target_id, invite_msg]
        ]
      end

      def handle_mode(client, msg)
        if msg.params.empty?
          return [[client.id,
                   client_msg(client, ERR_NEEDMOREPARAMS,
                              "MODE", "Not enough parameters")]]
        end

        target = msg.params[0]

        # Channel mode.
        if target.start_with?("#", "&")
          key     = target.downcase
          channel = @channels[key]

          unless channel
            return [[client.id,
                     make_msg(ERR_NOSUCHCHANNEL, client.nick, target,
                              "No such channel")]]
          end

          # Query current modes.
          if msg.params.length == 1
            mode_str = channel.modes.empty? ? "+" : "+#{channel.modes.to_a.join}"
            return [[client.id,
                     make_msg(RPL_CHANNELMODEIS, client.nick, target, mode_str)]]
          end

          mode_str = msg.params[1]
          param_idx = 2

          mode_str.chars.each do |ch|
            case ch
            when "+"
              nil
            when "-"
              nil
            when "o"
              # +o/-o: grant/revoke channel operator
              target_nick = msg.params[param_idx]
              param_idx  += 1
              next unless target_nick

              target_id = @nicks[target_nick.downcase]
              next unless target_id && channel.members.key?(target_id)

              adding = !mode_str.start_with?("-")
              channel.members[target_id].is_operator = adding
            when "v"
              target_nick = msg.params[param_idx]
              param_idx  += 1
              next unless target_nick

              target_id = @nicks[target_nick.downcase]
              next unless target_id && channel.members.key?(target_id)

              adding = !mode_str.start_with?("-")
              channel.members[target_id].has_voice = adding
            when "b"
              # Ban: store the mask.
              ban_mask = msg.params[param_idx]
              param_idx += 1
              channel.ban_list << ban_mask if ban_mask
            else
              # Simple toggle mode (e.g. +m moderated, +s secret).
              if mode_str.start_with?("-")
                channel.modes.delete(ch)
              else
                channel.modes.add(ch)
              end
            end
          end

          [[client.id, make_msg("MODE", target, mode_str)]]
        else
          # User mode — minimal support; just ACK.
          [[client.id, make_msg("MODE", target, msg.params[1] || "+i")]]
        end
      end

      def handle_ping(client, msg)
        token = msg.params.first || @server_name
        [[client.id,
          Message.new(prefix: @server_name, command: "PONG",
                      params: [@server_name, token])]]
      end

      def handle_pong(_client, _msg)
        # PONG is a keep-alive reply; nothing to do.
        []
      end

      def handle_away(client, msg)
        if msg.params.empty? || msg.params[0].empty?
          client.away_message = nil
          [[client.id, client_msg(client, RPL_UNAWAY, "You are no longer marked as being away")]]
        else
          client.away_message = msg.params[0]
          [[client.id, client_msg(client, RPL_NOWAWAY, "You have been marked as being away")]]
        end
      end

      def handle_whois(client, msg)
        if msg.params.empty?
          return [[client.id,
                   client_msg(client, ERR_NEEDMOREPARAMS,
                              "WHOIS", "Not enough parameters")]]
        end

        target_nick = msg.params[0]
        target_id   = @nicks[target_nick.downcase]

        unless target_id
          return [[client.id,
                   make_msg(ERR_NOSUCHNICK, client.nick, target_nick,
                            "No such nick/channel")]]
        end

        target = @clients[target_id]
        responses = [
          [client.id, make_msg(RPL_WHOISUSER, client.nick, target.nick,
                               target.username || "*", target.hostname, "*",
                               target.realname || "")],
          [client.id, make_msg(RPL_WHOISSERVER, client.nick, target.nick,
                               @server_name, "Ruby IRC Server")]
        ]

        unless target.channels.empty?
          chan_list = target.channels.map do |cn|
            ch = @channels[cn]
            next unless ch

            mem = ch.members[target_id]
            mem&.is_operator ? "@#{ch.name}" : ch.name
          end.compact.join(" ")

          responses << [client.id,
                        make_msg(RPL_WHOISCHANNELS, client.nick, target.nick, chan_list)]
        end

        responses << [client.id,
                      make_msg(RPL_ENDOFWHOIS, client.nick, target.nick,
                               "End of /WHOIS list")]
        responses
      end

      def handle_who(client, msg)
        mask = msg.params.first || "*"
        nick = client.nick

        who_row = lambda do |c|
          ch_name = c.channels.first ? @channels[c.channels.first]&.name || "*" : "*"
          [client.id, make_msg(RPL_WHOREPLY, nick, ch_name,
                               c.username || "*", c.hostname,
                               @server_name, c.nick,
                               "H", "0 #{c.realname || c.nick}")]
        end

        responses = []

        if mask.start_with?("#", "&")
          key     = mask.downcase
          channel = @channels[key]
          if channel
            channel.members.each_value do |m|
              responses << who_row.call(m.client)
            end
          end
        else
          @clients.each_value do |c|
            responses << who_row.call(c) if c.registered
          end
        end

        responses << [client.id,
                      make_msg(RPL_ENDOFWHO, nick, mask, "End of /WHO list")]
        responses
      end

      def handle_oper(client, msg)
        if msg.params.length < 2
          return [[client.id,
                   client_msg(client, ERR_NEEDMOREPARAMS,
                              "OPER", "Not enough parameters")]]
        end

        password = msg.params[1]

        if !@oper_password.empty? && password == @oper_password
          client.is_oper = true
          [[client.id, client_msg(client, RPL_YOUREOPER,
                                  "You are now an IRC operator")]]
        else
          [[client.id, client_msg(client, ERR_PASSWDMISMATCH,
                                  "Password incorrect")]]
        end
      end
    end
  end
end
