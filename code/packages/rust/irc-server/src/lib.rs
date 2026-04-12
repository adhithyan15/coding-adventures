//! # irc-server — IRC server state machine (channels, nicks, command dispatch)
//!
//! This crate is the brain of an IRC server.  It knows nothing about sockets,
//! threads, or I/O — it is a *pure state machine* that consumes [`Message`]
//! values (from `irc-proto`) and produces lists of [`Response`] pairs that the
//! transport layer should forward to the appropriate connections.
//!
//! ## Architecture overview
//!
//! An IRC server manages three kinds of mutable state:
//!
//! 1. **Clients** — each TCP connection is represented by a `Client` object
//!    keyed by a `ConnId` (an integer the transport layer assigns and owns).
//!    A client starts in the *unregistered* state and becomes *registered* once
//!    it has supplied both a NICK and a USER command.  Only registered clients
//!    may join channels or send messages.
//!
//! 2. **Channels** — a `Channel` groups a set of registered clients.  The
//!    first client to join a channel automatically becomes its *operator* and
//!    gains the power to kick members and change the topic.
//!
//! 3. **Nick index** — a `HashMap<String, ConnId>` mapping lowercase nick names
//!    to connection IDs enables O(1) uniqueness checks and direct-message delivery.
//!
//! ## Public interface
//!
//! [`IRCServer`] has exactly three methods that the transport layer calls:
//!
//! * `on_connect(conn_id, host)` — a new TCP connection arrived.
//! * `on_message(conn_id, msg)`  — a parsed message arrived from a client.
//! * `on_disconnect(conn_id)`    — the TCP connection closed.
//!
//! Each method returns a `Vec<Response>`.  The transport layer iterates this
//! list and sends each message to the given connection.  The server itself
//! never touches sockets.
//!
//! ## Example
//!
//! ```
//! use irc_server::{IRCServer, ConnId};
//! use irc_proto::parse;
//!
//! let mut server = IRCServer::new("irc.local", vec!["Welcome!".to_string()], "");
//!
//! // New connection arrives:
//! let _ = server.on_connect(ConnId(1), "127.0.0.1");
//!
//! // Client sends NICK:
//! let msg = parse("NICK alice").unwrap();
//! let _ = server.on_message(ConnId(1), &msg);
//!
//! // Client sends USER — this triggers the welcome sequence:
//! let msg = parse("USER alice 0 * :Alice Smith").unwrap();
//! let responses = server.on_message(ConnId(1), &msg);
//! assert!(!responses.is_empty()); // 001 Welcome etc.
//! ```

use std::collections::{HashMap, HashSet};
use irc_proto::Message;

// Re-export Message so dependents can import it from here if needed.
pub use irc_proto::Message as IrcMessage;

// ──────────────────────────────────────────────────────────────────────────────
// Core type aliases
// ──────────────────────────────────────────────────────────────────────────────

/// Opaque integer that the transport layer assigns to each TCP connection.
///
/// Using a newtype wrapper rather than a bare `u64` lets the compiler catch
/// accidental mix-ups between connection IDs and other integers.  The cost
/// is zero at runtime — Rust erases newtype wrappers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnId(pub u64);

/// One outbound message destined for a specific connection.
///
/// The `IRCServer` never writes to sockets — it only produces these pairs.
/// The transport layer iterates the returned `Vec<Response>` and calls its
/// own `send_to()` method for each entry.
pub struct Response {
    pub conn_id: ConnId,
    pub msg: Message,
}

// ──────────────────────────────────────────────────────────────────────────────
// Nick validation
// ──────────────────────────────────────────────────────────────────────────────

/// Return true if `nick` conforms to RFC 1459 nickname rules.
///
/// A valid nick:
/// - is 1–9 characters long
/// - starts with a letter OR one of: `[ ] \ ` ^ { | } _`
/// - subsequent characters may additionally include digits and the hyphen
///
/// # Examples
///
/// ```
/// # use irc_server::valid_nick;
/// assert!(valid_nick("alice"));
/// assert!(valid_nick("_bot"));
/// assert!(!valid_nick(""));       // empty → invalid
/// assert!(!valid_nick("bad nick")); // space → invalid
/// assert!(!valid_nick("aaaaaaaaaa")); // 10 chars → too long
/// ```
pub fn valid_nick(nick: &str) -> bool {
    // RFC 1459 §2.3.1 nick grammar:
    //   nick = ( letter / special ) *( letter / number / special / "-" )
    //   special = "[" / "]" / "\" / "`" / "^" / "{" / "|" / "}" / "_"
    //
    // Maximum length: 9 characters.
    if nick.is_empty() || nick.len() > 9 {
        return false;
    }

    let mut chars = nick.chars();

    // First character: letter or one of the special characters listed above.
    let first = chars.next().unwrap(); // safe: we checked non-empty
    if !is_nick_first_char(first) {
        return false;
    }

    // Subsequent characters: letter, digit, special, or hyphen.
    for ch in chars {
        if !is_nick_body_char(ch) {
            return false;
        }
    }

    true
}

fn is_nick_first_char(c: char) -> bool {
    // Letters (a-z, A-Z) or special IRC characters.
    c.is_ascii_alphabetic() || is_nick_special(c)
}

fn is_nick_body_char(c: char) -> bool {
    // Body adds: digits, hyphen.
    c.is_ascii_alphanumeric() || is_nick_special(c) || c == '-'
}

fn is_nick_special(c: char) -> bool {
    // IRC special characters allowed in nick names: [ ] \ ` ^ { | } _
    matches!(c, '[' | ']' | '\\' | '`' | '^' | '{' | '|' | '}' | '_')
}

// ──────────────────────────────────────────────────────────────────────────────
// State model — Client
// ──────────────────────────────────────────────────────────────────────────────

/// All the server-side state we keep for one TCP connection.
///
/// A freshly-connected client is *unregistered*: `registered=false`,
/// `nick=None`, `username=None`, `realname=None`.  The client
/// transitions to *registered* once both `NICK` and `USER` have been
/// successfully processed.  Until that point, only `NICK`, `USER`,
/// `CAP`, and `QUIT` are accepted; everything else gets `451 ERR_NOTREGISTERED`.
///
/// The `channels` field tracks lowercase channel names the client has
/// joined, giving us O(1) membership tests and enabling cleanup on disconnect
/// without iterating every channel on the server.
#[derive(Debug, Clone)]
pub struct Client {
    /// The transport-layer connection identifier.  Immutable once assigned.
    pub id: ConnId,

    /// IRC nickname.  None until the client sends NICK.
    pub nick: Option<String>,

    /// IRC username (from the USER command's first parameter).
    pub username: Option<String>,

    /// Real name / GECOS (from the USER command's trailing parameter).
    pub realname: Option<String>,

    /// Hostname of the connecting peer, supplied by the transport layer on
    /// connect.  Used in the `nick!user@host` mask attached to relayed messages.
    pub hostname: String,

    /// True once both NICK and USER have been processed successfully.
    pub registered: bool,

    /// Lowercase channel names this client has joined.
    pub channels: HashSet<String>,

    /// Optional away message.  None means the client is not away.
    pub away_message: Option<String>,

    /// True if the client has authenticated with OPER.
    pub is_oper: bool,
}

impl Client {
    /// Create a new unregistered client.
    fn new(id: ConnId, hostname: &str) -> Self {
        Client {
            id,
            nick: None,
            username: None,
            realname: None,
            hostname: hostname.to_string(),
            registered: false,
            channels: HashSet::new(),
            away_message: None,
            is_oper: false,
        }
    }

    /// Return the `nick!user@host` mask used as a message prefix.
    ///
    /// This is the standard IRC identity string.  Other clients see this in
    /// the prefix of any message we relay on behalf of this client.
    ///
    /// Example: `"alice!alice@192.168.1.1"`
    pub fn mask(&self) -> String {
        let nick = self.nick.as_deref().unwrap_or("*");
        let user = self.username.as_deref().unwrap_or("*");
        format!("{}!{}@{}", nick, user, self.hostname)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// State model — Channel
// ──────────────────────────────────────────────────────────────────────────────

/// Per-membership metadata for a client inside a channel.
///
/// A single client may be in many channels simultaneously; each membership
/// is represented by a separate `ChannelMember` entry in `Channel.members`.
#[derive(Debug, Clone)]
pub struct ChannelMember {
    /// True if this client is a channel operator (@).
    /// The first member of a newly-created channel gets this.
    pub is_operator: bool,

    /// True if this client has voice privilege (+v).
    /// Voice allows speaking in moderated (+m) channels.
    pub has_voice: bool,
}

/// All server-side state for one IRC channel.
///
/// Channel names are always stored and compared in lowercase, normalised when
/// the client sends `JOIN`.  Clients see the lowercase name in all responses.
///
/// `members`   — maps ConnId to ChannelMember.  Using ConnId as the key
///               gives O(1) look-ups by connection without needing the nick.
/// `modes`     — the set of single-character channel mode letters currently
///               active (e.g. `{'n', 't'}`).
/// `ban_list`  — list of nick/host mask patterns that are banned.
///               Stored but not enforced in this v1 scope.
#[derive(Debug, Clone)]
pub struct Channel {
    /// Lowercase channel name including the '#' sigil.
    pub name: String,

    /// Human-readable topic string.  Empty means no topic is set.
    pub topic: String,

    /// Active members indexed by ConnId.
    pub members: HashMap<ConnId, ChannelMember>,

    /// Active channel mode flags (single characters).
    pub modes: HashSet<char>,

    /// Ban mask list (stored but not enforced in v1).
    pub ban_list: Vec<String>,
}

impl Channel {
    fn new(name: &str) -> Self {
        Channel {
            name: name.to_string(),
            topic: String::new(),
            members: HashMap::new(),
            modes: HashSet::new(),
            ban_list: Vec::new(),
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// IRC numeric reply constants
// ──────────────────────────────────────────────────────────────────────────────
// Named constants make the handler code self-documenting.  Instead of
// scattering magic string literals like "433" throughout the code, we define
// them here with their RFC 1459 symbolic names so readers can look them up.

const RPL_WELCOME: &str = "001";       // :Welcome to the IRC Network, nick!user@host
const RPL_YOURHOST: &str = "002";      // :Your host is <name>, running version <ver>
const RPL_CREATED: &str = "003";       // :This server was created today
const RPL_MYINFO: &str = "004";        // <servername> <version> <usermodes> <chanmodes>
const RPL_LUSERCLIENT: &str = "251";   // :There are N users on 1 server
const RPL_AWAY: &str = "301";          // <nick> :<away message>
const RPL_UNAWAY: &str = "305";        // :You are no longer marked as being away
const RPL_NOWAWAY: &str = "306";       // :You have been marked as being away
const RPL_WHOISUSER: &str = "311";     // <nick> <user> <host> * :<realname>
const RPL_WHOISSERVER: &str = "312";   // <nick> <server> :<server info>
const RPL_WHOISCHANNELS: &str = "319"; // <nick> :{[@|+]channel...}
const RPL_LIST: &str = "322";          // <channel> <# visible> :<topic>
const RPL_LISTSTART: &str = "321";     // Channel :Users  Name
const RPL_LISTEND: &str = "323";       // :End of /LIST
const RPL_CHANNELMODEIS: &str = "324"; // <channel> <mode> [<mode params>]
const RPL_NOTOPIC: &str = "331";       // <channel> :No topic is set
const RPL_TOPIC: &str = "332";         // <channel> :<topic>
const RPL_INVITING: &str = "341";      // <channel> <nick>
const RPL_WHOREPLY: &str = "352";      // <channel> <user> <host> <server> <nick> H|G :hops realname
const RPL_NAMREPLY: &str = "353";      // = <channel> :<prefix>nick...
const RPL_ENDOFNAMES: &str = "366";    // <channel> :End of /NAMES list
const RPL_ENDOFWHO: &str = "315";      // <name> :End of /WHO list
const RPL_MOTDSTART: &str = "375";     // :- <server> Message of the Day -
const RPL_MOTD: &str = "372";          // :- <text> -
const RPL_ENDOFMOTD: &str = "376";     // :End of /MOTD command
const RPL_YOUREOPER: &str = "381";     // :You are now an IRC operator
const RPL_ENDOFWHOIS: &str = "318";    // <nick> :End of /WHOIS list

const ERR_NOSUCHNICK: &str = "401";         // <nick/channel> :No such nick/channel
const ERR_NOSUCHCHANNEL: &str = "403";      // <channel> :No such channel
const ERR_UNKNOWNCOMMAND: &str = "421";     // <command> :Unknown command
const ERR_NONICKNAMEGIVEN: &str = "431";    // :No nickname given
const ERR_ERRONEUSNICKNAME: &str = "432";   // <nick> :Erroneous nickname
const ERR_NICKNAMEINUSE: &str = "433";      // <nick> :Nickname is already in use
const ERR_USERNOTINCHANNEL: &str = "441";   // <nick> <channel> :They aren't on that channel
const ERR_NOTONCHANNEL: &str = "442";       // <channel> :You're not on that channel
const ERR_NEEDMOREPARAMS: &str = "461";     // <command> :Not enough parameters
const ERR_PASSWDMISMATCH: &str = "464";     // :Password incorrect
const ERR_CHANOPRIVSNEEDED: &str = "482";   // <channel> :You're not channel operator
const ERR_NOTREGISTERED: &str = "451";      // :You have not registered

// ──────────────────────────────────────────────────────────────────────────────
// IRCServer
// ──────────────────────────────────────────────────────────────────────────────

/// Pure IRC server state machine.
///
/// This struct contains the complete server state (clients, channels, nick
/// index) and the logic for every IRC command.  It never touches the network
/// — the transport layer calls `on_connect`, `on_message`, and
/// `on_disconnect`, and the server returns `Vec<Response>` that the transport
/// should deliver.
///
/// **Concurrency note**: this struct is intentionally **not** thread-safe.
/// If the transport layer is multi-threaded, it must serialize calls to these
/// three methods (e.g., with a `Mutex`).
///
/// # Example
///
/// ```
/// use irc_server::{IRCServer, ConnId};
/// use irc_proto::parse;
///
/// let mut server = IRCServer::new("irc.example.com", vec![], "");
///
/// let _ = server.on_connect(ConnId(1), "192.168.1.10");
/// let msg = parse("NICK alice").unwrap();
/// let _ = server.on_message(ConnId(1), &msg);
/// let msg = parse("USER alice 0 * :Alice Smith").unwrap();
/// let responses = server.on_message(ConnId(1), &msg);
/// // → responses contains the 001–376 welcome sequence
/// assert!(!responses.is_empty());
/// ```
pub struct IRCServer {
    /// The hostname this server advertises (e.g. `"irc.example.com"`).
    server_name: String,

    /// Software version string, shown in 002 and 004 numerics.
    version: String,

    /// Lines of the Message of the Day.  Zero lines means no MOTD lines,
    /// but we still send the 375/376 start/end markers.
    motd: Vec<String>,

    /// Plaintext password for the OPER command.  Empty = disabled.
    oper_password: String,

    /// All known clients keyed by ConnId.
    clients: HashMap<ConnId, Client>,

    /// All active channels keyed by lowercase name (including '#').
    channels: HashMap<String, Channel>,

    /// Nick → ConnId index.  All nicks stored in lowercase for case-insensitive
    /// uniqueness checks.
    nicks: HashMap<String, ConnId>,
}

impl IRCServer {
    /// Create a new IRC server with the given configuration.
    ///
    /// # Parameters
    ///
    /// - `server_name`: Hostname advertised to clients (e.g. `"irc.example.com"`).
    ///   Appears in the prefix of all server-generated messages.
    /// - `motd`: Lines of the Message of the Day.  An empty vec is allowed.
    /// - `oper_password`: Plaintext password for the OPER command.
    ///   An empty string disables oper promotion.
    ///
    /// # Example
    ///
    /// ```
    /// use irc_server::IRCServer;
    ///
    /// let server = IRCServer::new(
    ///     "irc.example.com",
    ///     vec!["Welcome to our network!".to_string()],
    ///     "secret",
    /// );
    /// ```
    pub fn new(server_name: &str, motd: Vec<String>, oper_password: &str) -> Self {
        IRCServer {
            server_name: server_name.to_string(),
            version: "1.0".to_string(),
            motd,
            oper_password: oper_password.to_string(),
            clients: HashMap::new(),
            channels: HashMap::new(),
            nicks: HashMap::new(),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Public API
    // ─────────────────────────────────────────────────────────────────────────

    /// Register a new TCP connection.
    ///
    /// Creates a `Client` record for the connection but does not send
    /// anything — IRC clients are expected to initiate registration by sending
    /// `CAP`, `NICK`, and `USER`.
    ///
    /// Returns an empty `Vec<Response>`; no messages are sent until the client speaks.
    pub fn on_connect(&mut self, conn_id: ConnId, host: &str) -> Vec<Response> {
        self.clients.insert(conn_id, Client::new(conn_id, host));
        vec![]
    }

    /// Dispatch an inbound IRC message and return the resulting responses.
    ///
    /// This is the central dispatch method.  It:
    ///
    /// 1. Looks up the client record for `conn_id`.
    /// 2. Routes the message to the appropriate handler based on `msg.command`.
    /// 3. Returns the list of `Response` values to send.
    ///
    /// If the client is unknown (which should not happen in normal usage, but
    /// might if the transport layer calls this after a disconnect) we return
    /// an empty vec rather than panicking.
    ///
    /// ## Pre-registration gate
    ///
    /// A client that has not yet completed the NICK+USER handshake may only
    /// send: `NICK`, `USER`, `CAP`, `QUIT`, and `PASS`.  Any other command
    /// gets `451 ERR_NOTREGISTERED`.
    pub fn on_message(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Check the client exists.
        if !self.clients.contains_key(&conn_id) {
            return vec![];
        }

        // Determine registration status.
        let registered = self.clients[&conn_id].registered;

        // Commands permitted before registration is complete.
        let pre_reg_allowed = ["NICK", "USER", "CAP", "QUIT", "PASS"];

        // Gate: reject post-registration commands from unregistered clients.
        if !registered && !pre_reg_allowed.contains(&msg.command.as_str()) {
            return vec![Response {
                conn_id,
                msg: self.make_msg(ERR_NOTREGISTERED, &["*", "You have not registered"]),
            }];
        }

        // Dispatch to the appropriate handler.
        match msg.command.as_str() {
            "NICK"    => self.handle_nick(conn_id, msg),
            "USER"    => self.handle_user(conn_id, msg),
            "CAP"     => self.handle_cap(conn_id, msg),
            "QUIT"    => self.handle_quit(conn_id, msg),
            "PASS"    => self.handle_pass(conn_id, msg),
            "JOIN"    => self.handle_join(conn_id, msg),
            "PART"    => self.handle_part(conn_id, msg),
            "PRIVMSG" => self.handle_privmsg(conn_id, msg),
            "NOTICE"  => self.handle_notice(conn_id, msg),
            "NAMES"   => self.handle_names(conn_id, msg),
            "LIST"    => self.handle_list(conn_id, msg),
            "TOPIC"   => self.handle_topic(conn_id, msg),
            "KICK"    => self.handle_kick(conn_id, msg),
            "INVITE"  => self.handle_invite(conn_id, msg),
            "MODE"    => self.handle_mode(conn_id, msg),
            "PING"    => self.handle_ping(conn_id, msg),
            "PONG"    => self.handle_pong(conn_id, msg),
            "AWAY"    => self.handle_away(conn_id, msg),
            "WHOIS"   => self.handle_whois(conn_id, msg),
            "WHO"     => self.handle_who(conn_id, msg),
            "OPER"    => self.handle_oper(conn_id, msg),
            _ => {
                let nick = self.clients[&conn_id].nick.clone().unwrap_or_else(|| "*".to_string());
                vec![Response {
                    conn_id,
                    msg: self.make_msg(ERR_UNKNOWNCOMMAND, &[&nick, &msg.command, "Unknown command"]),
                }]
            }
        }
    }

    /// Clean up state after a TCP connection closes.
    ///
    /// Called by the transport layer when it detects that a connection has been
    /// closed, either cleanly (after a QUIT) or unexpectedly (e.g. a network error).
    ///
    /// The cleanup procedure:
    /// 1. If the client has a nick, broadcast a QUIT message to all channel
    ///    members who share a channel with the disconnecting client.
    /// 2. Remove the client from every channel they were in.  Destroy any
    ///    channel that becomes empty.
    /// 3. Remove the client's nick from the nick index.
    /// 4. Remove the client record entirely.
    pub fn on_disconnect(&mut self, conn_id: ConnId) -> Vec<Response> {
        // Check the client exists (may have been cleaned up by QUIT already).
        if !self.clients.contains_key(&conn_id) {
            return vec![];
        }

        let mut responses = vec![];

        // Only registered clients (with a nick) get a quit broadcast.
        let (registered, nick_opt, mask) = {
            let client = &self.clients[&conn_id];
            (client.registered, client.nick.clone(), client.mask())
        };

        if registered {
            if let Some(_) = nick_opt {
                let quit_msg = Message {
                    prefix: Some(mask.clone()),
                    command: "QUIT".to_string(),
                    params: vec!["Connection closed".to_string()],
                };
                // Send to all unique channel members (excluding the quitting client).
                let peers = self.unique_channel_peers(conn_id);
                for peer_id in peers {
                    responses.push(Response {
                        conn_id: peer_id,
                        msg: quit_msg.clone(),
                    });
                }
            }
        }

        // Remove the client from every channel they were in.
        let channels_to_clean: Vec<String> = self.clients[&conn_id].channels.iter().cloned().collect();
        for chan_name in channels_to_clean {
            if let Some(channel) = self.channels.get_mut(&chan_name) {
                channel.members.remove(&conn_id);
                // Destroy the channel if it is now empty.
                if channel.members.is_empty() {
                    self.channels.remove(&chan_name);
                }
            }
        }

        // Remove from nick index.
        if let Some(nick) = self.clients[&conn_id].nick.clone() {
            self.nicks.remove(&nick.to_lowercase());
        }

        // Remove the client record.
        self.clients.remove(&conn_id);

        responses
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Private helpers
    // ─────────────────────────────────────────────────────────────────────────

    /// Build a Message whose prefix is the server name.
    ///
    /// Parameters that start with `:` are stripped of the leading colon
    /// (callers use colons for readability, but `Message.params` stores decoded
    /// values without the wire-format colon).
    fn make_msg(&self, command: &str, params: &[&str]) -> Message {
        let cleaned: Vec<String> = params.iter().map(|p| {
            if p.starts_with(':') { p[1..].to_string() } else { p.to_string() }
        }).collect();
        Message {
            prefix: Some(self.server_name.clone()),
            command: command.to_string(),
            params: cleaned,
        }
    }

    /// Build a server message addressed to the client's nick.
    ///
    /// Like `make_msg` but automatically inserts the client's nick (or `*`
    /// if not yet set) as the first parameter.  Most IRC numerics have the form:
    ///   `:server <numeric> <target_nick> <rest...>`
    fn client_msg(&self, conn_id: ConnId, command: &str, params: &[&str]) -> Message {
        let nick = self.clients[&conn_id].nick.clone().unwrap_or_else(|| "*".to_string());
        let mut all_params = vec![nick.as_str()];
        all_params.extend_from_slice(params);
        self.make_msg(command, &all_params)
    }

    /// Send the RFC 1459 welcome sequence to a newly-registered client.
    ///
    /// This sequence is sent exactly once, immediately after both NICK and
    /// USER have been received.  It consists of numerics 001–004 (the core
    /// welcome), 251 (LUSERCLIENT), and the MOTD block (375/372.../376).
    ///
    /// Numeric breakdown:
    /// - 001 — Personalised welcome message.
    /// - 002 — Which server the client is connected to and its version.
    /// - 003 — When the server was "created" (we always say "today").
    /// - 004 — Machine-readable server capabilities summary.
    /// - 251 — How many users are currently on the network.
    /// - 375 — MOTD header line.
    /// - 372 — One per MOTD line (may be zero lines).
    /// - 376 — MOTD footer line.
    fn welcome(&self, conn_id: ConnId) -> Vec<Response> {
        let client = &self.clients[&conn_id];
        let nick = client.nick.clone().unwrap_or_else(|| "*".to_string());
        let mask = client.mask();
        let host = &self.server_name;
        let ver = &self.version;

        // Count total registered users for the 251 numeric.
        let user_count = self.clients.values().filter(|c| c.registered).count();

        let mut responses = vec![
            Response {
                conn_id,
                msg: self.client_msg(conn_id, RPL_WELCOME,
                    &[&format!("Welcome to the IRC Network, {}", mask)]),
            },
            Response {
                conn_id,
                msg: self.client_msg(conn_id, RPL_YOURHOST,
                    &[&format!("Your host is {}, running version {}", host, ver)]),
            },
            Response {
                conn_id,
                msg: self.client_msg(conn_id, RPL_CREATED,
                    &["This server was created today"]),
            },
            Response {
                conn_id,
                msg: self.make_msg(RPL_MYINFO, &[&nick, host, ver, "o", "o"]),
            },
            Response {
                conn_id,
                msg: self.client_msg(conn_id, RPL_LUSERCLIENT,
                    &[&format!("There are {} users on 1 server", user_count)]),
            },
            // MOTD header
            Response {
                conn_id,
                msg: self.client_msg(conn_id, RPL_MOTDSTART,
                    &[&format!("- {} Message of the Day -", host)]),
            },
        ];

        // One 372 line per MOTD line (may be zero).
        for line in &self.motd {
            responses.push(Response {
                conn_id,
                msg: self.client_msg(conn_id, RPL_MOTD, &[&format!("- {} -", line)]),
            });
        }

        // MOTD footer.
        responses.push(Response {
            conn_id,
            msg: self.client_msg(conn_id, RPL_ENDOFMOTD, &["End of /MOTD command."]),
        });

        responses
    }

    /// Build 353 (NAMREPLY) + 366 (ENDOFNAMES) responses for a channel.
    ///
    /// The 353 line lists all visible members of the channel.  Each member's
    /// nick is prefixed with `@` if they are a channel operator or `+` if
    /// they have voice (but are not an operator).  Regular members have no prefix.
    ///
    /// Example 353 payload:
    ///   `= #general :@alice bob +carol`
    ///
    /// The `=` token indicates a public channel.
    fn names_responses(&self, channel: &Channel, requesting_nick: &str, requesting_conn: ConnId) -> Vec<Response> {
        // Build the space-separated list of prefixed nicks.
        let mut names_parts: Vec<String> = Vec::new();
        for (member_conn_id, member) in &channel.members {
            if let Some(client) = self.clients.get(member_conn_id) {
                if let Some(ref nick) = client.nick {
                    if member.is_operator {
                        names_parts.push(format!("@{}", nick));
                    } else if member.has_voice {
                        names_parts.push(format!("+{}", nick));
                    } else {
                        names_parts.push(nick.clone());
                    }
                }
            }
        }

        let names_str = names_parts.join(" ");

        vec![
            Response {
                conn_id: requesting_conn,
                msg: self.make_msg(RPL_NAMREPLY,
                    &[requesting_nick, "=", &channel.name, &names_str]),
            },
            Response {
                conn_id: requesting_conn,
                msg: self.make_msg(RPL_ENDOFNAMES,
                    &[requesting_nick, &channel.name, "End of /NAMES list"]),
            },
        ]
    }

    /// Return the set of ConnIds that share at least one channel with the client.
    ///
    /// This is used when broadcasting a QUIT or NICK-change: we need to reach
    /// every other client who can "see" this client exactly once, even if they
    /// share multiple channels.
    ///
    /// The client itself is excluded from the returned set.
    fn unique_channel_peers(&self, conn_id: ConnId) -> HashSet<ConnId> {
        let mut peers = HashSet::new();
        if let Some(client) = self.clients.get(&conn_id) {
            for chan_name in &client.channels {
                if let Some(channel) = self.channels.get(chan_name) {
                    for &peer_id in channel.members.keys() {
                        if peer_id != conn_id {
                            peers.insert(peer_id);
                        }
                    }
                }
            }
        }
        peers
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Command handlers
    // ─────────────────────────────────────────────────────────────────────────

    fn handle_cap(&mut self, conn_id: ConnId, _msg: &Message) -> Vec<Response> {
        // Handle the CAP (Capability Negotiation) command.
        //
        // Modern IRC clients send `CAP LS` at the start of a connection to
        // discover server capabilities before sending NICK/USER.  We do not
        // implement capability negotiation, so we acknowledge all CAP requests
        // with an empty ACK and move on.
        //
        // A real server would enumerate its supported capabilities here
        // (e.g., `multi-prefix`, `sasl`, `away-notify`).  For this v1
        // implementation we keep it simple: advertise nothing, accept everything.
        vec![Response {
            conn_id,
            msg: Message {
                prefix: Some(self.server_name.clone()),
                command: "CAP".to_string(),
                params: vec!["*".to_string(), "ACK".to_string(), String::new()],
            },
        }]
    }

    fn handle_pass(&mut self, _conn_id: ConnId, _msg: &Message) -> Vec<Response> {
        // PASS is sent before NICK/USER on servers that require a connection
        // password.  We do not enforce connection passwords in this v1, so we
        // accept and silently ignore PASS.
        vec![]
    }

    fn handle_nick(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Handle the NICK command — set or change a client's nickname.
        //
        // Pre-registration:
        //   The client is trying to set their nick for the first time.  We
        //   validate it, check uniqueness, store it, and — if USER has already
        //   been received — trigger the welcome sequence.
        //
        // Post-registration (nick change):
        //   Broadcast `:old!user@host NICK new` to all clients who share a
        //   channel with the nick-changer, then update the nick index.

        // ── Validate params ───────────────────────────────────────────────────
        if msg.params.is_empty() {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NONICKNAMEGIVEN, &["No nickname given"]),
            }];
        }

        let new_nick = msg.params[0].clone();

        // ── Validate nick format ──────────────────────────────────────────────
        if !valid_nick(&new_nick) {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_ERRONEUSNICKNAME,
                    &[&new_nick, "Erroneous nickname"]),
            }];
        }

        // ── Check uniqueness (case-insensitive) ───────────────────────────────
        if let Some(&existing) = self.nicks.get(&new_nick.to_lowercase()) {
            if existing != conn_id {
                // The nick is taken by a *different* client.
                return vec![Response {
                    conn_id,
                    msg: self.client_msg(conn_id, ERR_NICKNAMEINUSE,
                        &[&new_nick, "Nickname is already in use"]),
                }];
            }
        }

        // ── Apply the nick change ─────────────────────────────────────────────
        let old_nick = self.clients[&conn_id].nick.clone();
        let old_mask = self.clients[&conn_id].mask();
        let was_registered = self.clients[&conn_id].registered;
        let has_username = self.clients[&conn_id].username.is_some();

        // Remove old nick from the index (if any).
        if let Some(ref on) = old_nick {
            self.nicks.remove(&on.to_lowercase());
        }

        // Register the new nick.
        self.clients.get_mut(&conn_id).unwrap().nick = Some(new_nick.clone());
        self.nicks.insert(new_nick.to_lowercase(), conn_id);

        // ── Post-registration: broadcast NICK change to peers ─────────────────
        if was_registered && old_nick.is_some() {
            let nick_change_msg = Message {
                prefix: Some(old_mask),
                command: "NICK".to_string(),
                params: vec![new_nick],
            };
            let mut responses = vec![Response {
                conn_id,
                msg: nick_change_msg.clone(),
            }];
            // Notify all channel peers (unique, excluding client already added).
            let peers = self.unique_channel_peers(conn_id);
            for peer_id in peers {
                responses.push(Response {
                    conn_id: peer_id,
                    msg: nick_change_msg.clone(),
                });
            }
            return responses;
        }

        // ── Pre-registration: check if USER already done → welcome ────────────
        if has_username {
            self.clients.get_mut(&conn_id).unwrap().registered = true;
            return self.welcome(conn_id);
        }

        // NICK stored; waiting for USER — send nothing yet.
        vec![]
    }

    fn handle_user(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Handle the USER command — supply username and real name.
        //
        // Syntax: `USER <username> <mode> <unused> :<realname>`
        //
        // The second and third parameters (mode flags and "unused") are accepted
        // and discarded — most servers ignore them on initial registration.
        //
        // After successfully storing username/realname, if NICK has already been
        // received, we trigger the welcome sequence and mark the client as
        // registered.

        if self.clients[&conn_id].registered {
            // Already registered — ignore duplicate USER.
            return vec![];
        }

        // We need at least 4 params: username, mode, unused, realname.
        if msg.params.len() < 4 {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NEEDMOREPARAMS,
                    &["USER", "Not enough parameters"]),
            }];
        }

        let username = msg.params[0].clone();
        // params[1] is mode flags (e.g. "0" or "8"), params[2] is "*" — both ignored.
        let realname = msg.params[3].clone();

        let has_nick = self.clients[&conn_id].nick.is_some();

        {
            let client = self.clients.get_mut(&conn_id).unwrap();
            client.username = Some(username);
            client.realname = Some(realname);
        }

        // If NICK has already been received, complete the registration handshake.
        if has_nick {
            self.clients.get_mut(&conn_id).unwrap().registered = true;
            return self.welcome(conn_id);
        }

        // USER stored; waiting for NICK — send nothing yet.
        vec![]
    }

    fn handle_quit(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Handle the QUIT command — graceful client disconnect.
        //
        // The client is telling us they are leaving.  We:
        // 1. Broadcast a QUIT message to all channel peers.
        // 2. Send an ERROR message to the quitting client (RFC 1459 §4.1.6).
        // 3. Clean up state (without calling on_disconnect to avoid double-broadcast).

        let quit_reason = msg.params.first().cloned().unwrap_or_else(|| "Quit".to_string());

        let mut responses = vec![];

        // Broadcast QUIT to channel peers.
        {
            let client = &self.clients[&conn_id];
            if client.registered && client.nick.is_some() {
                let quit_broadcast = Message {
                    prefix: Some(client.mask()),
                    command: "QUIT".to_string(),
                    params: vec![quit_reason.clone()],
                };
                let peers = self.unique_channel_peers(conn_id);
                for peer_id in peers {
                    responses.push(Response {
                        conn_id: peer_id,
                        msg: quit_broadcast.clone(),
                    });
                }
            }
        }

        // Send ERROR to the quitting client as a farewell.
        let hostname = self.clients[&conn_id].hostname.clone();
        responses.push(Response {
            conn_id,
            msg: Message {
                prefix: None,
                command: "ERROR".to_string(),
                params: vec![format!("Closing Link: {} (Quit: {})", hostname, quit_reason)],
            },
        });

        // Clean up state — do this manually rather than calling on_disconnect so we
        // don't double-broadcast the QUIT.
        let channel_names: Vec<String> = self.clients[&conn_id].channels.iter().cloned().collect();
        for chan_name in channel_names {
            if let Some(channel) = self.channels.get_mut(&chan_name) {
                channel.members.remove(&conn_id);
                if channel.members.is_empty() {
                    self.channels.remove(&chan_name);
                }
            }
        }

        if let Some(nick) = self.clients[&conn_id].nick.clone() {
            self.nicks.remove(&nick.to_lowercase());
        }
        self.clients.remove(&conn_id);

        responses
    }

    fn handle_join(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Handle the JOIN command — add a client to a channel.
        //
        // Syntax: `JOIN <#channel>[,<#channel2>...]`
        //
        // When joining a channel:
        // - If the channel does not exist, create it.  The first member becomes
        //   the channel operator automatically.
        // - If the channel already exists, add the client as a regular member.
        // - Broadcast `:nick!user@host JOIN #channel` to ALL members.
        // - Send NAMES (353 + 366) to the joiner.

        if msg.params.is_empty() {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NEEDMOREPARAMS,
                    &["JOIN", "Not enough parameters"]),
            }];
        }

        let chan_names: Vec<String> = msg.params[0].split(',').map(|s| s.to_string()).collect();
        let mut responses = vec![];

        for chan_name_raw in chan_names {
            let chan_name = chan_name_raw.to_lowercase();

            // Skip if already in channel.
            if self.clients[&conn_id].channels.contains(&chan_name) {
                continue;
            }

            // Create the channel if it does not exist yet.
            if !self.channels.contains_key(&chan_name) {
                self.channels.insert(chan_name.clone(), Channel::new(&chan_name));
            }

            // Add the client to the channel.  The first member is the operator.
            let is_first_member = self.channels[&chan_name].members.is_empty();
            self.channels.get_mut(&chan_name).unwrap().members.insert(
                conn_id,
                ChannelMember { is_operator: is_first_member, has_voice: false },
            );
            self.clients.get_mut(&conn_id).unwrap().channels.insert(chan_name.clone());

            // Broadcast JOIN to all current members (including the joiner).
            let client_mask = self.clients[&conn_id].mask();
            let join_msg = Message {
                prefix: Some(client_mask),
                command: "JOIN".to_string(),
                params: vec![chan_name.clone()],
            };
            let member_ids: Vec<ConnId> = self.channels[&chan_name].members.keys().cloned().collect();
            for member_conn_id in member_ids {
                responses.push(Response {
                    conn_id: member_conn_id,
                    msg: join_msg.clone(),
                });
            }

            // Send the current topic if one is set.
            let nick = self.clients[&conn_id].nick.clone().unwrap_or_else(|| "*".to_string());
            let topic = self.channels[&chan_name].topic.clone();
            if !topic.is_empty() {
                responses.push(Response {
                    conn_id,
                    msg: self.make_msg(RPL_TOPIC, &[&nick, &chan_name, &topic]),
                });
            } else {
                responses.push(Response {
                    conn_id,
                    msg: self.make_msg(RPL_NOTOPIC, &[&nick, &chan_name, "No topic is set"]),
                });
            }

            // Send NAMES (353 + 366) to the joiner.
            let channel = self.channels[&chan_name].clone();
            let nick_str = nick.clone();
            responses.extend(self.names_responses(&channel, &nick_str, conn_id));
        }

        responses
    }

    fn handle_part(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Handle the PART command — remove a client from a channel.
        //
        // Syntax: `PART <#channel> [:<message>]`
        //
        // The optional part message is relayed in the PART broadcast.
        // After removing the client, if the channel is now empty it is destroyed.

        if msg.params.is_empty() {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NEEDMOREPARAMS,
                    &["PART", "Not enough parameters"]),
            }];
        }

        let chan_name = msg.params[0].to_lowercase();
        let part_msg_text = msg.params.get(1).cloned().unwrap_or_else(|| {
            self.clients[&conn_id].nick.clone().unwrap_or_default()
        });

        // Check the client is actually in this channel.
        if !self.clients[&conn_id].channels.contains(&chan_name) {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NOTONCHANNEL,
                    &[&chan_name, "You're not on that channel"]),
            }];
        }

        if !self.channels.contains_key(&chan_name) {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NOTONCHANNEL,
                    &[&chan_name, "You're not on that channel"]),
            }];
        }

        // Build the PART message *before* removing the client from the channel
        // so that they still appear in the member list and receive their own PART.
        let client_mask = self.clients[&conn_id].mask();
        let part_broadcast = Message {
            prefix: Some(client_mask),
            command: "PART".to_string(),
            params: vec![chan_name.clone(), part_msg_text],
        };

        // Collect member IDs *before* removing the client.
        let member_ids: Vec<ConnId> = self.channels[&chan_name].members.keys().cloned().collect();

        // Remove the client from the channel.
        self.channels.get_mut(&chan_name).unwrap().members.remove(&conn_id);
        self.clients.get_mut(&conn_id).unwrap().channels.remove(&chan_name);

        // Broadcast PART to all former members (including the departing client).
        let responses: Vec<Response> = member_ids.into_iter().map(|mid| Response {
            conn_id: mid,
            msg: part_broadcast.clone(),
        }).collect();

        // Destroy the channel if it is now empty.
        if self.channels[&chan_name].members.is_empty() {
            self.channels.remove(&chan_name);
        }

        responses
    }

    fn deliver_message(&mut self, conn_id: ConnId, msg: &Message, command: &str) -> Vec<Response> {
        // Common logic for PRIVMSG and NOTICE delivery.
        //
        // Both commands use the same delivery mechanics:
        // - Target is either a channel (starts with `#`) or a nick.
        // - For channels: deliver to all members except the sender.
        // - For nicks: deliver directly to the target client.

        if msg.params.is_empty() {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NEEDMOREPARAMS,
                    &[command, "No recipient given"]),
            }];
        }

        let target = msg.params[0].clone();
        let text = msg.params.get(1).cloned().unwrap_or_default();

        if text.is_empty() {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, "412", &["No text to send"]),
            }];
        }

        let client_mask = self.clients[&conn_id].mask();
        let out_msg = Message {
            prefix: Some(client_mask),
            command: command.to_string(),
            params: vec![target.clone(), text],
        };

        let mut responses = vec![];

        if target.starts_with('#') {
            // ── Channel message ───────────────────────────────────────────────
            let chan_name = target.to_lowercase();
            if !self.channels.contains_key(&chan_name) {
                return vec![Response {
                    conn_id,
                    msg: self.client_msg(conn_id, ERR_NOSUCHCHANNEL,
                        &[&target, "No such channel"]),
                }];
            }
            // Deliver to all members except the sender.
            let targets: Vec<ConnId> = self.channels[&chan_name].members.keys()
                .filter(|&&id| id != conn_id)
                .cloned().collect();
            for target_id in targets {
                responses.push(Response { conn_id: target_id, msg: out_msg.clone() });
            }
        } else {
            // ── Private message to a nick ─────────────────────────────────────
            let target_conn_opt = self.nicks.get(&target.to_lowercase()).cloned();
            match target_conn_opt {
                None => {
                    return vec![Response {
                        conn_id,
                        msg: self.client_msg(conn_id, ERR_NOSUCHNICK,
                            &[&target, "No such nick/channel"]),
                    }];
                }
                Some(target_conn) => {
                    if !self.clients.contains_key(&target_conn) {
                        return vec![Response {
                            conn_id,
                            msg: self.client_msg(conn_id, ERR_NOSUCHNICK,
                                &[&target, "No such nick/channel"]),
                        }];
                    }
                    responses.push(Response { conn_id: target_conn, msg: out_msg });
                    // For PRIVMSG (not NOTICE), if the target is away, inform the sender.
                    if command == "PRIVMSG" {
                        if let Some(away_msg) = self.clients[&target_conn].away_message.clone() {
                            let target_nick = self.clients[&target_conn].nick.clone()
                                .unwrap_or_else(|| target.clone());
                            responses.push(Response {
                                conn_id,
                                msg: self.client_msg(conn_id, RPL_AWAY,
                                    &[&target_nick, &away_msg]),
                            });
                        }
                    }
                }
            }
        }

        responses
    }

    fn handle_privmsg(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        self.deliver_message(conn_id, msg, "PRIVMSG")
    }

    fn handle_notice(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        self.deliver_message(conn_id, msg, "NOTICE")
    }

    fn handle_names(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Handle the NAMES command — list members of a channel.
        //
        // Returns 353 (NAMREPLY) + 366 (ENDOFNAMES) for the requested channel.
        // If no channel is specified, we return NAMES for all channels.

        let nick = self.clients[&conn_id].nick.clone().unwrap_or_else(|| "*".to_string());

        if !msg.params.is_empty() {
            let chan_name = msg.params[0].to_lowercase();
            if let Some(channel) = self.channels.get(&chan_name).cloned() {
                return self.names_responses(&channel, &nick, conn_id);
            } else {
                // Channel not found — send just the terminator.
                return vec![Response {
                    conn_id,
                    msg: self.make_msg(RPL_ENDOFNAMES, &[&nick, &chan_name, "End of /NAMES list"]),
                }];
            }
        }

        // No channel specified — send NAMES for all channels.
        let channel_names: Vec<String> = self.channels.keys().cloned().collect();
        let mut responses = vec![];
        for chan_name in channel_names {
            if let Some(channel) = self.channels.get(&chan_name).cloned() {
                responses.extend(self.names_responses(&channel, &nick, conn_id));
            }
        }
        responses
    }

    fn handle_list(&mut self, conn_id: ConnId, _msg: &Message) -> Vec<Response> {
        // Handle the LIST command — enumerate all channels.
        //
        // Returns:
        //   321 — RPL_LISTSTART header.
        //   322 — RPL_LIST, one per channel: name, member count, topic.
        //   323 — RPL_LISTEND terminator.

        let nick = self.clients[&conn_id].nick.clone().unwrap_or_else(|| "*".to_string());
        let mut responses = vec![Response {
            conn_id,
            msg: self.make_msg(RPL_LISTSTART, &[&nick, "Channel", "Users  Name"]),
        }];

        let channel_data: Vec<(String, usize, String)> = self.channels.values()
            .map(|ch| (ch.name.clone(), ch.members.len(), ch.topic.clone()))
            .collect();

        for (name, member_count, topic) in channel_data {
            responses.push(Response {
                conn_id,
                msg: self.make_msg(RPL_LIST, &[&nick, &name, &member_count.to_string(), &topic]),
            });
        }

        responses.push(Response {
            conn_id,
            msg: self.make_msg(RPL_LISTEND, &[&nick, "End of /LIST"]),
        });
        responses
    }

    fn handle_topic(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Handle the TOPIC command — get or set a channel's topic.
        //
        // Syntax:
        //   `TOPIC <#channel>`           → query the current topic.
        //   `TOPIC <#channel> :<topic>`  → set a new topic.

        if msg.params.is_empty() {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NEEDMOREPARAMS,
                    &["TOPIC", "Not enough parameters"]),
            }];
        }

        let chan_name = msg.params[0].to_lowercase();
        let nick = self.clients[&conn_id].nick.clone().unwrap_or_else(|| "*".to_string());

        if !self.channels.contains_key(&chan_name) {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NOSUCHCHANNEL,
                    &[&chan_name, "No such channel"]),
            }];
        }

        // Check the client is in the channel.
        if !self.channels[&chan_name].members.contains_key(&conn_id) {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NOTONCHANNEL,
                    &[&chan_name, "You're not on that channel"]),
            }];
        }

        if msg.params.len() < 2 {
            // ── Query mode ────────────────────────────────────────────────────
            let topic = self.channels[&chan_name].topic.clone();
            if !topic.is_empty() {
                return vec![Response {
                    conn_id,
                    msg: self.make_msg(RPL_TOPIC, &[&nick, &chan_name, &topic]),
                }];
            } else {
                return vec![Response {
                    conn_id,
                    msg: self.make_msg(RPL_NOTOPIC, &[&nick, &chan_name, "No topic is set"]),
                }];
            }
        }

        // ── Set mode ──────────────────────────────────────────────────────────
        let new_topic = msg.params[1].clone();
        self.channels.get_mut(&chan_name).unwrap().topic = new_topic.clone();

        // Broadcast the new topic to all channel members.
        let client_mask = self.clients[&conn_id].mask();
        let topic_broadcast = Message {
            prefix: Some(client_mask),
            command: "TOPIC".to_string(),
            params: vec![chan_name.clone(), new_topic],
        };
        let member_ids: Vec<ConnId> = self.channels[&chan_name].members.keys().cloned().collect();
        member_ids.into_iter().map(|mid| Response { conn_id: mid, msg: topic_broadcast.clone() }).collect()
    }

    fn handle_kick(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Handle the KICK command — remove a member from a channel (op only).
        //
        // Syntax: `KICK <#channel> <nick> [:<reason>]`

        if msg.params.len() < 2 {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NEEDMOREPARAMS,
                    &["KICK", "Not enough parameters"]),
            }];
        }

        let chan_name = msg.params[0].to_lowercase();
        let target_nick = msg.params[1].clone();
        let reason = msg.params.get(2).cloned().unwrap_or_else(|| {
            self.clients[&conn_id].nick.clone().unwrap_or_default()
        });

        if !self.channels.contains_key(&chan_name) {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NOSUCHCHANNEL,
                    &[&chan_name, "No such channel"]),
            }];
        }

        // Verify the kicker is in the channel.
        let kicker_is_op = self.channels[&chan_name].members.get(&conn_id)
            .map(|m| m.is_operator)
            .unwrap_or(false);

        if !self.channels[&chan_name].members.contains_key(&conn_id) {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NOTONCHANNEL,
                    &[&chan_name, "You're not on that channel"]),
            }];
        }

        // Verify the kicker has operator privileges.
        if !kicker_is_op {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_CHANOPRIVSNEEDED,
                    &[&chan_name, "You're not channel operator"]),
            }];
        }

        // Find the target nick in the channel.
        let target_conn = self.nicks.get(&target_nick.to_lowercase()).cloned();
        let target_conn = match target_conn {
            Some(tc) if self.channels[&chan_name].members.contains_key(&tc) => tc,
            _ => {
                return vec![Response {
                    conn_id,
                    msg: self.client_msg(conn_id, ERR_USERNOTINCHANNEL,
                        &[&target_nick, &chan_name, "They aren't on that channel"]),
                }];
            }
        };

        // Broadcast KICK to all current members (before removing the victim).
        let client_mask = self.clients[&conn_id].mask();
        let kick_broadcast = Message {
            prefix: Some(client_mask),
            command: "KICK".to_string(),
            params: vec![chan_name.clone(), target_nick, reason],
        };
        let responses: Vec<Response> = self.channels[&chan_name].members.keys()
            .cloned()
            .map(|mid| Response { conn_id: mid, msg: kick_broadcast.clone() })
            .collect();

        // Remove victim from the channel.
        self.channels.get_mut(&chan_name).unwrap().members.remove(&target_conn);
        self.clients.get_mut(&target_conn).unwrap().channels.remove(&chan_name);

        // Destroy channel if empty.
        if self.channels[&chan_name].members.is_empty() {
            self.channels.remove(&chan_name);
        }

        responses
    }

    fn handle_invite(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Handle the INVITE command — invite a nick to a channel.
        //
        // Syntax: `INVITE <nick> <#channel>`

        if msg.params.len() < 2 {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NEEDMOREPARAMS,
                    &["INVITE", "Not enough parameters"]),
            }];
        }

        let target_nick = msg.params[0].clone();
        let chan_name = msg.params[1].to_lowercase();
        let nick = self.clients[&conn_id].nick.clone().unwrap_or_else(|| "*".to_string());

        // Find the target.
        let target_conn = self.nicks.get(&target_nick.to_lowercase()).cloned();
        let target_conn = match target_conn {
            None => {
                return vec![Response {
                    conn_id,
                    msg: self.client_msg(conn_id, ERR_NOSUCHNICK,
                        &[&target_nick, "No such nick/channel"]),
                }];
            }
            Some(tc) => tc,
        };

        let client_mask = self.clients[&conn_id].mask();

        // Confirmation to the inviter.
        let mut responses = vec![Response {
            conn_id,
            msg: self.make_msg(RPL_INVITING, &[&nick, &chan_name, &target_nick]),
        }];

        // Send INVITE to the target.
        responses.push(Response {
            conn_id: target_conn,
            msg: Message {
                prefix: Some(client_mask),
                command: "INVITE".to_string(),
                params: vec![target_nick, chan_name],
            },
        });

        responses
    }

    fn handle_mode(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Handle the MODE command — query or set channel/user modes.
        //
        // This v1 implementation supports:
        // - `MODE #channel`       → 324 RPL_CHANNELMODEIS (current channel modes).
        // - `MODE nick`           → 221 RPL_UMODEIS (current user modes).
        // - `MODE #channel +/-X`  → acknowledge with a MODE broadcast.
        // - `MODE nick +/-X`      → acknowledge with a MODE broadcast.

        if msg.params.is_empty() {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NEEDMOREPARAMS,
                    &["MODE", "Not enough parameters"]),
            }];
        }

        let target = msg.params[0].clone();
        let nick = self.clients[&conn_id].nick.clone().unwrap_or_else(|| "*".to_string());

        if target.starts_with('#') {
            // ── Channel MODE ──────────────────────────────────────────────────
            let chan_name = target.to_lowercase();
            if !self.channels.contains_key(&chan_name) {
                return vec![Response {
                    conn_id,
                    msg: self.client_msg(conn_id, ERR_NOSUCHCHANNEL,
                        &[&chan_name, "No such channel"]),
                }];
            }

            if msg.params.len() == 1 {
                // Query: return current channel modes.
                let modes = &self.channels[&chan_name].modes;
                let mode_str = if modes.is_empty() {
                    "+".to_string()
                } else {
                    let mut sorted: Vec<char> = modes.iter().cloned().collect();
                    sorted.sort_unstable();
                    format!("+{}", sorted.iter().collect::<String>())
                };
                return vec![Response {
                    conn_id,
                    msg: self.make_msg(RPL_CHANNELMODEIS, &[&nick, &chan_name, &mode_str]),
                }];
            } else {
                // Set: acknowledge by broadcasting MODE to channel members.
                let mode_str = msg.params[1].clone();
                // Apply simple single-char modes (no parameters in v1).
                if mode_str.starts_with('+') {
                    for ch in mode_str[1..].chars() {
                        self.channels.get_mut(&chan_name).unwrap().modes.insert(ch);
                    }
                } else if mode_str.starts_with('-') {
                    for ch in mode_str[1..].chars() {
                        self.channels.get_mut(&chan_name).unwrap().modes.remove(&ch);
                    }
                }

                let client_mask = self.clients[&conn_id].mask();
                let mode_broadcast = Message {
                    prefix: Some(client_mask),
                    command: "MODE".to_string(),
                    params: vec![chan_name.clone(), mode_str],
                };
                return self.channels[&chan_name].members.keys()
                    .cloned()
                    .map(|mid| Response { conn_id: mid, msg: mode_broadcast.clone() })
                    .collect();
            }
        } else {
            // ── User MODE ─────────────────────────────────────────────────────
            if msg.params.len() == 1 {
                // Query: return user modes.  We don't track user modes in v1.
                return vec![Response {
                    conn_id,
                    msg: self.make_msg("221", &[&nick, "+"]),
                }];
            } else {
                // Set user mode: acknowledge.
                let mode_str = msg.params[1].clone();
                let client_mask = self.clients[&conn_id].mask();
                let mode_broadcast = Message {
                    prefix: Some(client_mask),
                    command: "MODE".to_string(),
                    params: vec![target, mode_str],
                };
                return vec![Response { conn_id, msg: mode_broadcast }];
            }
        }
    }

    fn handle_ping(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Handle the PING command — keepalive from client to server.
        //
        // The client sends PING periodically to verify the connection is alive.
        // We respond with a matching PONG.  The PONG carries the same server
        // token the client sent, which lets the client match the response.
        let server_token = msg.params.first().cloned()
            .unwrap_or_else(|| self.server_name.clone());
        vec![Response {
            conn_id,
            msg: Message {
                prefix: Some(self.server_name.clone()),
                command: "PONG".to_string(),
                params: vec![self.server_name.clone(), server_token],
            },
        }]
    }

    fn handle_pong(&mut self, _conn_id: ConnId, _msg: &Message) -> Vec<Response> {
        // Handle the PONG command — client's response to a server PING.
        //
        // Servers send PING to verify clients are still alive; clients reply
        // with PONG.  We don't send server-initiated PINGs in v1, so we simply
        // ignore any PONG we receive from a client.
        vec![]
    }

    fn handle_away(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Handle the AWAY command — set or clear away status.
        //
        // Syntax:
        //   `AWAY :<message>`  — mark as away.
        //   `AWAY`             — clear away status.
        if msg.params.first().map(|p| !p.is_empty()).unwrap_or(false) {
            let away_text = msg.params[0].clone();
            self.clients.get_mut(&conn_id).unwrap().away_message = Some(away_text);
            vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, RPL_NOWAWAY,
                    &["You have been marked as being away"]),
            }]
        } else {
            self.clients.get_mut(&conn_id).unwrap().away_message = None;
            vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, RPL_UNAWAY,
                    &["You are no longer marked as being away"]),
            }]
        }
    }

    fn handle_whois(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Handle the WHOIS command — retrieve information about a nick.
        //
        // Returns: 311 (WHOISUSER), 312 (WHOISSERVER), 319 (WHOISCHANNELS),
        //          301 (AWAY if applicable), 318 (ENDOFWHOIS).

        if msg.params.is_empty() {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NEEDMOREPARAMS,
                    &["WHOIS", "Not enough parameters"]),
            }];
        }

        let target_nick = msg.params[0].clone();
        let nick = self.clients[&conn_id].nick.clone().unwrap_or_else(|| "*".to_string());

        let target_conn = match self.nicks.get(&target_nick.to_lowercase()).cloned() {
            None => {
                return vec![Response {
                    conn_id,
                    msg: self.client_msg(conn_id, ERR_NOSUCHNICK,
                        &[&target_nick, "No such nick/channel"]),
                }];
            }
            Some(tc) => tc,
        };

        if !self.clients.contains_key(&target_conn) {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NOSUCHNICK,
                    &[&target_nick, "No such nick/channel"]),
            }];
        }

        let target_nick_str = self.clients[&target_conn].nick.clone()
            .unwrap_or_else(|| target_nick.clone());
        let target_username = self.clients[&target_conn].username.clone()
            .unwrap_or_else(|| "*".to_string());
        let target_hostname = self.clients[&target_conn].hostname.clone();
        let target_realname = self.clients[&target_conn].realname.clone()
            .unwrap_or_default();
        let target_channels: Vec<String> = self.clients[&target_conn].channels.iter()
            .cloned().collect();
        let target_away = self.clients[&target_conn].away_message.clone();

        let mut responses = vec![
            // 311: nick user host * :realname
            Response {
                conn_id,
                msg: self.make_msg(RPL_WHOISUSER, &[
                    &nick, &target_nick_str, &target_username,
                    &target_hostname, "*", &target_realname,
                ]),
            },
            // 312: nick server :server info
            Response {
                conn_id,
                msg: self.make_msg(RPL_WHOISSERVER, &[
                    &nick, &target_nick_str, &self.server_name.clone(), "IRC server",
                ]),
            },
        ];

        // 319: channels the user is in.
        if !target_channels.is_empty() {
            let mut sorted_chans = target_channels;
            sorted_chans.sort();
            let chan_list = sorted_chans.join(" ");
            responses.push(Response {
                conn_id,
                msg: self.make_msg(RPL_WHOISCHANNELS, &[&nick, &target_nick_str, &chan_list]),
            });
        }

        // 301: away message if applicable.
        if let Some(away_msg) = target_away {
            responses.push(Response {
                conn_id,
                msg: self.make_msg(RPL_AWAY, &[&nick, &target_nick_str, &away_msg]),
            });
        }

        // 318: terminator.
        responses.push(Response {
            conn_id,
            msg: self.make_msg(RPL_ENDOFWHOIS, &[&nick, &target_nick_str, "End of /WHOIS list"]),
        });

        responses
    }

    fn handle_who(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Handle the WHO command — list users matching a mask.
        //
        // Syntax: `WHO [<mask>]`
        //
        // Returns 352 (WHOREPLY) rows followed by 315 (ENDOFWHO).
        // The 352 format is:
        //   `<channel> <user> <host> <server> <nick> H|G :<hopcount> <realname>`
        // H = here (not away), G = gone (away).

        let nick = self.clients[&conn_id].nick.clone().unwrap_or_else(|| "*".to_string());
        let mask = msg.params.first().cloned().unwrap_or_else(|| "*".to_string());
        let mut responses = vec![];
        let server_name = self.server_name.clone();

        if mask.starts_with('#') {
            // List members of the given channel.
            let chan_name = mask.to_lowercase();
            if let Some(channel) = self.channels.get(&chan_name) {
                let members: Vec<ConnId> = channel.members.keys().cloned().collect();
                for member_conn_id in members {
                    if let Some(target) = self.clients.get(&member_conn_id) {
                        let here_or_gone = if target.away_message.is_some() { "G" } else { "H" };
                        let target_nick = target.nick.clone().unwrap_or_else(|| "*".to_string());
                        let target_user = target.username.clone().unwrap_or_else(|| "*".to_string());
                        let target_host = target.hostname.clone();
                        let target_real = target.realname.clone().unwrap_or_default();
                        responses.push(Response {
                            conn_id,
                            msg: self.make_msg(RPL_WHOREPLY, &[
                                &nick, &chan_name, &target_user, &target_host,
                                &server_name, &target_nick, here_or_gone,
                                &format!("0 {}", target_real),
                            ]),
                        });
                    }
                }
            }
        } else {
            // List all registered clients.
            let client_ids: Vec<ConnId> = self.clients.keys().cloned().collect();
            for cid in client_ids {
                if let Some(target) = self.clients.get(&cid) {
                    if !target.registered { continue; }
                    let here_or_gone = if target.away_message.is_some() { "G" } else { "H" };
                    let target_nick = target.nick.clone().unwrap_or_else(|| "*".to_string());
                    let target_user = target.username.clone().unwrap_or_else(|| "*".to_string());
                    let target_host = target.hostname.clone();
                    let target_real = target.realname.clone().unwrap_or_default();
                    responses.push(Response {
                        conn_id,
                        msg: self.make_msg(RPL_WHOREPLY, &[
                            &nick, "*", &target_user, &target_host,
                            &server_name, &target_nick, here_or_gone,
                            &format!("0 {}", target_real),
                        ]),
                    });
                }
            }
        }

        responses.push(Response {
            conn_id,
            msg: self.make_msg(RPL_ENDOFWHO, &[&nick, &mask, "End of /WHO list"]),
        });
        responses
    }

    fn handle_oper(&mut self, conn_id: ConnId, msg: &Message) -> Vec<Response> {
        // Handle the OPER command — gain IRC operator privileges.
        //
        // Syntax: `OPER <name> <password>`
        //
        // If the supplied password matches the configured oper password,
        // the client's `is_oper` flag is set and they receive 381 RPL_YOUREOPER.

        if msg.params.len() < 2 {
            return vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_NEEDMOREPARAMS,
                    &["OPER", "Not enough parameters"]),
            }];
        }

        // We ignore the name (params[0]); only the password matters in v1.
        let password = msg.params[1].clone();

        if !self.oper_password.is_empty() && password == self.oper_password {
            self.clients.get_mut(&conn_id).unwrap().is_oper = true;
            vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, RPL_YOUREOPER,
                    &["You are now an IRC operator"]),
            }]
        } else {
            vec![Response {
                conn_id,
                msg: self.client_msg(conn_id, ERR_PASSWDMISMATCH, &["Password incorrect"]),
            }]
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use irc_proto::parse;

    /// Helper: create a fresh server for each test.
    fn make_server() -> IRCServer {
        IRCServer::new("irc.test", vec!["Test MOTD".to_string()], "operpass")
    }

    /// Helper: register a client fully (NICK + USER → welcome sequence).
    fn register_client(server: &mut IRCServer, conn_id: ConnId, nick: &str) {
        server.on_connect(conn_id, "127.0.0.1");
        let nick_msg = parse(&format!("NICK {}", nick)).unwrap();
        server.on_message(conn_id, &nick_msg);
        let user_msg = parse("USER test 0 * :Test User").unwrap();
        server.on_message(conn_id, &user_msg);
    }

    // ── nick validation ───────────────────────────────────────────────────────

    #[test]
    fn test_valid_nick_simple() {
        assert!(valid_nick("alice"));
        assert!(valid_nick("Bob"));
        assert!(valid_nick("_bot"));
        assert!(valid_nick("[ninja]"));
    }

    #[test]
    fn test_valid_nick_max_length() {
        assert!(valid_nick("abcdefghi")); // 9 chars — at limit
        assert!(!valid_nick("abcdefghij")); // 10 chars — over limit
    }

    #[test]
    fn test_valid_nick_invalid() {
        assert!(!valid_nick("")); // empty
        assert!(!valid_nick("0abc")); // starts with digit
        assert!(!valid_nick("bad nick")); // space
        assert!(!valid_nick("bad.nick")); // dot
    }

    // ── on_connect ────────────────────────────────────────────────────────────

    #[test]
    fn test_on_connect_returns_empty() {
        let mut server = make_server();
        let responses = server.on_connect(ConnId(1), "127.0.0.1");
        assert!(responses.is_empty());
    }

    // ── NICK + USER registration ──────────────────────────────────────────────

    #[test]
    fn test_nick_then_user_triggers_welcome() {
        let mut server = make_server();
        server.on_connect(ConnId(1), "127.0.0.1");

        let nick_msg = parse("NICK alice").unwrap();
        let r1 = server.on_message(ConnId(1), &nick_msg);
        // NICK alone does not trigger welcome yet.
        assert!(r1.is_empty());

        let user_msg = parse("USER alice 0 * :Alice Smith").unwrap();
        let r2 = server.on_message(ConnId(1), &user_msg);
        // USER completes registration — welcome sequence fires.
        assert!(!r2.is_empty());
        // First message should be 001 RPL_WELCOME.
        assert_eq!(r2[0].msg.command, "001");
    }

    #[test]
    fn test_user_then_nick_triggers_welcome() {
        // USER before NICK is also valid — welcome fires when NICK arrives.
        let mut server = make_server();
        server.on_connect(ConnId(1), "127.0.0.1");

        let user_msg = parse("USER alice 0 * :Alice Smith").unwrap();
        let r1 = server.on_message(ConnId(1), &user_msg);
        assert!(r1.is_empty()); // No NICK yet.

        let nick_msg = parse("NICK alice").unwrap();
        let r2 = server.on_message(ConnId(1), &nick_msg);
        assert!(!r2.is_empty());
        assert_eq!(r2[0].msg.command, "001");
    }

    #[test]
    fn test_nick_already_in_use() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");

        server.on_connect(ConnId(2), "127.0.0.2");
        let nick_msg = parse("NICK alice").unwrap();
        let responses = server.on_message(ConnId(2), &nick_msg);
        assert_eq!(responses[0].msg.command, "433"); // ERR_NICKNAMEINUSE
    }

    #[test]
    fn test_invalid_nick_rejected() {
        let mut server = make_server();
        server.on_connect(ConnId(1), "127.0.0.1");

        let nick_msg = parse("NICK 0invalid").unwrap();
        let responses = server.on_message(ConnId(1), &nick_msg);
        assert_eq!(responses[0].msg.command, "432"); // ERR_ERRONEUSNICKNAME
    }

    // ── ERR_NOTREGISTERED gate ────────────────────────────────────────────────

    #[test]
    fn test_join_before_registration_rejected() {
        let mut server = make_server();
        server.on_connect(ConnId(1), "127.0.0.1");

        let join_msg = parse("JOIN #test").unwrap();
        let responses = server.on_message(ConnId(1), &join_msg);
        assert_eq!(responses[0].msg.command, "451"); // ERR_NOTREGISTERED
    }

    // ── JOIN ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_join_creates_channel_and_makes_operator() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");

        let join_msg = parse("JOIN #general").unwrap();
        let responses = server.on_message(ConnId(1), &join_msg);

        // Should receive JOIN + topic (or NOTOPIC) + NAMES
        let commands: Vec<&str> = responses.iter().map(|r| r.msg.command.as_str()).collect();
        assert!(commands.contains(&"JOIN"));
        assert!(commands.contains(&"353") || commands.contains(&"366"));
    }

    #[test]
    fn test_join_broadcasts_to_all_members() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");
        register_client(&mut server, ConnId(2), "bob");

        // Alice joins first.
        let join_msg = parse("JOIN #general").unwrap();
        server.on_message(ConnId(1), &join_msg);

        // Bob joins — both should receive Bob's JOIN message.
        let join_msg2 = parse("JOIN #general").unwrap();
        let responses = server.on_message(ConnId(2), &join_msg2);

        let join_responses: Vec<&Response> = responses.iter()
            .filter(|r| r.msg.command == "JOIN")
            .collect();
        // Both alice (ConnId 1) and bob (ConnId 2) should get the JOIN message.
        let targets: Vec<ConnId> = join_responses.iter().map(|r| r.conn_id).collect();
        assert!(targets.contains(&ConnId(1)));
        assert!(targets.contains(&ConnId(2)));
    }

    // ── PART ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_part_removes_client_from_channel() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");

        let join_msg = parse("JOIN #test").unwrap();
        server.on_message(ConnId(1), &join_msg);

        let part_msg = parse("PART #test :Goodbye").unwrap();
        let responses = server.on_message(ConnId(1), &part_msg);

        let commands: Vec<&str> = responses.iter().map(|r| r.msg.command.as_str()).collect();
        assert!(commands.contains(&"PART"));
    }

    #[test]
    fn test_part_not_on_channel() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");

        let part_msg = parse("PART #nonexistent").unwrap();
        let responses = server.on_message(ConnId(1), &part_msg);
        assert_eq!(responses[0].msg.command, "442"); // ERR_NOTONCHANNEL
    }

    // ── PRIVMSG ───────────────────────────────────────────────────────────────

    #[test]
    fn test_privmsg_to_channel() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");
        register_client(&mut server, ConnId(2), "bob");

        server.on_message(ConnId(1), &parse("JOIN #chat").unwrap());
        server.on_message(ConnId(2), &parse("JOIN #chat").unwrap());

        let msg = parse("PRIVMSG #chat :Hello everyone!").unwrap();
        let responses = server.on_message(ConnId(1), &msg);

        // Alice sends, so only Bob should receive it.
        assert!(responses.iter().all(|r| r.conn_id != ConnId(1)));
        assert!(responses.iter().any(|r| r.conn_id == ConnId(2)));
    }

    #[test]
    fn test_privmsg_to_nick() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");
        register_client(&mut server, ConnId(2), "bob");

        let msg = parse("PRIVMSG bob :Hey Bob!").unwrap();
        let responses = server.on_message(ConnId(1), &msg);

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].conn_id, ConnId(2));
        assert_eq!(responses[0].msg.command, "PRIVMSG");
    }

    #[test]
    fn test_privmsg_no_such_nick() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");

        let msg = parse("PRIVMSG nobody :Hello?").unwrap();
        let responses = server.on_message(ConnId(1), &msg);
        assert_eq!(responses[0].msg.command, "401"); // ERR_NOSUCHNICK
    }

    // ── QUIT ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_quit_cleans_up_client() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");

        let quit_msg = parse("QUIT :Goodbye").unwrap();
        server.on_message(ConnId(1), &quit_msg);

        // After quit, on_disconnect should be a no-op (already cleaned up).
        let responses = server.on_disconnect(ConnId(1));
        assert!(responses.is_empty());
    }

    #[test]
    fn test_quit_broadcasts_to_channel_peers() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");
        register_client(&mut server, ConnId(2), "bob");

        server.on_message(ConnId(1), &parse("JOIN #lobby").unwrap());
        server.on_message(ConnId(2), &parse("JOIN #lobby").unwrap());

        let quit_msg = parse("QUIT :Later").unwrap();
        let responses = server.on_message(ConnId(1), &quit_msg);

        // Bob should receive alice's QUIT.
        let quit_to_bob = responses.iter().any(|r| r.conn_id == ConnId(2) && r.msg.command == "QUIT");
        assert!(quit_to_bob);
    }

    // ── PING/PONG ─────────────────────────────────────────────────────────────

    #[test]
    fn test_ping_returns_pong() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");

        let ping_msg = parse("PING :irc.test").unwrap();
        let responses = server.on_message(ConnId(1), &ping_msg);
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].msg.command, "PONG");
    }

    // ── AWAY ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_away_set_and_clear() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");

        let away_msg = parse("AWAY :Out to lunch").unwrap();
        let r1 = server.on_message(ConnId(1), &away_msg);
        assert_eq!(r1[0].msg.command, "306"); // RPL_NOWAWAY

        let unaway_msg = parse("AWAY").unwrap();
        let r2 = server.on_message(ConnId(1), &unaway_msg);
        assert_eq!(r2[0].msg.command, "305"); // RPL_UNAWAY
    }

    // ── OPER ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_oper_correct_password() {
        let mut server = make_server(); // oper_password = "operpass"
        register_client(&mut server, ConnId(1), "alice");

        let oper_msg = parse("OPER admin operpass").unwrap();
        let responses = server.on_message(ConnId(1), &oper_msg);
        assert_eq!(responses[0].msg.command, "381"); // RPL_YOUREOPER
    }

    #[test]
    fn test_oper_wrong_password() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");

        let oper_msg = parse("OPER admin wrongpass").unwrap();
        let responses = server.on_message(ConnId(1), &oper_msg);
        assert_eq!(responses[0].msg.command, "464"); // ERR_PASSWDMISMATCH
    }

    // ── on_disconnect ─────────────────────────────────────────────────────────

    #[test]
    fn test_on_disconnect_broadcasts_quit() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");
        register_client(&mut server, ConnId(2), "bob");

        server.on_message(ConnId(1), &parse("JOIN #room").unwrap());
        server.on_message(ConnId(2), &parse("JOIN #room").unwrap());

        // Simulate unexpected disconnect (no QUIT sent).
        let responses = server.on_disconnect(ConnId(1));

        // Bob should get the QUIT broadcast.
        let quit_to_bob = responses.iter().any(|r| r.conn_id == ConnId(2) && r.msg.command == "QUIT");
        assert!(quit_to_bob);
    }

    #[test]
    fn test_on_disconnect_unknown_conn_is_noop() {
        let mut server = make_server();
        let responses = server.on_disconnect(ConnId(999));
        assert!(responses.is_empty());
    }

    // ── MODE ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_mode_channel_query() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");
        server.on_message(ConnId(1), &parse("JOIN #test").unwrap());

        let mode_msg = parse("MODE #test").unwrap();
        let responses = server.on_message(ConnId(1), &mode_msg);
        assert_eq!(responses[0].msg.command, "324"); // RPL_CHANNELMODEIS
    }

    // ── TOPIC ────────────────────────────────────────────────────────────────

    #[test]
    fn test_topic_set_and_query() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice");
        server.on_message(ConnId(1), &parse("JOIN #test").unwrap());

        let set_msg = parse("TOPIC #test :New topic here").unwrap();
        server.on_message(ConnId(1), &set_msg);

        let query_msg = parse("TOPIC #test").unwrap();
        let responses = server.on_message(ConnId(1), &query_msg);
        assert_eq!(responses[0].msg.command, "332"); // RPL_TOPIC
        assert!(responses[0].msg.params.iter().any(|p| p.contains("New topic here")));
    }

    // ── KICK ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_kick_by_operator() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice"); // operator
        register_client(&mut server, ConnId(2), "bob");

        server.on_message(ConnId(1), &parse("JOIN #test").unwrap());
        server.on_message(ConnId(2), &parse("JOIN #test").unwrap());

        let kick_msg = parse("KICK #test bob :Out!").unwrap();
        let responses = server.on_message(ConnId(1), &kick_msg);

        // KICK should be broadcast to members.
        let kick_cmds: Vec<&Response> = responses.iter().filter(|r| r.msg.command == "KICK").collect();
        assert!(!kick_cmds.is_empty());
    }

    #[test]
    fn test_kick_non_operator_fails() {
        let mut server = make_server();
        register_client(&mut server, ConnId(1), "alice"); // operator
        register_client(&mut server, ConnId(2), "bob");  // regular member

        server.on_message(ConnId(1), &parse("JOIN #test").unwrap());
        server.on_message(ConnId(2), &parse("JOIN #test").unwrap());

        let kick_msg = parse("KICK #test alice :Out!").unwrap();
        let responses = server.on_message(ConnId(2), &kick_msg);
        assert_eq!(responses[0].msg.command, "482"); // ERR_CHANOPRIVSNEEDED
    }
}
