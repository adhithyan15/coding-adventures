// Package irc_server implements a pure IRC state machine.
//
// This package is the "brain" of the IRC stack. It knows the IRC protocol
// (RFC 1459) but knows nothing about TCP sockets, goroutines, or file I/O.
// Every function takes a ConnID + message and returns a slice of responses.
//
// Architecture (Russian nesting doll)
//
//	irc-net-stdlib  ← TCP socket management
//	    irc-server  ← THIS PACKAGE: IRC state machine
//	    irc-proto   ← message parsing / serialisation
//
// The central abstraction is "pure function on a state machine":
//
//	(State, Input) → (State', []Response)
//
// The state (IRCServer struct) is mutated in place rather than returned, but
// the observable behaviour is identical to a pure function: the same sequence
// of inputs always produces the same sequence of outputs.
//
// Design choices
//
//   - No goroutines inside this package. Thread-safety is the caller's
//     responsibility (irc-net-stdlib uses a mutex to serialise calls).
//   - ConnID is the handle for a connection. It is a plain int64 so it can be
//     used as a map key without any interface overhead.
//   - All map lookups are case-insensitive for nicks and channel names
//     (IRC convention: #General == #general, Alice == ALICE).
package irc_server

import (
	"fmt"
	"regexp"
	"strings"

	irc_proto "github.com/adhithyan15/coding-adventures/code/packages/go/irc-proto"
)

// Version of this package.
const Version = "0.1.0"

// ---------------------------------------------------------------------------
// IRC numeric reply constants (RFC 1459)
// ---------------------------------------------------------------------------
//
// IRC servers respond to commands with numeric codes. The naming convention is:
//   rpl* = success reply
//   err* = error reply
//
// Having named constants avoids magic numbers scattered through the code.
// They are strings because irc_proto.Message.Command is a string.

const (
	// Connection registration
	rplWelcome  = "001" // :server 001 nick :Welcome to the Internet Relay Network
	rplYourHost = "002" // :server 002 nick :Your host is ...
	rplCreated  = "003" // :server 003 nick :This server was created ...
	rplMyInfo   = "004" // :server 004 nick <server> <version> <umodes> <cmodes>

	// MOTD
	rplMotdStart = "375" // :server 375 nick :- server Message of the Day -
	rplMotd      = "372" // :server 372 nick :- <text>
	rplEndOfMotd = "376" // :server 376 nick :End of /MOTD command
	errNoMotd    = "422" // :server 422 nick :MOTD File is missing

	// NAMES
	rplNamReply   = "353" // = <channel> :<prefix>nick...
	rplEndOfNames = "366" // <channel> :End of /NAMES list

	// LIST
	rplListStart = "321" // Channel :Users  Name
	rplList      = "322" // <channel> <count> :<topic>
	rplListEnd   = "323" // :End of /LIST

	// TOPIC
	rplTopic   = "332" // <channel> :<topic>
	rplNotopic = "331" // <channel> :No topic is set

	// WHOIS
	rplWhoisUser     = "311" // <nick> <user> <host> * :<realname>
	rplWhoisServer   = "312" // <nick> <server> :<server-info>
	rplWhoisChannels = "319" // <nick> :<channel list>
	rplEndOfWhois    = "318" // <nick> :End of /WHOIS list

	// WHO
	rplWhoReply = "352" // <channel> <user> <host> <server> <nick> H|G :<hopcount> <realname>
	rplEndOfWho = "315" // <name> :End of /WHO list

	// AWAY
	rplAway    = "301" // <nick> :<away message>
	rplUnaway  = "305" // :You are no longer marked as being away
	rplNowaway = "306" // :You have been marked as being away

	// INVITE
	rplInviting = "341" // <channel> <nick>

	// MODE
	rplChannelModeIs = "324" // <channel> <mode>

	// OPER
	rplYoureOper = "381" // :You are now an IRC operator

	// Errors
	errNoSuchNick        = "401" // <nick/channel> :No such nick/channel
	errNoSuchChannel     = "403" // <channel> :No such channel
	errCannotSendToChan  = "404" // <channel> :Cannot send to channel
	errNotOnChannel      = "442" // <channel> :You're not on that channel
	errUserNotInChannel  = "441" // <nick> <channel> :They aren't on that channel
	errNickNameInUse     = "433" // <nick> :Nickname is already in use
	errNicknameInUse     = "433" // alias
	errNoNicknameGiven   = "431" // :No nickname given
	errErroneusNickname  = "432" // <nick> :Erroneous nickname
	errNeedMoreParams    = "461" // <command> :Not enough parameters
	errNotRegistered     = "451" // :You have not registered
	errAlreadyRegistered = "462" // :Unauthorized command (already registered)
	errPasswdMismatch    = "464" // :Password incorrect
	errChanOpPrivsNeeded = "482" // <channel> :You're not channel operator
	errUnknownCommand    = "421" // <command> :Unknown command
)

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

// ConnID identifies a single TCP connection to the server.
// The event loop assigns unique IDs; this package treats them as opaque handles.
type ConnID int64

// Response bundles a target connection and the message to send.
// Handlers return slices of Response so they can send different messages to
// different connections (e.g. broadcasting a PRIVMSG to all channel members).
type Response struct {
	ConnID ConnID
	Msg    *irc_proto.Message
}

// client represents a connected IRC user.
//
// A client progresses through two states:
//  1. Pre-registration: the client has connected but not yet sent both NICK
//     and USER. Only a limited set of commands are accepted in this state.
//  2. Registered: the server has sent the 001 welcome; the client may now
//     use all IRC commands.
type client struct {
	id       ConnID
	hostname string // remote IP address (used in nick!user@host masks)
	nick     string
	username string
	realname string

	// Channel membership: key = lowercase channel name.
	channels map[string]bool

	// Away status.
	hasAway     bool
	awayMessage string

	// Whether this client has completed NICK+USER registration.
	registered bool

	// Whether this client has gained IRC operator status (via OPER).
	isOper bool
}

// mask returns the full nick!user@host mask for this client.
// This is used as the Prefix in messages originating from a user.
func (c *client) mask() string {
	return fmt.Sprintf("%s!%s@%s", c.nick, c.username, c.hostname)
}

// channelMember records a client's membership in a channel along with flags.
type channelMember struct {
	c          *client
	isOperator bool // channel operator (set on the first member to join)
}

// channel represents an IRC channel.
type channel struct {
	name    string                    // canonical name, lowercase for map key
	topic   string                    // current topic, empty if not set
	members map[ConnID]*channelMember // key = ConnID
	modes   map[byte]bool             // channel modes (e.g. 'm' for moderated)
}

// ---------------------------------------------------------------------------
// IRCServer — the central state machine
// ---------------------------------------------------------------------------

// IRCServer is the stateful IRC server core.
//
// It is NOT goroutine-safe. The caller (irc-net-stdlib) must serialise all
// method calls, which it does by holding a mutex during every callback.
type IRCServer struct {
	serverName string
	version    string
	motd       []string
	operPwd    string

	// clients maps ConnID → *client for all currently connected clients.
	clients map[ConnID]*client

	// channels maps lowercase channel name → *channel.
	channels map[string]*channel

	// nicks maps lowercase nick → ConnID for uniqueness lookups.
	nicks map[string]ConnID
}

// NewIRCServer creates a new IRC server with the given configuration.
//
// serverName: the hostname used in server messages (e.g. "irc.example.com").
// motd: lines of the Message of the Day.
// operPassword: password for the OPER command. Empty string means no OPER.
func NewIRCServer(serverName string, motd []string, operPassword string) *IRCServer {
	if motd == nil {
		motd = []string{}
	}
	return &IRCServer{
		serverName: serverName,
		version:    "1.0",
		motd:       motd,
		operPwd:    operPassword,
		clients:    make(map[ConnID]*client),
		channels:   make(map[string]*channel),
		nicks:      make(map[string]ConnID),
	}
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

// OnConnect registers a new TCP connection.
//
// Creates a client record for the connection but does not send anything —
// IRC clients are expected to initiate registration by sending CAP, NICK, and USER.
//
// Returns an empty slice; no messages are sent until the client speaks.
func (s *IRCServer) OnConnect(connID ConnID, host string) []Response {
	s.clients[connID] = &client{
		id:       connID,
		hostname: host,
		channels: make(map[string]bool),
	}
	return nil
}

// OnMessage dispatches an inbound IRC message and returns the resulting responses.
//
// This is the central dispatch method. It:
//  1. Looks up the client record for connID.
//  2. Routes the message to the appropriate handler method based on msg.Command.
//  3. Returns the slice of Response values to send.
//
// If the client is unknown, we return nil rather than panicking.
//
// Pre-registration gate: a client that has not yet completed the NICK+USER
// handshake may only send: NICK, USER, CAP, QUIT, and PASS. Any other command
// gets 451 ERR_NOTREGISTERED. This mirrors the behaviour of real IRC servers.
func (s *IRCServer) OnMessage(connID ConnID, msg *irc_proto.Message) []Response {
	c := s.clients[connID]
	if c == nil {
		// Unknown connection -- should not happen in normal operation.
		return nil
	}

	cmd := msg.Command

	// Commands allowed before registration completes.
	switch cmd {
	case "NICK":
		return s.handleNick(c, msg)
	case "USER":
		return s.handleUser(c, msg)
	case "CAP":
		return s.handleCap(c, msg)
	case "PASS":
		return s.handlePass(c, msg)
	case "QUIT":
		return s.handleQuit(c, msg)
	}

	// All other commands require registration.
	if !c.registered {
		return []Response{{c.id, s.clientMsg(c, errNotRegistered, "You have not registered")}}
	}

	// Post-registration command dispatch.
	switch cmd {
	case "JOIN":
		return s.handleJoin(c, msg)
	case "PART":
		return s.handlePart(c, msg)
	case "PRIVMSG":
		return s.handlePrivmsg(c, msg)
	case "NOTICE":
		return s.handleNotice(c, msg)
	case "NAMES":
		return s.handleNames(c, msg)
	case "LIST":
		return s.handleList(c, msg)
	case "TOPIC":
		return s.handleTopic(c, msg)
	case "KICK":
		return s.handleKick(c, msg)
	case "INVITE":
		return s.handleInvite(c, msg)
	case "MODE":
		return s.handleMode(c, msg)
	case "PING":
		return s.handlePing(c, msg)
	case "PONG":
		return s.handlePong(c, msg)
	case "AWAY":
		return s.handleAway(c, msg)
	case "WHOIS":
		return s.handleWhois(c, msg)
	case "WHO":
		return s.handleWho(c, msg)
	case "OPER":
		return s.handleOper(c, msg)
	default:
		return []Response{{c.id, s.clientMsg(c, errUnknownCommand, cmd, "Unknown command")}}
	}
}

// OnDisconnect handles a client disconnecting (either they closed, or we did).
//
//  1. Broadcasts QUIT to all channel peers.
//  2. Removes the client from all channels.
//  3. Removes the client from the nicks and clients maps.
func (s *IRCServer) OnDisconnect(connID ConnID) []Response {
	c := s.clients[connID]
	if c == nil {
		return nil
	}

	var responses []Response

	if c.registered && c.nick != "" {
		// Build the QUIT message to broadcast to peers.
		quitMsg := &irc_proto.Message{
			Prefix:  c.mask(),
			Command: "QUIT",
			Params:  []string{"Connection closed"},
		}

		// Find all unique peers across all channels.
		peers := s.uniqueChannelPeers(c)
		for peerID := range peers {
			responses = append(responses, Response{peerID, quitMsg})
		}
	}

	// Remove from all channels.
	for chanName := range c.channels {
		ch := s.channels[chanName]
		if ch != nil {
			delete(ch.members, connID)
			if len(ch.members) == 0 {
				delete(s.channels, chanName)
			}
		}
	}

	// Remove nick registration.
	if c.nick != "" {
		delete(s.nicks, strings.ToLower(c.nick))
	}

	// Remove client record.
	delete(s.clients, connID)

	return responses
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

// makeMsg constructs a server-prefixed Message with the given command and params.
// Params that begin with ":" have the colon stripped (irc_proto's Serialize
// stores decoded values -- no leading colons -- and adds them back when needed).
func (s *IRCServer) makeMsg(command string, params ...string) *irc_proto.Message {
	cleaned := make([]string, len(params))
	for i, p := range params {
		if strings.HasPrefix(p, ":") {
			cleaned[i] = p[1:]
		} else {
			cleaned[i] = p
		}
	}
	return &irc_proto.Message{
		Prefix:  s.serverName,
		Command: command,
		Params:  cleaned,
	}
}

// clientMsg builds a server message addressed to c's nick.
// Like makeMsg but automatically inserts the client's nick (or "*" if not yet
// set) as the first parameter. Most IRC numerics have the form:
//
//	:server <numeric> <target_nick> <rest...>
//
// so this helper eliminates the repeated "c.nick or *" boilerplate.
func (s *IRCServer) clientMsg(c *client, command string, params ...string) *irc_proto.Message {
	nick := c.nick
	if nick == "" {
		nick = "*"
	}
	all := make([]string, 0, 1+len(params))
	all = append(all, nick)
	all = append(all, params...)
	return s.makeMsg(command, all...)
}

// welcome sends the full connection registration sequence (001–376) to c.
//
// The welcome sequence is:
//
//	001 RPL_WELCOME
//	002 RPL_YOURHOST
//	003 RPL_CREATED
//	004 RPL_MYINFO
//	375 RPL_MOTDSTART  (or 422 ERR_NOMOTD if no MOTD)
//	372 RPL_MOTD       (one per MOTD line)
//	376 RPL_ENDOFMOTD
func (s *IRCServer) welcome(c *client) []Response {
	nick := c.nick
	responses := []Response{
		{c.id, s.makeMsg(rplWelcome, nick, fmt.Sprintf("Welcome to the IRC Network %s!%s@%s", nick, c.username, c.hostname))},
		{c.id, s.makeMsg(rplYourHost, nick, fmt.Sprintf("Your host is %s, running version %s", s.serverName, s.version))},
		{c.id, s.makeMsg(rplCreated, nick, "This server was created just now")},
		{c.id, s.makeMsg(rplMyInfo, nick, s.serverName, s.version, "o", "o")},
	}

	if len(s.motd) == 0 {
		responses = append(responses, Response{c.id, s.makeMsg(errNoMotd, nick, "MOTD File is missing")})
	} else {
		responses = append(responses, Response{c.id, s.makeMsg(rplMotdStart, nick, fmt.Sprintf("- %s Message of the Day -", s.serverName))})
		for _, line := range s.motd {
			responses = append(responses, Response{c.id, s.makeMsg(rplMotd, nick, fmt.Sprintf("- %s", line))})
		}
		responses = append(responses, Response{c.id, s.makeMsg(rplEndOfMotd, nick, "End of /MOTD command")})
	}
	return responses
}

// names builds 353 (NAMREPLY) + 366 (ENDOFNAMES) responses for ch.
//
// The 353 line lists all visible members of the channel. Each member's nick
// is prefixed with "@" if they are a channel operator.
//
// Example 353 payload: "= #general :@alice bob +carol"
func (s *IRCServer) names(ch *channel, requestingNick string) []Response {
	var parts []string
	for _, member := range ch.members {
		prefix := ""
		if member.isOperator {
			prefix = "@"
		}
		parts = append(parts, prefix+member.c.nick)
	}
	nameList := strings.Join(parts, " ")
	return []Response{
		{s.findConnID(requestingNick), s.makeMsg(rplNamReply, requestingNick, "=", ch.name, nameList)},
		{s.findConnID(requestingNick), s.makeMsg(rplEndOfNames, requestingNick, ch.name, "End of /NAMES list")},
	}
}

// findConnID looks up the ConnID for a nick. Returns 0 if not found.
func (s *IRCServer) findConnID(nick string) ConnID {
	id, ok := s.nicks[strings.ToLower(nick)]
	if !ok {
		return 0
	}
	return id
}

// uniqueChannelPeers returns the set of ConnIDs of all clients that share at
// least one channel with c, excluding c itself.
//
// This is used to determine who should receive QUIT and NICK-change broadcasts.
func (s *IRCServer) uniqueChannelPeers(c *client) map[ConnID]bool {
	peers := make(map[ConnID]bool)
	for chanName := range c.channels {
		ch := s.channels[chanName]
		if ch == nil {
			continue
		}
		for peerID := range ch.members {
			if peerID != c.id {
				peers[peerID] = true
			}
		}
	}
	return peers
}

// nickRegexp validates IRC nicks per RFC 1459.
//
// Rules:
//   - First character: letter or one of [ ] \ ` _ ^ { | }
//   - Subsequent characters: same + digits + hyphen
//   - Maximum length: 9 characters
//
// The regex is compiled once at package init time.
var nickRegexp = regexp.MustCompile(`^[a-zA-Z\[\]\\` + "`" + `_^{|}][a-zA-Z0-9\[\]\\` + "`" + `_^{|}\-]{0,8}$`)

// validNick returns true iff nick passes IRC nick validation.
func validNick(nick string) bool {
	return nickRegexp.MatchString(nick)
}

// ---------------------------------------------------------------------------
// Command handlers
// ---------------------------------------------------------------------------

// handleCap handles the CAP (Capability Negotiation) command.
//
// Modern IRC clients send "CAP LS" at the start of a connection to discover
// server capabilities before sending NICK/USER. We do not implement capability
// negotiation, so we acknowledge all CAP requests with an empty ACK and move on.
// This prevents clients that require a response from hanging.
func (s *IRCServer) handleCap(c *client, msg *irc_proto.Message) []Response {
	// Reply with an empty CAP ACK to satisfy clients that wait for it.
	// The format is: CAP * ACK :<capabilities>
	// An empty capabilities list means "no capabilities negotiated".
	return []Response{{
		c.id,
		&irc_proto.Message{
			Prefix:  s.serverName,
			Command: "CAP",
			Params:  []string{"*", "ACK", ""},
		},
	}}
}

// handlePass handles the PASS command (connection password).
// PASS is sent before NICK/USER on servers that require a connection password.
// We do not enforce connection passwords in this v1, so we silently ignore PASS.
func (s *IRCServer) handlePass(c *client, msg *irc_proto.Message) []Response {
	return nil
}

// handleNick handles the NICK command -- set or change a client's nickname.
//
// Pre-registration: the client is trying to set their nick for the first time.
// We validate it, check uniqueness, store it, and -- if USER has already been
// received -- trigger the welcome sequence.
//
// Post-registration (nick change): broadcast ":old!user@host NICK new" to all
// clients who share a channel with the nick-changer, then update the nick index.
//
// Error cases:
//
//	431  -- No nick given (empty params).
//	432  -- Nick fails the RFC 1459 character/length validation.
//	433  -- Nick is already in use by another client.
func (s *IRCServer) handleNick(c *client, msg *irc_proto.Message) []Response {
	if len(msg.Params) == 0 {
		return []Response{{c.id, s.clientMsg(c, errNoNicknameGiven, "No nickname given")}}
	}

	newNick := msg.Params[0]

	if !validNick(newNick) {
		return []Response{{c.id, s.clientMsg(c, errErroneusNickname, newNick, "Erroneous nickname")}}
	}

	if existingID, taken := s.nicks[strings.ToLower(newNick)]; taken && existingID != c.id {
		return []Response{{c.id, s.clientMsg(c, errNicknameInUse, newNick, "Nickname is already in use")}}
	}

	oldNick := c.nick
	oldMask := c.mask()

	if oldNick != "" {
		delete(s.nicks, strings.ToLower(oldNick))
	}

	c.nick = newNick
	s.nicks[strings.ToLower(newNick)] = c.id

	// Post-registration: broadcast NICK change to peers.
	if c.registered && oldNick != "" {
		nickChangeMsg := &irc_proto.Message{
			Prefix:  oldMask,
			Command: "NICK",
			Params:  []string{newNick},
		}
		var responses []Response
		responses = append(responses, Response{c.id, nickChangeMsg})
		for peerID := range s.uniqueChannelPeers(c) {
			responses = append(responses, Response{peerID, nickChangeMsg})
		}
		return responses
	}

	// Pre-registration: check if USER already done -- welcome.
	if c.username != "" {
		c.registered = true
		return s.welcome(c)
	}

	// NICK stored; waiting for USER -- send nothing yet.
	return nil
}

// handleUser handles the USER command -- provide username and realname.
//
// Syntax: USER <username> <mode> <unused> :<realname>
//
// This command is sent once at registration time. After receiving both NICK
// and USER, the server sends the welcome sequence.
//
// Error cases:
//   - 461 ERR_NEEDMOREPARAMS if fewer than 4 params.
//   - 462 ERR_ALREADYREGISTERED if the client has already registered.
func (s *IRCServer) handleUser(c *client, msg *irc_proto.Message) []Response {
	if c.registered {
		return []Response{{c.id, s.clientMsg(c, errAlreadyRegistered, "Unauthorized command (already registered)")}}
	}

	if len(msg.Params) < 4 {
		return []Response{{c.id, s.clientMsg(c, errNeedMoreParams, "USER", "Not enough parameters")}}
	}

	c.username = msg.Params[0]
	c.realname = msg.Params[3]

	if c.nick != "" {
		c.registered = true
		return s.welcome(c)
	}

	// USER stored; waiting for NICK.
	return nil
}

// handleQuit handles the QUIT command -- graceful disconnection.
//
// Syntax: QUIT [:<message>]
//
// Broadcasts a QUIT message to all channel peers, then sends ERROR to the
// quitting client. The calling layer (irc-net-stdlib) is responsible for
// closing the TCP connection after receiving the ERROR response.
func (s *IRCServer) handleQuit(c *client, msg *irc_proto.Message) []Response {
	quitMsg := "Quit"
	if len(msg.Params) > 0 && msg.Params[0] != "" {
		quitMsg = msg.Params[0]
	}

	var responses []Response

	if c.registered && c.nick != "" {
		broadcast := &irc_proto.Message{
			Prefix:  c.mask(),
			Command: "QUIT",
			Params:  []string{quitMsg},
		}
		for peerID := range s.uniqueChannelPeers(c) {
			responses = append(responses, Response{peerID, broadcast})
		}
	}

	// Send ERROR to the quitting client.
	responses = append(responses, Response{
		c.id,
		&irc_proto.Message{
			Command: "ERROR",
			Params:  []string{fmt.Sprintf("Closing Link: %s (%s)", c.hostname, quitMsg)},
		},
	})

	// Clean up client state.
	for chanName := range c.channels {
		ch := s.channels[chanName]
		if ch != nil {
			delete(ch.members, c.id)
			if len(ch.members) == 0 {
				delete(s.channels, chanName)
			}
		}
	}
	if c.nick != "" {
		delete(s.nicks, strings.ToLower(c.nick))
	}
	delete(s.clients, c.id)

	return responses
}

// handleJoin handles the JOIN command -- join one or more channels.
//
// Syntax: JOIN <#channel> [,<#channel>...] [<key> [,<key>...]]
//
// For each channel:
//   - Creates the channel if it does not exist; the first member becomes op.
//   - If the client is already in the channel, it is a no-op.
//   - Broadcasts JOIN to all existing members (so they know a new user arrived).
//   - Sends NAMES (353 + 366) to the joiner so they know who is in the room.
//   - Sends TOPIC (332) to the joiner if a topic is set.
func (s *IRCServer) handleJoin(c *client, msg *irc_proto.Message) []Response {
	if len(msg.Params) == 0 {
		return []Response{{c.id, s.clientMsg(c, errNeedMoreParams, "JOIN", "Not enough parameters")}}
	}

	channelNames := strings.Split(msg.Params[0], ",")
	var responses []Response

	for _, chanName := range channelNames {
		if chanName == "" {
			continue
		}
		chanKey := strings.ToLower(chanName)

		ch := s.channels[chanKey]

		// Already in the channel -- no-op.
		if ch != nil && ch.members[c.id] != nil {
			continue
		}

		// Create channel if needed.
		if ch == nil {
			ch = &channel{
				name:    chanName,
				members: make(map[ConnID]*channelMember),
				modes:   make(map[byte]bool),
			}
			s.channels[chanKey] = ch
		}

		// Add the member; first member becomes operator.
		isOp := len(ch.members) == 0
		ch.members[c.id] = &channelMember{c: c, isOperator: isOp}
		c.channels[chanKey] = true

		// Broadcast JOIN to everyone in the channel (including the joiner).
		joinMsg := &irc_proto.Message{
			Prefix:  c.mask(),
			Command: "JOIN",
			Params:  []string{chanName},
		}
		for memberID := range ch.members {
			responses = append(responses, Response{memberID, joinMsg})
		}

		// Send NAMES to the joiner.
		responses = append(responses, s.names(ch, c.nick)...)

		// Send TOPIC to the joiner if set.
		if ch.topic != "" {
			responses = append(responses, Response{c.id, s.makeMsg(rplTopic, c.nick, chanName, ch.topic)})
		}
	}

	return responses
}

// handlePart handles the PART command -- leave a channel.
//
// Syntax: PART <#channel> [:<message>]
//
// Broadcasts PART to all members (including the departing user), then removes
// the client from the channel. If the channel becomes empty, it is destroyed.
func (s *IRCServer) handlePart(c *client, msg *irc_proto.Message) []Response {
	if len(msg.Params) == 0 {
		return []Response{{c.id, s.clientMsg(c, errNeedMoreParams, "PART", "Not enough parameters")}}
	}

	chanName := strings.ToLower(msg.Params[0])
	partMsg := ""
	if len(msg.Params) > 1 {
		partMsg = msg.Params[1]
	}

	ch := s.channels[chanName]
	if ch == nil {
		return []Response{{c.id, s.clientMsg(c, errNoSuchChannel, chanName, "No such channel")}}
	}

	if ch.members[c.id] == nil {
		return []Response{{c.id, s.clientMsg(c, errNotOnChannel, chanName, "You're not on that channel")}}
	}

	// Build PART broadcast.
	partParams := []string{chanName}
	if partMsg != "" {
		partParams = append(partParams, partMsg)
	}
	partBroadcast := &irc_proto.Message{
		Prefix:  c.mask(),
		Command: "PART",
		Params:  partParams,
	}

	var responses []Response
	for memberID := range ch.members {
		responses = append(responses, Response{memberID, partBroadcast})
	}

	// Remove client from channel.
	delete(ch.members, c.id)
	delete(c.channels, chanName)

	// Destroy channel if empty.
	if len(ch.members) == 0 {
		delete(s.channels, chanName)
	}

	return responses
}

// deliverMessage handles both PRIVMSG and NOTICE delivery.
// The key difference: NOTICE never triggers auto-reply (e.g. away messages).
func (s *IRCServer) deliverMessage(c *client, msg *irc_proto.Message, isNotice bool) []Response {
	if len(msg.Params) < 2 {
		return []Response{{c.id, s.clientMsg(c, errNeedMoreParams, msg.Command, "Not enough parameters")}}
	}

	target := msg.Params[0]
	text := msg.Params[1]

	outMsg := &irc_proto.Message{
		Prefix:  c.mask(),
		Command: msg.Command,
		Params:  []string{target, text},
	}

	if strings.HasPrefix(target, "#") {
		// Channel message.
		chanName := strings.ToLower(target)
		ch := s.channels[chanName]
		if ch == nil {
			return []Response{{c.id, s.clientMsg(c, errNoSuchChannel, chanName, "No such channel")}}
		}

		var responses []Response
		for memberID, member := range ch.members {
			if memberID == c.id {
				continue // don't echo back to sender
			}
			responses = append(responses, Response{memberID, outMsg})

			// Auto-reply away message for PRIVMSG only.
			if !isNotice && member.c.hasAway {
				responses = append(responses, Response{
					c.id,
					s.makeMsg(rplAway, c.nick, member.c.nick, member.c.awayMessage),
				})
			}
		}
		return responses
	}

	// Direct nick message.
	targetConnID, ok := s.nicks[strings.ToLower(target)]
	if !ok {
		return []Response{{c.id, s.clientMsg(c, errNoSuchNick, target, "No such nick/channel")}}
	}

	var responses []Response
	responses = append(responses, Response{targetConnID, outMsg})

	// Away auto-reply for PRIVMSG.
	if !isNotice {
		targetClient := s.clients[targetConnID]
		if targetClient != nil && targetClient.hasAway {
			responses = append(responses, Response{
				c.id,
				s.makeMsg(rplAway, c.nick, targetClient.nick, targetClient.awayMessage),
			})
		}
	}

	return responses
}

// handlePrivmsg handles the PRIVMSG command -- send a message.
func (s *IRCServer) handlePrivmsg(c *client, msg *irc_proto.Message) []Response {
	return s.deliverMessage(c, msg, false)
}

// handleNotice handles the NOTICE command -- send a notice (no auto-replies).
func (s *IRCServer) handleNotice(c *client, msg *irc_proto.Message) []Response {
	return s.deliverMessage(c, msg, true)
}

// handleNames handles the NAMES command -- list members of a channel.
//
// Syntax: NAMES [<#channel>]
//
// Returns 353 (NAMREPLY) + 366 (ENDOFNAMES) for the requested channel.
// If no channel is specified, returns NAMES for all channels.
func (s *IRCServer) handleNames(c *client, msg *irc_proto.Message) []Response {
	nick := c.nick
	if nick == "" {
		nick = "*"
	}

	if len(msg.Params) > 0 {
		chanName := strings.ToLower(msg.Params[0])
		ch := s.channels[chanName]
		if ch != nil {
			return s.names(ch, nick)
		}
		// Channel not found -- send just the terminator.
		return []Response{{c.id, s.makeMsg(rplEndOfNames, nick, chanName, "End of /NAMES list")}}
	}

	// No channel specified -- send NAMES for all channels.
	var responses []Response
	for _, ch := range s.channels {
		responses = append(responses, s.names(ch, nick)...)
	}
	return responses
}

// handleList handles the LIST command -- enumerate all channels.
//
// Returns:
//
//	321 -- RPL_LISTSTART header.
//	322 -- RPL_LIST, one per channel: name, member count, topic.
//	323 -- RPL_LISTEND terminator.
func (s *IRCServer) handleList(c *client, msg *irc_proto.Message) []Response {
	nick := c.nick
	if nick == "" {
		nick = "*"
	}

	responses := []Response{
		{c.id, s.makeMsg(rplListStart, nick, "Channel", "Users  Name")},
	}

	for _, ch := range s.channels {
		responses = append(responses, Response{
			c.id,
			s.makeMsg(rplList, nick, ch.name, fmt.Sprintf("%d", len(ch.members)), ch.topic),
		})
	}

	responses = append(responses, Response{c.id, s.makeMsg(rplListEnd, nick, "End of /LIST")})
	return responses
}

// handleTopic handles the TOPIC command -- get or set a channel's topic.
//
// Syntax:
//
//	TOPIC <#channel>           -- query the current topic.
//	TOPIC <#channel> :<topic>  -- set a new topic.
func (s *IRCServer) handleTopic(c *client, msg *irc_proto.Message) []Response {
	if len(msg.Params) == 0 {
		return []Response{{c.id, s.clientMsg(c, errNeedMoreParams, "TOPIC", "Not enough parameters")}}
	}

	chanName := strings.ToLower(msg.Params[0])
	nick := c.nick
	if nick == "" {
		nick = "*"
	}

	ch := s.channels[chanName]
	if ch == nil {
		return []Response{{c.id, s.clientMsg(c, errNoSuchChannel, chanName, "No such channel")}}
	}

	if ch.members[c.id] == nil {
		return []Response{{c.id, s.clientMsg(c, errNotOnChannel, chanName, "You're not on that channel")}}
	}

	if len(msg.Params) < 2 {
		if ch.topic != "" {
			return []Response{{c.id, s.makeMsg(rplTopic, nick, chanName, ch.topic)}}
		}
		return []Response{{c.id, s.makeMsg(rplNotopic, nick, chanName, "No topic is set")}}
	}

	// Set mode.
	newTopic := msg.Params[1]
	ch.topic = newTopic

	topicBroadcast := &irc_proto.Message{
		Prefix:  c.mask(),
		Command: "TOPIC",
		Params:  []string{chanName, newTopic},
	}
	var responses []Response
	for memberID := range ch.members {
		responses = append(responses, Response{memberID, topicBroadcast})
	}
	return responses
}

// handleKick handles the KICK command -- remove a member from a channel (op only).
//
// Syntax: KICK <#channel> <nick> [:<reason>]
//
// Only channel operators may use KICK. After kicking:
//   - Broadcast ":kicker!user@host KICK #channel victim :<reason>" to all
//     current channel members.
//   - Remove the victim from the channel's member list.
func (s *IRCServer) handleKick(c *client, msg *irc_proto.Message) []Response {
	if len(msg.Params) < 2 {
		return []Response{{c.id, s.clientMsg(c, errNeedMoreParams, "KICK", "Not enough parameters")}}
	}

	chanName := strings.ToLower(msg.Params[0])
	targetNick := msg.Params[1]
	reason := c.nick
	if reason == "" {
		reason = "*"
	}
	if len(msg.Params) > 2 {
		reason = msg.Params[2]
	}

	ch := s.channels[chanName]
	if ch == nil {
		return []Response{{c.id, s.clientMsg(c, errNoSuchChannel, chanName, "No such channel")}}
	}

	kickerMember := ch.members[c.id]
	if kickerMember == nil {
		return []Response{{c.id, s.clientMsg(c, errNotOnChannel, chanName, "You're not on that channel")}}
	}

	if !kickerMember.isOperator {
		return []Response{{c.id, s.clientMsg(c, errChanOpPrivsNeeded, chanName, "You're not channel operator")}}
	}

	targetConnID, ok := s.nicks[strings.ToLower(targetNick)]
	if !ok || ch.members[targetConnID] == nil {
		return []Response{{c.id, s.clientMsg(c, errUserNotInChannel, targetNick, chanName, "They aren't on that channel")}}
	}

	targetClient := s.clients[targetConnID]

	kickBroadcast := &irc_proto.Message{
		Prefix:  c.mask(),
		Command: "KICK",
		Params:  []string{chanName, targetNick, reason},
	}
	var responses []Response
	for memberID := range ch.members {
		responses = append(responses, Response{memberID, kickBroadcast})
	}

	delete(ch.members, targetConnID)
	if targetClient != nil {
		delete(targetClient.channels, chanName)
	}

	if len(ch.members) == 0 {
		delete(s.channels, chanName)
	}

	return responses
}

// handleInvite handles the INVITE command -- invite a nick to a channel.
//
// Syntax: INVITE <nick> <#channel>
//
// Sends an INVITE message directly to the target nick. The inviting client
// receives 341 RPL_INVITING as confirmation.
func (s *IRCServer) handleInvite(c *client, msg *irc_proto.Message) []Response {
	if len(msg.Params) < 2 {
		return []Response{{c.id, s.clientMsg(c, errNeedMoreParams, "INVITE", "Not enough parameters")}}
	}

	targetNick := msg.Params[0]
	chanName := strings.ToLower(msg.Params[1])
	nick := c.nick
	if nick == "" {
		nick = "*"
	}

	targetConnID, ok := s.nicks[strings.ToLower(targetNick)]
	if !ok {
		return []Response{{c.id, s.clientMsg(c, errNoSuchNick, targetNick, "No such nick/channel")}}
	}

	responses := []Response{
		{c.id, s.makeMsg(rplInviting, nick, chanName, targetNick)},
	}

	inviteMsg := &irc_proto.Message{
		Prefix:  c.mask(),
		Command: "INVITE",
		Params:  []string{targetNick, chanName},
	}
	responses = append(responses, Response{targetConnID, inviteMsg})

	return responses
}

// handleMode handles the MODE command -- query or set channel/user modes.
//
// This v1 implementation supports:
//   - MODE #channel       -- 324 RPL_CHANNELMODEIS (current channel modes).
//   - MODE nick           -- 221 RPL_UMODEIS (current user modes).
//   - MODE #channel +/-X  -- acknowledge with a MODE broadcast.
//   - MODE nick +/-X      -- acknowledge with a MODE broadcast.
func (s *IRCServer) handleMode(c *client, msg *irc_proto.Message) []Response {
	if len(msg.Params) == 0 {
		return []Response{{c.id, s.clientMsg(c, errNeedMoreParams, "MODE", "Not enough parameters")}}
	}

	target := msg.Params[0]
	nick := c.nick
	if nick == "" {
		nick = "*"
	}

	if strings.HasPrefix(target, "#") {
		chanName := strings.ToLower(target)
		ch := s.channels[chanName]
		if ch == nil {
			return []Response{{c.id, s.clientMsg(c, errNoSuchChannel, chanName, "No such channel")}}
		}

		if len(msg.Params) == 1 {
			modeStr := "+"
			for m := range ch.modes {
				modeStr += string([]byte{m})
			}
			return []Response{{c.id, s.makeMsg(rplChannelModeIs, nick, chanName, modeStr)}}
		}

		modeStr := msg.Params[1]
		if strings.HasPrefix(modeStr, "+") {
			for _, ch2 := range modeStr[1:] {
				ch.modes[byte(ch2)] = true
			}
		} else if strings.HasPrefix(modeStr, "-") {
			for _, ch2 := range modeStr[1:] {
				delete(ch.modes, byte(ch2))
			}
		}

		modeBroadcast := &irc_proto.Message{
			Prefix:  c.mask(),
			Command: "MODE",
			Params:  []string{chanName, modeStr},
		}
		var responses []Response
		for memberID := range ch.members {
			responses = append(responses, Response{memberID, modeBroadcast})
		}
		return responses
	}

	// User MODE.
	if len(msg.Params) == 1 {
		return []Response{{c.id, s.makeMsg("221", nick, "+")}}
	}

	modeStr := msg.Params[1]
	modeBroadcast := &irc_proto.Message{
		Prefix:  c.mask(),
		Command: "MODE",
		Params:  []string{target, modeStr},
	}
	return []Response{{c.id, modeBroadcast}}
}

// handlePing handles the PING command -- keepalive from client to server.
func (s *IRCServer) handlePing(c *client, msg *irc_proto.Message) []Response {
	serverToken := s.serverName
	if len(msg.Params) > 0 {
		serverToken = msg.Params[0]
	}
	return []Response{{
		c.id,
		&irc_proto.Message{
			Prefix:  s.serverName,
			Command: "PONG",
			Params:  []string{s.serverName, serverToken},
		},
	}}
}

// handlePong handles the PONG command -- client's response to a server PING.
// We don't send server-initiated PINGs in v1, so we simply ignore any PONG.
func (s *IRCServer) handlePong(c *client, msg *irc_proto.Message) []Response {
	return nil
}

// handleAway handles the AWAY command -- set or clear away status.
//
// Syntax:
//
//	AWAY :<message>  -- mark as away with the given message.
//	AWAY             -- clear away status (mark as present).
func (s *IRCServer) handleAway(c *client, msg *irc_proto.Message) []Response {
	if len(msg.Params) > 0 && msg.Params[0] != "" {
		c.awayMessage = msg.Params[0]
		c.hasAway = true
		return []Response{{c.id, s.clientMsg(c, rplNowaway, "You have been marked as being away")}}
	}

	c.awayMessage = ""
	c.hasAway = false
	return []Response{{c.id, s.clientMsg(c, rplUnaway, "You are no longer marked as being away")}}
}

// handleWhois handles the WHOIS command -- retrieve information about a nick.
//
// Syntax: WHOIS <nick>
//
// Returns a sequence of numerics describing the target user:
//
//	311  -- RPL_WHOISUSER: nick, username, hostname, realname.
//	312  -- RPL_WHOISSERVER: which server the user is on.
//	319  -- RPL_WHOISCHANNELS: list of channels the user is in.
//	301  -- RPL_AWAY: away message (only if user is away).
//	318  -- RPL_ENDOFWHOIS: terminator.
func (s *IRCServer) handleWhois(c *client, msg *irc_proto.Message) []Response {
	if len(msg.Params) == 0 {
		return []Response{{c.id, s.clientMsg(c, errNeedMoreParams, "WHOIS", "Not enough parameters")}}
	}

	targetNick := msg.Params[0]
	nick := c.nick
	if nick == "" {
		nick = "*"
	}

	targetConnID, ok := s.nicks[strings.ToLower(targetNick)]
	if !ok {
		return []Response{{c.id, s.clientMsg(c, errNoSuchNick, targetNick, "No such nick/channel")}}
	}

	target := s.clients[targetConnID]
	if target == nil {
		return []Response{{c.id, s.clientMsg(c, errNoSuchNick, targetNick, "No such nick/channel")}}
	}

	targetNickStr := target.nick
	if targetNickStr == "" {
		targetNickStr = targetNick
	}
	username := target.username
	if username == "" {
		username = "*"
	}
	realname := target.realname

	responses := []Response{
		{c.id, s.makeMsg(rplWhoisUser, nick, targetNickStr, username, target.hostname, "*", realname)},
		{c.id, s.makeMsg(rplWhoisServer, nick, targetNickStr, s.serverName, "IRC server")},
	}

	if len(target.channels) > 0 {
		var chanList []string
		for ch := range target.channels {
			chanList = append(chanList, ch)
		}
		responses = append(responses, Response{
			c.id, s.makeMsg(rplWhoisChannels, nick, targetNickStr, strings.Join(chanList, " ")),
		})
	}

	if target.hasAway {
		responses = append(responses, Response{
			c.id, s.makeMsg(rplAway, nick, targetNickStr, target.awayMessage),
		})
	}

	responses = append(responses, Response{
		c.id, s.makeMsg(rplEndOfWhois, nick, targetNickStr, "End of /WHOIS list"),
	})

	return responses
}

// handleWho handles the WHO command -- list users matching a mask.
//
// Syntax: WHO [<mask>]
//
// Returns 352 (WHOREPLY) rows followed by 315 (ENDOFWHO).
// The 352 format: <channel> <user> <host> <server> <nick> H|G :<hopcount> <realname>
// H = here (not away), G = gone (away).
func (s *IRCServer) handleWho(c *client, msg *irc_proto.Message) []Response {
	nick := c.nick
	if nick == "" {
		nick = "*"
	}

	mask := "*"
	if len(msg.Params) > 0 {
		mask = msg.Params[0]
	}

	var responses []Response

	whoRow := func(targetClient *client, channelName string) Response {
		hereOrGone := "H"
		if targetClient.hasAway {
			hereOrGone = "G"
		}
		username := targetClient.username
		if username == "" {
			username = "*"
		}
		targetNick := targetClient.nick
		if targetNick == "" {
			targetNick = "*"
		}
		realname := targetClient.realname
		return Response{
			c.id,
			s.makeMsg(
				rplWhoReply,
				nick,
				channelName,
				username,
				targetClient.hostname,
				s.serverName,
				targetNick,
				hereOrGone,
				fmt.Sprintf("0 %s", realname),
			),
		}
	}

	if strings.HasPrefix(mask, "#") {
		chanName := strings.ToLower(mask)
		ch := s.channels[chanName]
		if ch != nil {
			for _, member := range ch.members {
				responses = append(responses, whoRow(member.c, chanName))
			}
		}
	} else {
		for _, cl := range s.clients {
			if cl.registered {
				responses = append(responses, whoRow(cl, "*"))
			}
		}
	}

	responses = append(responses, Response{c.id, s.makeMsg(rplEndOfWho, nick, mask, "End of /WHO list")})
	return responses
}

// handleOper handles the OPER command -- gain IRC operator privileges.
//
// Syntax: OPER <name> <password>
//
// If the supplied password matches the server's configured oper password,
// the client's isOper flag is set to true and they receive 381 RPL_YOUREOPER.
// Otherwise they receive 464 ERR_PASSWDMISMATCH.
func (s *IRCServer) handleOper(c *client, msg *irc_proto.Message) []Response {
	if len(msg.Params) < 2 {
		return []Response{{c.id, s.clientMsg(c, errNeedMoreParams, "OPER", "Not enough parameters")}}
	}

	password := msg.Params[1]

	if s.operPwd != "" && password == s.operPwd {
		c.isOper = true
		return []Response{{c.id, s.clientMsg(c, rplYoureOper, "You are now an IRC operator")}}
	}

	return []Response{{c.id, s.clientMsg(c, errPasswdMismatch, "Password incorrect")}}
}
