/**
 * irc-server — IRC server state machine (channels, nicks, command dispatch).
 *
 * This package is the brain of an IRC server.  It knows nothing about sockets,
 * threads, or I/O — it is a *pure state machine* that consumes `Message` values
 * (from `irc-proto`) and produces arrays of `[ConnId, Message]` pairs that the
 * transport layer should forward to the appropriate connections.
 *
 * ## Architecture overview
 *
 * An IRC server manages three kinds of mutable state:
 *
 * 1. **Clients** — each TCP connection is represented by a `Client` object
 *    keyed by a `ConnId` (an integer the transport layer assigns and owns).
 *    A client starts in the *unregistered* state and becomes *registered* once
 *    it has supplied both a NICK and a USER command.  Only registered clients
 *    may join channels or send messages.
 *
 * 2. **Channels** — a `Channel` groups a set of registered clients.  The
 *    first client to join a channel automatically becomes its *operator* and
 *    gains the power to kick members and change the topic.
 *
 * 3. **Nick index** — a `Map<string, ConnId>` mapping lowercase nick names to
 *    connection IDs enables O(1) uniqueness checks and direct-message delivery.
 *
 * ## Public interface
 *
 * The `IRCServer` has exactly three methods that the transport layer calls:
 *
 * * `onConnect(connId, host)` — a new TCP connection arrived.
 * * `onMessage(connId, msg)`  — a parsed message arrived from a client.
 * * `onDisconnect(connId)`    — the TCP connection closed.
 *
 * Each method returns a *list of responses*: `[ConnId, Message]` tuples.  The
 * transport layer iterates this list and sends each message to the given
 * connection.  The server itself never touches sockets.
 *
 * RFC 1459 references:
 * - Commands: https://www.rfc-editor.org/rfc/rfc1459#section-4
 * - Numerics: https://www.rfc-editor.org/rfc/rfc1459#section-6
 */

import { Message } from "@coding-adventures/irc-proto";

// ---------------------------------------------------------------------------
// Core type aliases
// ---------------------------------------------------------------------------

/**
 * Opaque connection identifier.  The transport layer assigns these when a
 * TCP connection opens.  Using a branded type rather than a bare `number`
 * lets TypeScript catch accidental mix-ups between connection IDs and other
 * integers.
 *
 * Branding pattern: `number & { __connId?: true }` is erased at runtime
 * (the `?` means the property is optional, so regular numbers satisfy it at
 * runtime) but is distinct from plain `number` in the type system.
 */
export type ConnId = number & { __connId?: true };

/**
 * A single outbound message destined for a specific connection.
 * The IRCServer never writes to sockets — it only produces these pairs.
 */
export type Response = [ConnId, Message];

// ---------------------------------------------------------------------------
// Nick validation
// ---------------------------------------------------------------------------

// RFC 1459 §2.3.1 defines which characters may appear in a nickname.
//
// A valid nick:
//   - is 1–9 characters long
//   - starts with a letter OR one of: [ ] \ ` ^ { | } _
//   - subsequent characters may additionally include digits and the hyphen
//
// We compile the regex once at module load time for performance.
const NICK_RE = /^[a-zA-Z\[\]\\`^{|}_][a-zA-Z0-9\[\]\\`^{|}_-]{0,8}$/;

function validNick(nick: string): boolean {
  return NICK_RE.test(nick);
}

// ---------------------------------------------------------------------------
// State model
// ---------------------------------------------------------------------------

/**
 * All the server-side state we keep for one TCP connection.
 *
 * A freshly-connected client is *unregistered*: `registered=false`,
 * `nick=null`, `username=null`, `realname=null`.  The client
 * transitions to *registered* once both `NICK` and `USER` have been
 * successfully processed.  Until that point, only `NICK`, `USER`,
 * `CAP`, and `QUIT` are accepted; everything else gets `451 ERR_NOTREGISTERED`.
 */
interface Client {
  /** The transport-layer connection identifier.  Immutable once assigned. */
  readonly id: ConnId;
  /** IRC nickname.  null until the client sends NICK. */
  nick: string | null;
  /** IRC username (from the USER command's first parameter). */
  username: string | null;
  /** Real name / GECOS (from the USER command's trailing parameter). */
  realname: string | null;
  /** Hostname of the connecting peer, supplied by the transport layer on connect. */
  hostname: string;
  /** True once both NICK and USER have been processed successfully. */
  registered: boolean;
  /** Lowercase channel names this client has joined. */
  channels: Set<string>;
  /** Optional away message.  null means the client is not away. */
  awayMessage: string | null;
  /** True if the client has authenticated with OPER. */
  isOper: boolean;
}

function clientMask(client: Client): string {
  // The standard IRC identity string: "nick!user@host"
  const nick = client.nick ?? "*";
  const user = client.username ?? "*";
  return `${nick}!${user}@${client.hostname}`;
}

/**
 * Per-membership metadata for a client inside a channel.
 *
 * A single client may be in many channels simultaneously; each membership
 * is represented by a separate `ChannelMember` instance stored in
 * `Channel.members`.
 */
interface ChannelMember {
  client: Client;
  /** True if this client is a channel operator (@). */
  isOperator: boolean;
  /** True if this client has voice privilege (+v). */
  hasVoice: boolean;
}

/**
 * All server-side state for one IRC channel.
 *
 * Channel names are always stored and compared in lowercase, normalised when
 * the client sends `JOIN`.
 */
interface Channel {
  /** Lowercase channel name including the '#' sigil. */
  name: string;
  /** Human-readable topic string.  Empty string means no topic is set. */
  topic: string;
  /** Active members indexed by ConnId. */
  members: Map<ConnId, ChannelMember>;
  /** Active channel mode flags (single characters). */
  modes: Set<string>;
  /** Ban mask list (stored but not enforced in v1). */
  banList: string[];
}

// ---------------------------------------------------------------------------
// IRC numeric reply constants
// ---------------------------------------------------------------------------
// Named constants make the handler code self-documenting.

const RPL_WELCOME        = "001"; // :Welcome to the IRC Network, nick!user@host
const RPL_YOURHOST       = "002"; // :Your host is <name>, running version <ver>
const RPL_CREATED        = "003"; // :This server was created today
const RPL_MYINFO         = "004"; // <servername> <version> <usermodes> <chanmodes>
const RPL_LUSERCLIENT    = "251"; // :There are N users on 1 server
const RPL_AWAY           = "301"; // <nick> :<away message>
const RPL_UNAWAY         = "305"; // :You are no longer marked as being away
const RPL_NOWAWAY        = "306"; // :You have been marked as being away
const RPL_WHOISUSER      = "311"; // <nick> <user> <host> * :<realname>
const RPL_WHOISSERVER    = "312"; // <nick> <server> :<server info>
const RPL_WHOISCHANNELS  = "319"; // <nick> :{[@|+]channel...}
const RPL_LIST           = "322"; // <channel> <# visible> :<topic>
const RPL_LISTSTART      = "321"; // Channel :Users  Name
const RPL_LISTEND        = "323"; // :End of /LIST
const RPL_CHANNELMODEIS  = "324"; // <channel> <mode> [<mode params>]
const RPL_NOTOPIC        = "331"; // <channel> :No topic is set
const RPL_TOPIC          = "332"; // <channel> :<topic>
const RPL_INVITING       = "341"; // <channel> <nick>
const RPL_WHOREPLY       = "352"; // <channel> <user> <host> <server> <nick> H|G :hops realname
const RPL_NAMREPLY       = "353"; // = <channel> :<prefix>nick...
const RPL_ENDOFNAMES     = "366"; // <channel> :End of /NAMES list
const RPL_ENDOFWHO       = "315"; // <name> :End of /WHO list
const RPL_MOTDSTART      = "375"; // :- <server> Message of the Day -
const RPL_MOTD           = "372"; // :- <text> -
const RPL_ENDOFMOTD      = "376"; // :End of /MOTD command
const RPL_YOUREOPER      = "381"; // :You are now an IRC operator
const RPL_ENDOFWHOIS     = "318"; // <nick> :End of /WHOIS list

const ERR_NOSUCHNICK         = "401"; // <nick/channel> :No such nick/channel
const ERR_NOSUCHCHANNEL      = "403"; // <channel> :No such channel
const ERR_UNKNOWNCOMMAND     = "421"; // <command> :Unknown command
const ERR_NONICKNAMEGIVEN    = "431"; // :No nickname given
const ERR_ERRONEUSNICKNAME   = "432"; // <nick> :Erroneous nickname
const ERR_NICKNAMEINUSE      = "433"; // <nick> :Nickname is already in use
const ERR_USERNOTINCHANNEL   = "441"; // <nick> <channel> :They aren't on that channel
const ERR_NOTONCHANNEL       = "442"; // <channel> :You're not on that channel
const ERR_NOTREGISTERED      = "451"; // :You have not registered
const ERR_NEEDMOREPARAMS     = "461"; // <command> :Not enough parameters
const ERR_PASSWDMISMATCH     = "464"; // :Password incorrect
const ERR_CHANOPRIVSNEEDED   = "482"; // <channel> :You're not channel operator

// ---------------------------------------------------------------------------
// IRCServer
// ---------------------------------------------------------------------------

/**
 * Pure IRC server state machine.
 *
 * This class contains the complete server state (clients, channels, nick
 * index) and the logic for every IRC command.  It never touches the network
 * — the transport layer calls `onConnect`, `onMessage`, and `onDisconnect`,
 * and the server returns arrays of `[ConnId, Message]` pairs that the
 * transport should deliver.
 *
 * **Concurrency note**: this class is intentionally NOT thread-safe.  Node.js
 * is single-threaded so this is fine; if you use worker threads, serialize
 * calls to these three methods.
 *
 * @example
 * ```ts
 * const server = new IRCServer("irc.example.com");
 *
 * // New connection arrives:
 * const r1 = server.onConnect(1 as ConnId, "192.168.1.10");
 *
 * // Client sends "NICK alice":
 * const r2 = server.onMessage(1 as ConnId, parse("NICK alice"));
 *
 * // Client sends "USER alice 0 * :Alice Smith":
 * const r3 = server.onMessage(1 as ConnId, parse("USER alice 0 * :Alice Smith"));
 * // → r3 contains the 001–376 welcome sequence
 *
 * // Later, the TCP connection drops:
 * const r4 = server.onDisconnect(1 as ConnId);
 * // → QUIT is broadcast to all channels alice was in
 * ```
 */
export class IRCServer {
  private readonly serverName: string;
  private readonly version: string;
  private readonly motd: string[];
  private readonly operPassword: string;

  // All known clients keyed by ConnId.
  private clients: Map<ConnId, Client> = new Map();

  // All active channels keyed by lowercase name (including '#').
  private channels: Map<string, Channel> = new Map();

  // Nick → ConnId index.  All nicks stored in lowercase for case-insensitive
  // uniqueness checks.
  private nicks: Map<string, ConnId> = new Map();

  constructor(
    serverName: string,
    motd: string[] = [],
    operPassword: string = "",
    version: string = "1.0"
  ) {
    this.serverName = serverName;
    this.motd = motd;
    this.operPassword = operPassword;
    this.version = version;
  }

  // -----------------------------------------------------------------------
  // Public API
  // -----------------------------------------------------------------------

  /**
   * Register a new TCP connection.
   *
   * Creates a `Client` record for the connection but does not send anything
   * — IRC clients are expected to initiate registration by sending `CAP`,
   * `NICK`, and `USER`.
   *
   * Returns an empty array; no messages are sent until the client speaks.
   */
  onConnect(connId: ConnId, host: string): Response[] {
    const client: Client = {
      id: connId,
      nick: null,
      username: null,
      realname: null,
      hostname: host,
      registered: false,
      channels: new Set(),
      awayMessage: null,
      isOper: false,
    };
    this.clients.set(connId, client);
    return [];
  }

  /**
   * Dispatch an inbound IRC message and return the resulting responses.
   *
   * This is the central dispatch method.  It:
   * 1. Looks up the client record for `connId`.
   * 2. Routes the message to the appropriate handler based on `msg.command`.
   * 3. Returns the array of `[ConnId, Message]` pairs to send.
   *
   * If the client is unknown (should not happen in normal usage), returns [].
   *
   * ## Pre-registration gate
   *
   * A client that has not yet completed the NICK+USER handshake may only
   * send: `NICK`, `USER`, `CAP`, `QUIT`, and `PASS`.  Any other command
   * gets `451 ERR_NOTREGISTERED`.
   */
  onMessage(connId: ConnId, msg: Message): Response[] {
    const client = this.clients.get(connId);
    if (client === undefined) {
      return [];
    }

    // Commands permitted before registration is complete.
    const preRegAllowed = new Set(["NICK", "USER", "CAP", "QUIT", "PASS"]);

    // Gate: reject post-registration commands from unregistered clients.
    if (!client.registered && !preRegAllowed.has(msg.command)) {
      return [
        [connId, this.makeMsg(ERR_NOTREGISTERED, "*", "You have not registered")],
      ];
    }

    switch (msg.command) {
      case "NICK":    return this.handleNick(client, msg);
      case "USER":    return this.handleUser(client, msg);
      case "CAP":     return this.handleCap(client, msg);
      case "QUIT":    return this.handleQuit(client, msg);
      case "PASS":    return this.handlePass(client, msg);
      case "JOIN":    return this.handleJoin(client, msg);
      case "PART":    return this.handlePart(client, msg);
      case "PRIVMSG": return this.handlePrivmsg(client, msg);
      case "NOTICE":  return this.handleNotice(client, msg);
      case "NAMES":   return this.handleNames(client, msg);
      case "LIST":    return this.handleList(client, msg);
      case "TOPIC":   return this.handleTopic(client, msg);
      case "KICK":    return this.handleKick(client, msg);
      case "INVITE":  return this.handleInvite(client, msg);
      case "MODE":    return this.handleMode(client, msg);
      case "PING":    return this.handlePing(client, msg);
      case "PONG":    return this.handlePong(client, msg);
      case "AWAY":    return this.handleAway(client, msg);
      case "WHOIS":   return this.handleWhois(client, msg);
      case "WHO":     return this.handleWho(client, msg);
      case "OPER":    return this.handleOper(client, msg);
      default:
        return [
          [
            connId,
            this.makeMsg(
              ERR_UNKNOWNCOMMAND,
              client.nick ?? "*",
              msg.command,
              "Unknown command"
            ),
          ],
        ];
    }
  }

  /**
   * Clean up state after a TCP connection closes.
   *
   * This is called by the transport layer when it detects that a connection
   * has been closed, either cleanly (after a QUIT) or unexpectedly.
   *
   * The cleanup procedure:
   * 1. If the client has a nick, broadcast a QUIT message to all channel
   *    members who share a channel with the disconnecting client.
   * 2. Remove the client from every channel they were in.  Destroy any
   *    channel that becomes empty.
   * 3. Remove the client's nick from the nick index.
   * 4. Remove the client record entirely.
   */
  onDisconnect(connId: ConnId): Response[] {
    const client = this.clients.get(connId);
    if (client === undefined) {
      return []; // Already cleaned up or never registered.
    }

    const responses: Response[] = [];

    // Only registered clients (with a nick) get a quit broadcast.
    if (client.registered && client.nick) {
      const quitMsg: Message = {
        prefix: clientMask(client),
        command: "QUIT",
        params: ["Connection closed"],
      };
      for (const peerId of this.uniqueChannelPeers(client)) {
        responses.push([peerId, quitMsg]);
      }
    }

    // Remove the client from every channel they were in.
    for (const chanName of client.channels) {
      const channel = this.channels.get(chanName);
      if (channel) {
        channel.members.delete(connId);
        if (channel.members.size === 0) {
          this.channels.delete(chanName);
        }
      }
    }

    // Remove from nick index.
    if (client.nick) {
      this.nicks.delete(client.nick.toLowerCase());
    }

    // Remove the client record.
    this.clients.delete(connId);

    return responses;
  }

  // -----------------------------------------------------------------------
  // Private helpers
  // -----------------------------------------------------------------------

  /**
   * Build a Message whose prefix is the server name.
   *
   * Parameters that start with `:` have the colon stripped — the serializer
   * re-adds it if the param contains spaces.
   */
  private makeMsg(command: string, ...params: string[]): Message {
    // Strip the leading colon that callers often include for readability.
    // The serialize() function re-adds it if the param contains spaces.
    const cleaned = params.map((p) => (p.startsWith(":") ? p.slice(1) : p));
    return {
      prefix: this.serverName,
      command,
      params: cleaned,
    };
  }

  /**
   * Build a server message addressed to the client's nick.
   *
   * Like `makeMsg` but automatically inserts the client's nick (or `*` if not
   * yet set) as the first parameter.  Most IRC numerics have the form:
   * `:server <numeric> <target_nick> <rest...>`
   */
  private clientMsg(client: Client, command: string, ...params: string[]): Message {
    const nick = client.nick ?? "*";
    return this.makeMsg(command, nick, ...params);
  }

  /**
   * Send the RFC 1459 welcome sequence to a newly-registered client.
   *
   * This sequence is sent exactly once, immediately after both NICK and
   * USER have been received.  It consists of numerics 001–004 (the core
   * welcome), 251 (LUSERCLIENT), and the MOTD block (375/372.../376).
   *
   * ```
   * 001 — Personalised welcome message.
   * 002 — Which server the client is connected to and its version.
   * 003 — When the server was "created" (we always say "today").
   * 004 — Machine-readable server capabilities summary.
   * 251 — How many users are currently on the network.
   * 375 — MOTD header line.
   * 372 — One per MOTD line (may be zero lines).
   * 376 — MOTD footer line.
   * ```
   */
  private welcome(client: Client): Response[] {
    const nick = client.nick ?? "*";
    const host = this.serverName;
    const ver = this.version;

    // Count total registered users for the 251 numeric.
    let userCount = 0;
    for (const c of this.clients.values()) {
      if (c.registered) userCount++;
    }

    const responses: Response[] = [
      [client.id, this.clientMsg(client, RPL_WELCOME, `Welcome to the IRC Network, ${clientMask(client)}`)],
      [client.id, this.clientMsg(client, RPL_YOURHOST, `Your host is ${host}, running version ${ver}`)],
      [client.id, this.clientMsg(client, RPL_CREATED, "This server was created today")],
      [client.id, this.makeMsg(RPL_MYINFO, nick, host, ver, "o", "o")],
      [client.id, this.clientMsg(client, RPL_LUSERCLIENT, `There are ${userCount} users on 1 server`)],
      // MOTD header
      [client.id, this.clientMsg(client, RPL_MOTDSTART, `- ${host} Message of the Day -`)],
    ];

    // One 372 line per MOTD line (may be zero).
    for (const line of this.motd) {
      responses.push([client.id, this.clientMsg(client, RPL_MOTD, `- ${line} -`)]);
    }

    // MOTD footer.
    responses.push([client.id, this.clientMsg(client, RPL_ENDOFMOTD, "End of /MOTD command.")]);

    return responses;
  }

  /**
   * Build 353 (NAMREPLY) + 366 (ENDOFNAMES) responses for a channel.
   *
   * The 353 line lists all visible members of the channel.  Each member's
   * nick is prefixed with `@` if they are a channel operator or `+` if
   * they have voice.  Regular members have no prefix.
   *
   * Example 353 payload: `= #general :@alice bob +carol`
   */
  private names(channel: Channel, requestingNick: string): Response[] {
    const connId = this.nicks.get(requestingNick.toLowerCase());
    if (connId === undefined) return [];

    const nameParts: string[] = [];
    for (const member of channel.members.values()) {
      const n = member.client.nick ?? "";
      if (member.isOperator) {
        nameParts.push(`@${n}`);
      } else if (member.hasVoice) {
        nameParts.push(`+${n}`);
      } else {
        nameParts.push(n);
      }
    }

    const namesStr = nameParts.join(" ");

    return [
      [connId, this.makeMsg(RPL_NAMREPLY, requestingNick, "=", channel.name, namesStr)],
      [connId, this.makeMsg(RPL_ENDOFNAMES, requestingNick, channel.name, "End of /NAMES list")],
    ];
  }

  /**
   * Return the set of ConnIds that share at least one channel with the client.
   *
   * Used when broadcasting a QUIT or NICK-change: we need to reach every other
   * client who can "see" this client exactly once, even if they share multiple
   * channels.  The client itself is excluded from the returned set.
   */
  private uniqueChannelPeers(client: Client): Set<ConnId> {
    const peers: Set<ConnId> = new Set();
    for (const chanName of client.channels) {
      const channel = this.channels.get(chanName);
      if (channel) {
        for (const connId of channel.members.keys()) {
          if (connId !== client.id) {
            peers.add(connId);
          }
        }
      }
    }
    return peers;
  }

  // -----------------------------------------------------------------------
  // Command handlers
  // -----------------------------------------------------------------------

  /**
   * Handle the CAP (Capability Negotiation) command.
   *
   * Modern IRC clients send `CAP LS` at the start of a connection to
   * discover server capabilities before sending NICK/USER.  We acknowledge
   * all CAP requests with an empty ACK to prevent clients from hanging.
   */
  private handleCap(client: Client, _msg: Message): Response[] {
    return [
      [
        client.id,
        {
          prefix: this.serverName,
          command: "CAP",
          params: ["*", "ACK", ""],
        },
      ],
    ];
  }

  /**
   * Handle the PASS command (connection password).
   *
   * We do not enforce connection passwords, so we silently accept PASS.
   */
  private handlePass(_client: Client, _msg: Message): Response[] {
    return [];
  }

  /**
   * Handle the NICK command — set or change a client's nickname.
   *
   * Pre-registration: validate the nick, check uniqueness, store it, and
   * — if USER has already been received — trigger the welcome sequence.
   *
   * Post-registration (nick change): broadcast `:old!user@host NICK new` to
   * all clients who share a channel with the nick-changer, then update the
   * nick index.
   *
   * Error cases:
   * - 431 — No nick given
   * - 432 — Nick fails the RFC 1459 validation
   * - 433 — Nick is already in use
   */
  private handleNick(client: Client, msg: Message): Response[] {
    // ── Validate params ────────────────────────────────────────────────────
    if (msg.params.length === 0) {
      return [[client.id, this.clientMsg(client, ERR_NONICKNAMEGIVEN, "No nickname given")]];
    }

    const newNick = msg.params[0];

    // ── Validate nick format ───────────────────────────────────────────────
    if (!validNick(newNick)) {
      return [[client.id, this.clientMsg(client, ERR_ERRONEUSNICKNAME, newNick, "Erroneous nickname")]];
    }

    // ── Check uniqueness (case-insensitive) ────────────────────────────────
    const existing = this.nicks.get(newNick.toLowerCase());
    if (existing !== undefined && existing !== client.id) {
      return [[client.id, this.clientMsg(client, ERR_NICKNAMEINUSE, newNick, "Nickname is already in use")]];
    }

    // ── Apply the nick change ──────────────────────────────────────────────
    const oldNick = client.nick;
    const oldMask = clientMask(client); // capture before we mutate

    // Remove old nick from the index (if any).
    if (oldNick) {
      this.nicks.delete(oldNick.toLowerCase());
    }

    // Register the new nick.
    client.nick = newNick;
    this.nicks.set(newNick.toLowerCase(), client.id);

    // ── Post-registration: broadcast NICK change to peers ──────────────────
    if (client.registered && oldNick !== null) {
      const nickChangeMsg: Message = {
        prefix: oldMask,
        command: "NICK",
        params: [newNick],
      };
      const responses: Response[] = [[client.id, nickChangeMsg]];
      for (const peerId of this.uniqueChannelPeers(client)) {
        responses.push([peerId, nickChangeMsg]);
      }
      return responses;
    }

    // ── Pre-registration: check if USER already done → welcome ─────────────
    if (client.username !== null) {
      client.registered = true;
      return this.welcome(client);
    }

    // NICK stored; waiting for USER — send nothing yet.
    return [];
  }

  /**
   * Handle the USER command — supply username and real name.
   *
   * Syntax: `USER <username> <mode> <unused> :<realname>`
   *
   * Error cases:
   * - 461 — Not enough parameters (need at least 4)
   *
   * After successfully storing username/realname, if NICK has already been
   * received, we trigger the welcome sequence and mark the client as registered.
   */
  private handleUser(client: Client, msg: Message): Response[] {
    if (client.registered) {
      // Already registered — ignore duplicate USER.
      return [];
    }

    if (msg.params.length < 4) {
      return [[client.id, this.clientMsg(client, ERR_NEEDMOREPARAMS, "USER", "Not enough parameters")]];
    }

    client.username = msg.params[0];
    // params[1] is mode flags (e.g., "0" or "8"), params[2] is "*" — both ignored.
    client.realname = msg.params[3];

    // If NICK has already been received, complete the registration handshake.
    if (client.nick !== null) {
      client.registered = true;
      return this.welcome(client);
    }

    // USER stored; waiting for NICK — send nothing yet.
    return [];
  }

  /**
   * Handle the QUIT command — graceful client disconnect.
   *
   * 1. Broadcast a QUIT message to all channel peers.
   * 2. Send an ERROR message to the quitting client.
   * 3. Clean up state manually (to avoid double-broadcasting).
   */
  private handleQuit(client: Client, msg: Message): Response[] {
    const quitReason = msg.params[0] ?? "Quit";
    const responses: Response[] = [];

    // Broadcast QUIT to channel peers.
    if (client.registered && client.nick) {
      const quitBroadcast: Message = {
        prefix: clientMask(client),
        command: "QUIT",
        params: [quitReason],
      };
      for (const peerId of this.uniqueChannelPeers(client)) {
        responses.push([peerId, quitBroadcast]);
      }
    }

    // Send ERROR to the quitting client as a farewell.
    responses.push([
      client.id,
      {
        prefix: null,
        command: "ERROR",
        params: [`Closing Link: ${client.hostname} (Quit: ${quitReason})`],
      },
    ]);

    // Clean up state manually (don't call onDisconnect to avoid double-broadcast).
    for (const chanName of client.channels) {
      const channel = this.channels.get(chanName);
      if (channel) {
        channel.members.delete(client.id);
        if (channel.members.size === 0) {
          this.channels.delete(chanName);
        }
      }
    }

    if (client.nick) {
      this.nicks.delete(client.nick.toLowerCase());
    }

    this.clients.delete(client.id);

    return responses;
  }

  /**
   * Handle the JOIN command — add a client to a channel.
   *
   * Syntax: `JOIN <#channel>[,<#channel2>...]`
   *
   * When joining a channel:
   * - If the channel does not exist, create it.  The first member becomes
   *   the channel operator automatically.
   * - Broadcast `:nick!user@host JOIN #channel` to ALL members.
   * - Send NAMES (353 + 366) to the joiner.
   *
   * Error cases:
   * - 461 — No channel specified
   */
  private handleJoin(client: Client, msg: Message): Response[] {
    if (msg.params.length === 0) {
      return [[client.id, this.clientMsg(client, ERR_NEEDMOREPARAMS, "JOIN", "Not enough parameters")]];
    }

    // Handle comma-separated list; take all of them.
    const chanNames = msg.params[0].split(",");
    const responses: Response[] = [];

    for (const chanNameRaw of chanNames) {
      const chanName = chanNameRaw.toLowerCase();

      // Skip if already in channel.
      if (client.channels.has(chanName)) continue;

      // Create the channel if it does not exist yet.
      if (!this.channels.has(chanName)) {
        this.channels.set(chanName, {
          name: chanName,
          topic: "",
          members: new Map(),
          modes: new Set(),
          banList: [],
        });
      }

      const channel = this.channels.get(chanName)!;

      // Add the client to the channel.  The first member is the operator.
      const isFirstMember = channel.members.size === 0;
      channel.members.set(client.id, {
        client,
        isOperator: isFirstMember,
        hasVoice: false,
      });
      client.channels.add(chanName);

      // Broadcast JOIN to all current members (including the joiner).
      const joinMsg: Message = {
        prefix: clientMask(client),
        command: "JOIN",
        params: [chanName],
      };
      for (const memberConnId of channel.members.keys()) {
        responses.push([memberConnId, joinMsg]);
      }

      // Send the current topic if one is set.
      const nick = client.nick ?? "*";
      if (channel.topic) {
        responses.push([client.id, this.makeMsg(RPL_TOPIC, nick, chanName, channel.topic)]);
      } else {
        responses.push([client.id, this.makeMsg(RPL_NOTOPIC, nick, chanName, "No topic is set")]);
      }

      // Send NAMES (353 + 366) to the joiner.
      responses.push(...this.names(channel, nick));
    }

    return responses;
  }

  /**
   * Handle the PART command — remove a client from a channel.
   *
   * Syntax: `PART <#channel> [:<message>]`
   *
   * The optional part message is relayed in the PART broadcast.
   * After removing the client, if the channel is empty it is destroyed.
   *
   * Error cases:
   * - 442 — Client is not in the channel
   * - 461 — No channel specified
   */
  private handlePart(client: Client, msg: Message): Response[] {
    if (msg.params.length === 0) {
      return [[client.id, this.clientMsg(client, ERR_NEEDMOREPARAMS, "PART", "Not enough parameters")]];
    }

    const chanName = msg.params[0].toLowerCase();
    const partMsgText = msg.params[1] ?? (client.nick ?? "");

    // Check the client is actually in this channel.
    if (!client.channels.has(chanName)) {
      return [[client.id, this.clientMsg(client, ERR_NOTONCHANNEL, chanName, "You're not on that channel")]];
    }

    const channel = this.channels.get(chanName);
    if (!channel) {
      return [[client.id, this.clientMsg(client, ERR_NOTONCHANNEL, chanName, "You're not on that channel")]];
    }

    // Build the PART message *before* removing the client from the channel
    // so that they still appear in the member list and receive their own
    // PART broadcast.
    const partBroadcast: Message = {
      prefix: clientMask(client),
      command: "PART",
      params: [chanName, partMsgText],
    };

    // Collect member IDs *before* removing the client.
    const memberIds = Array.from(channel.members.keys());

    // Remove the client from the channel.
    channel.members.delete(client.id);
    client.channels.delete(chanName);

    // Broadcast PART to all former members (including the departing client).
    const responses: Response[] = memberIds.map((id) => [id, partBroadcast] as Response);

    // Destroy the channel if it is now empty.
    if (channel.members.size === 0) {
      this.channels.delete(chanName);
    }

    return responses;
  }

  /**
   * Common delivery logic for PRIVMSG and NOTICE.
   *
   * Target is either a channel (starts with `#`) or a nick.
   * - For channels: deliver to all members except the sender.
   * - For nicks: deliver directly to the target client.
   */
  private deliverMessage(client: Client, msg: Message, command: string): Response[] {
    if (msg.params.length === 0) {
      return [[client.id, this.clientMsg(client, ERR_NEEDMOREPARAMS, command, "No recipient given")]];
    }

    const target = msg.params[0];
    const text = msg.params[1] ?? "";

    if (!text) {
      return [[client.id, this.clientMsg(client, "412", "No text to send")]];
    }

    const outMsg: Message = {
      prefix: clientMask(client),
      command,
      params: [target, text],
    };

    const responses: Response[] = [];

    if (target.startsWith("#")) {
      // ── Channel message ────────────────────────────────────────────────
      const chanName = target.toLowerCase();
      const channel = this.channels.get(chanName);
      if (!channel) {
        return [[client.id, this.clientMsg(client, ERR_NOSUCHCHANNEL, target, "No such channel")]];
      }
      // Deliver to all members except the sender.
      for (const [memberConnId] of channel.members) {
        if (memberConnId !== client.id) {
          responses.push([memberConnId, outMsg]);
        }
      }
    } else {
      // ── Private message to a nick ──────────────────────────────────────
      const targetConn = this.nicks.get(target.toLowerCase());
      if (targetConn === undefined) {
        return [[client.id, this.clientMsg(client, ERR_NOSUCHNICK, target, "No such nick/channel")]];
      }
      const targetClient = this.clients.get(targetConn);
      if (!targetClient) {
        return [[client.id, this.clientMsg(client, ERR_NOSUCHNICK, target, "No such nick/channel")]];
      }
      responses.push([targetConn, outMsg]);
      // For PRIVMSG (not NOTICE), if the target is away, inform the sender.
      if (command === "PRIVMSG" && targetClient.awayMessage !== null) {
        responses.push([
          client.id,
          this.clientMsg(client, RPL_AWAY, targetClient.nick ?? target, targetClient.awayMessage),
        ]);
      }
    }

    return responses;
  }

  /** Handle PRIVMSG — send a message to a nick or channel. */
  private handlePrivmsg(client: Client, msg: Message): Response[] {
    return this.deliverMessage(client, msg, "PRIVMSG");
  }

  /** Handle NOTICE — send a notice (no auto-replies). */
  private handleNotice(client: Client, msg: Message): Response[] {
    return this.deliverMessage(client, msg, "NOTICE");
  }

  /**
   * Handle NAMES — list members of a channel.
   *
   * Returns 353 (NAMREPLY) + 366 (ENDOFNAMES) for the requested channel.
   * If no channel is specified, returns NAMES for all channels.
   */
  private handleNames(client: Client, msg: Message): Response[] {
    const nick = client.nick ?? "*";

    if (msg.params.length > 0) {
      const chanName = msg.params[0].toLowerCase();
      const channel = this.channels.get(chanName);
      if (channel) {
        return this.names(channel, nick);
      } else {
        return [[client.id, this.makeMsg(RPL_ENDOFNAMES, nick, chanName, "End of /NAMES list")]];
      }
    } else {
      const responses: Response[] = [];
      for (const channel of this.channels.values()) {
        responses.push(...this.names(channel, nick));
      }
      return responses;
    }
  }

  /**
   * Handle LIST — enumerate all channels.
   *
   * Returns 321 (LISTSTART), one 322 (LIST) per channel, and 323 (LISTEND).
   */
  private handleList(client: Client, _msg: Message): Response[] {
    const nick = client.nick ?? "*";
    const responses: Response[] = [
      [client.id, this.makeMsg(RPL_LISTSTART, nick, "Channel", "Users  Name")],
    ];

    for (const channel of this.channels.values()) {
      responses.push([
        client.id,
        this.makeMsg(RPL_LIST, nick, channel.name, String(channel.members.size), channel.topic),
      ]);
    }

    responses.push([client.id, this.makeMsg(RPL_LISTEND, nick, "End of /LIST")]);
    return responses;
  }

  /**
   * Handle TOPIC — get or set a channel's topic.
   *
   * Query (`TOPIC #channel`): returns 332 or 331.
   * Set (`TOPIC #channel :new topic`): updates and broadcasts to all members.
   */
  private handleTopic(client: Client, msg: Message): Response[] {
    if (msg.params.length === 0) {
      return [[client.id, this.clientMsg(client, ERR_NEEDMOREPARAMS, "TOPIC", "Not enough parameters")]];
    }

    const chanName = msg.params[0].toLowerCase();
    const nick = client.nick ?? "*";

    const channel = this.channels.get(chanName);
    if (!channel) {
      return [[client.id, this.clientMsg(client, ERR_NOSUCHCHANNEL, chanName, "No such channel")]];
    }

    if (!channel.members.has(client.id)) {
      return [[client.id, this.clientMsg(client, ERR_NOTONCHANNEL, chanName, "You're not on that channel")]];
    }

    if (msg.params.length < 2) {
      // Query mode
      if (channel.topic) {
        return [[client.id, this.makeMsg(RPL_TOPIC, nick, chanName, channel.topic)]];
      } else {
        return [[client.id, this.makeMsg(RPL_NOTOPIC, nick, chanName, "No topic is set")]];
      }
    } else {
      // Set mode
      channel.topic = msg.params[1];
      const topicBroadcast: Message = {
        prefix: clientMask(client),
        command: "TOPIC",
        params: [chanName, channel.topic],
      };
      return Array.from(channel.members.keys()).map((id) => [id, topicBroadcast] as Response);
    }
  }

  /**
   * Handle KICK — remove a member from a channel (op only).
   *
   * Syntax: `KICK <#channel> <nick> [:<reason>]`
   *
   * Error cases:
   * - 441 — Target not in channel
   * - 442 — Kicker not in channel
   * - 461 — Not enough parameters
   * - 482 — Not channel operator
   */
  private handleKick(client: Client, msg: Message): Response[] {
    if (msg.params.length < 2) {
      return [[client.id, this.clientMsg(client, ERR_NEEDMOREPARAMS, "KICK", "Not enough parameters")]];
    }

    const chanName = msg.params[0].toLowerCase();
    const targetNick = msg.params[1];
    const reason = msg.params[2] ?? (client.nick ?? "");

    const channel = this.channels.get(chanName);
    if (!channel) {
      return [[client.id, this.clientMsg(client, ERR_NOSUCHCHANNEL, chanName, "No such channel")]];
    }

    const kickerMember = channel.members.get(client.id);
    if (!kickerMember) {
      return [[client.id, this.clientMsg(client, ERR_NOTONCHANNEL, chanName, "You're not on that channel")]];
    }

    if (!kickerMember.isOperator) {
      return [[client.id, this.clientMsg(client, ERR_CHANOPRIVSNEEDED, chanName, "You're not channel operator")]];
    }

    const targetConn = this.nicks.get(targetNick.toLowerCase());
    if (targetConn === undefined || !channel.members.has(targetConn)) {
      return [[client.id, this.clientMsg(client, ERR_USERNOTINCHANNEL, targetNick, chanName, "They aren't on that channel")]];
    }

    const targetClient = this.clients.get(targetConn);

    // Broadcast KICK to all current members (before removing the victim).
    const kickBroadcast: Message = {
      prefix: clientMask(client),
      command: "KICK",
      params: [chanName, targetNick, reason],
    };
    const responses: Response[] = Array.from(channel.members.keys()).map(
      (id) => [id, kickBroadcast] as Response
    );

    // Remove victim from the channel.
    channel.members.delete(targetConn);
    if (targetClient) {
      targetClient.channels.delete(chanName);
    }

    // Destroy channel if empty.
    if (channel.members.size === 0) {
      this.channels.delete(chanName);
    }

    return responses;
  }

  /**
   * Handle INVITE — invite a nick to a channel.
   *
   * Syntax: `INVITE <nick> <#channel>`
   *
   * Sends an INVITE message directly to the target nick.
   * The inviting client receives 341 RPL_INVITING as confirmation.
   */
  private handleInvite(client: Client, msg: Message): Response[] {
    if (msg.params.length < 2) {
      return [[client.id, this.clientMsg(client, ERR_NEEDMOREPARAMS, "INVITE", "Not enough parameters")]];
    }

    const targetNick = msg.params[0];
    const chanName = msg.params[1].toLowerCase();
    const nick = client.nick ?? "*";

    const targetConn = this.nicks.get(targetNick.toLowerCase());
    if (targetConn === undefined) {
      return [[client.id, this.clientMsg(client, ERR_NOSUCHNICK, targetNick, "No such nick/channel")]];
    }

    const responses: Response[] = [
      [client.id, this.makeMsg(RPL_INVITING, nick, chanName, targetNick)],
    ];

    const inviteMsg: Message = {
      prefix: clientMask(client),
      command: "INVITE",
      params: [targetNick, chanName],
    };
    responses.push([targetConn, inviteMsg]);

    return responses;
  }

  /**
   * Handle MODE — query or set channel/user modes.
   *
   * Supports:
   * - `MODE #channel` → 324 RPL_CHANNELMODEIS
   * - `MODE #channel +/-X` → acknowledge with MODE broadcast
   * - `MODE nick` → 221 RPL_UMODEIS
   * - `MODE nick +/-X` → acknowledge with MODE broadcast
   *
   * Full mode enforcement is out of scope for v1.
   */
  private handleMode(client: Client, msg: Message): Response[] {
    if (msg.params.length === 0) {
      return [[client.id, this.clientMsg(client, ERR_NEEDMOREPARAMS, "MODE", "Not enough parameters")]];
    }

    const target = msg.params[0];
    const nick = client.nick ?? "*";

    if (target.startsWith("#")) {
      // ── Channel MODE ──────────────────────────────────────────────────
      const chanName = target.toLowerCase();
      const channel = this.channels.get(chanName);
      if (!channel) {
        return [[client.id, this.clientMsg(client, ERR_NOSUCHCHANNEL, chanName, "No such channel")]];
      }

      if (msg.params.length === 1) {
        // Query: return current channel modes.
        const modeStr = channel.modes.size > 0 ? "+" + [...channel.modes].sort().join("") : "+";
        return [[client.id, this.makeMsg(RPL_CHANNELMODEIS, nick, chanName, modeStr)]];
      } else {
        // Set: acknowledge by broadcasting MODE to channel members.
        const modeStr = msg.params[1];
        if (modeStr.startsWith("+")) {
          for (const ch of modeStr.slice(1)) {
            channel.modes.add(ch);
          }
        } else if (modeStr.startsWith("-")) {
          for (const ch of modeStr.slice(1)) {
            channel.modes.delete(ch);
          }
        }

        const modeBroadcast: Message = {
          prefix: clientMask(client),
          command: "MODE",
          params: [chanName, modeStr],
        };
        return Array.from(channel.members.keys()).map((id) => [id, modeBroadcast] as Response);
      }
    } else {
      // ── User MODE ─────────────────────────────────────────────────────
      if (msg.params.length === 1) {
        return [[client.id, this.makeMsg("221", nick, "+")]];
      } else {
        const modeStr = msg.params[1];
        const modeBroadcast: Message = {
          prefix: clientMask(client),
          command: "MODE",
          params: [target, modeStr],
        };
        return [[client.id, modeBroadcast]];
      }
    }
  }

  /**
   * Handle PING — keepalive from client to server.
   *
   * The client sends PING periodically to verify the connection is alive.
   * We respond with a matching PONG carrying the same server token.
   */
  private handlePing(client: Client, msg: Message): Response[] {
    const serverToken = msg.params[0] ?? this.serverName;
    return [
      [
        client.id,
        {
          prefix: this.serverName,
          command: "PONG",
          params: [this.serverName, serverToken],
        },
      ],
    ];
  }

  /**
   * Handle PONG — client's response to a server PING.
   *
   * We don't send server-initiated PINGs in v1, so we simply ignore PONG.
   */
  private handlePong(_client: Client, _msg: Message): Response[] {
    return [];
  }

  /**
   * Handle AWAY — set or clear away status.
   *
   * `AWAY :<message>` → set away (306 RPL_NOWAWAY).
   * `AWAY`            → clear away (305 RPL_UNAWAY).
   *
   * When another client sends PRIVMSG to an away user, the server
   * automatically sends 301 RPL_AWAY with the away message text.
   */
  private handleAway(client: Client, msg: Message): Response[] {
    if (msg.params.length > 0 && msg.params[0]) {
      // Setting away.
      client.awayMessage = msg.params[0];
      return [[client.id, this.clientMsg(client, RPL_NOWAWAY, "You have been marked as being away")]];
    } else {
      // Clearing away.
      client.awayMessage = null;
      return [[client.id, this.clientMsg(client, RPL_UNAWAY, "You are no longer marked as being away")]];
    }
  }

  /**
   * Handle WHOIS — retrieve information about a nick.
   *
   * Syntax: `WHOIS <nick>`
   *
   * Returns 311 (user), 312 (server), 319 (channels), optionally 301 (away),
   * and 318 (end of whois).
   */
  private handleWhois(client: Client, msg: Message): Response[] {
    if (msg.params.length === 0) {
      return [[client.id, this.clientMsg(client, ERR_NEEDMOREPARAMS, "WHOIS", "Not enough parameters")]];
    }

    const targetNick = msg.params[0];
    const nick = client.nick ?? "*";

    const targetConn = this.nicks.get(targetNick.toLowerCase());
    if (targetConn === undefined) {
      return [[client.id, this.clientMsg(client, ERR_NOSUCHNICK, targetNick, "No such nick/channel")]];
    }

    const target = this.clients.get(targetConn);
    if (!target) {
      return [[client.id, this.clientMsg(client, ERR_NOSUCHNICK, targetNick, "No such nick/channel")]];
    }

    const targetNickStr = target.nick ?? targetNick;
    const responses: Response[] = [
      // 311: nick user host * :realname
      [
        client.id,
        this.makeMsg(
          RPL_WHOISUSER,
          nick,
          targetNickStr,
          target.username ?? "*",
          target.hostname,
          "*",
          target.realname ?? ""
        ),
      ],
      // 312: nick server :server info
      [
        client.id,
        this.makeMsg(RPL_WHOISSERVER, nick, targetNickStr, this.serverName, "IRC server"),
      ],
    ];

    // 319: channels the user is in.
    if (target.channels.size > 0) {
      const chanList = [...target.channels].sort().join(" ");
      responses.push([client.id, this.makeMsg(RPL_WHOISCHANNELS, nick, targetNickStr, chanList)]);
    }

    // 301: away message if applicable.
    if (target.awayMessage !== null) {
      responses.push([client.id, this.makeMsg(RPL_AWAY, nick, targetNickStr, target.awayMessage)]);
    }

    // 318: terminator.
    responses.push([client.id, this.makeMsg(RPL_ENDOFWHOIS, nick, targetNickStr, "End of /WHOIS list")]);

    return responses;
  }

  /**
   * Handle WHO — list users matching a mask.
   *
   * Syntax: `WHO [<mask>]`
   *
   * Returns 352 (WHOREPLY) rows followed by 315 (ENDOFWHO).
   *
   * `H` = here (not away), `G` = gone (away).
   */
  private handleWho(client: Client, msg: Message): Response[] {
    const nick = client.nick ?? "*";
    const mask = msg.params[0] ?? "*";
    const responses: Response[] = [];

    const whoRow = (targetClient: Client, channelName: string = "*"): Response => {
      const hereOrGone = targetClient.awayMessage !== null ? "G" : "H";
      return [
        client.id,
        this.makeMsg(
          RPL_WHOREPLY,
          nick,
          channelName,
          targetClient.username ?? "*",
          targetClient.hostname,
          this.serverName,
          targetClient.nick ?? "*",
          hereOrGone,
          `0 ${targetClient.realname ?? ""}`
        ),
      ];
    };

    if (mask.startsWith("#")) {
      // List members of the given channel.
      const chanName = mask.toLowerCase();
      const channel = this.channels.get(chanName);
      if (channel) {
        for (const member of channel.members.values()) {
          responses.push(whoRow(member.client, chanName));
        }
      }
    } else {
      // List all registered clients.
      for (const c of this.clients.values()) {
        if (c.registered) {
          responses.push(whoRow(c));
        }
      }
    }

    responses.push([client.id, this.makeMsg(RPL_ENDOFWHO, nick, mask, "End of /WHO list")]);
    return responses;
  }

  /**
   * Handle OPER — gain IRC operator privileges.
   *
   * Syntax: `OPER <name> <password>`
   *
   * Error cases:
   * - 461 — Not enough parameters
   * - 464 — Wrong password
   */
  private handleOper(client: Client, msg: Message): Response[] {
    if (msg.params.length < 2) {
      return [[client.id, this.clientMsg(client, ERR_NEEDMOREPARAMS, "OPER", "Not enough parameters")]];
    }

    // We ignore the name (params[0]); only the password matters.
    const password = msg.params[1];

    if (this.operPassword && password === this.operPassword) {
      client.isOper = true;
      return [[client.id, this.clientMsg(client, RPL_YOUREOPER, "You are now an IRC operator")]];
    } else {
      return [[client.id, this.clientMsg(client, ERR_PASSWDMISMATCH, "Password incorrect")]];
    }
  }
}
