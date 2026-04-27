package irc_server

import (
	"strings"
	"testing"

	irc_proto "github.com/adhithyan15/coding-adventures/code/packages/go/irc-proto"
)

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

func newTestServer() *IRCServer {
	return NewIRCServer("irc.test", []string{"Test MOTD line."}, "operpass")
}

func parse(line string) *irc_proto.Message {
	msg, err := irc_proto.Parse(line)
	if err != nil {
		panic("test helper parse failed: " + err.Error())
	}
	return msg
}

func findResponse(responses []Response, command string) *Response {
	for i := range responses {
		if responses[i].Msg.Command == command {
			return &responses[i]
		}
	}
	return nil
}

func registerClient(s *IRCServer, connID ConnID, nick, user, realname string) {
	s.OnMessage(connID, parse("NICK "+nick))
	s.OnMessage(connID, parse("USER "+user+" 0 * :"+realname))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

func TestRegistration_FullHandshake(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	resp := s.OnMessage(1, parse("NICK alice"))
	if len(resp) != 0 {
		t.Error("expected no response until USER is sent")
	}
	resp = s.OnMessage(1, parse("USER alice 0 * :Alice Smith"))
	if findResponse(resp, rplWelcome) == nil {
		t.Error("expected 001 RPL_WELCOME after NICK+USER")
	}
	if findResponse(resp, rplEndOfMotd) == nil {
		t.Error("expected 376 RPL_ENDOFMOTD")
	}
}

func TestRegistration_UserFirst(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	resp := s.OnMessage(1, parse("USER alice 0 * :Alice"))
	if len(resp) != 0 {
		t.Error("expected no response until NICK is sent")
	}
	resp = s.OnMessage(1, parse("NICK alice"))
	if findResponse(resp, rplWelcome) == nil {
		t.Error("expected 001 RPL_WELCOME after USER+NICK")
	}
}

func TestRegistration_AlreadyRegistered(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	resp := s.OnMessage(1, parse("USER alice 0 * :Alice"))
	if findResponse(resp, errAlreadyRegistered) == nil {
		t.Error("expected 462 ERR_ALREADYREGISTERED")
	}
}

// ---------------------------------------------------------------------------
// NICK errors
// ---------------------------------------------------------------------------

func TestNick_NoNickGiven(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	resp := s.OnMessage(1, parse("NICK"))
	if findResponse(resp, errNoNicknameGiven) == nil {
		t.Error("expected 431 ERR_NONICKNAMEGIVEN")
	}
}

func TestNick_InvalidNick(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	resp := s.OnMessage(1, parse("NICK 123bad"))
	if findResponse(resp, errErroneusNickname) == nil {
		t.Error("expected 432 ERR_ERRONEUSNICKNAME")
	}
}

func TestNick_NickInUse(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	resp := s.OnMessage(2, parse("NICK alice"))
	if findResponse(resp, errNickNameInUse) == nil {
		t.Error("expected 433 ERR_NICKNAMEINUSE")
	}
}

func TestNick_PostRegistrationChange(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	resp := s.OnMessage(1, parse("NICK alicia"))
	nickChange := findResponse(resp, "NICK")
	if nickChange == nil {
		t.Fatal("expected NICK broadcast")
	}
	if len(nickChange.Msg.Params) == 0 || nickChange.Msg.Params[0] != "alicia" {
		t.Errorf("expected new nick 'alicia', got %v", nickChange.Msg.Params)
	}
}

// ---------------------------------------------------------------------------
// JOIN
// ---------------------------------------------------------------------------

func TestJoin_FirstMemberIsOp(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("JOIN #test"))

	ch := s.channels["#test"]
	if ch == nil {
		t.Fatal("channel not created")
	}
	if !ch.members[1].isOperator {
		t.Error("first member should be channel operator")
	}
}

func TestJoin_SecondMemberNotOp(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(1, parse("JOIN #test"))
	s.OnMessage(2, parse("JOIN #test"))

	ch := s.channels["#test"]
	if ch.members[2].isOperator {
		t.Error("second member should not be channel operator")
	}
}

func TestJoin_BroadcastsToExistingMembers(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(1, parse("JOIN #test"))

	resp := s.OnMessage(2, parse("JOIN #test"))
	aliceGotJoin := false
	for _, r := range resp {
		if r.Msg.Command == "JOIN" && r.ConnID == 1 {
			aliceGotJoin = true
		}
	}
	if !aliceGotJoin {
		t.Error("alice should be notified when bob joins")
	}
}

// ---------------------------------------------------------------------------
// PART
// ---------------------------------------------------------------------------

func TestPart_Basic(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("PART #test"))
	if findResponse(resp, "PART") == nil {
		t.Error("expected PART response")
	}

	if s.channels["#test"] != nil {
		t.Error("channel should be destroyed when empty")
	}
}

func TestPart_NotInChannel(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(2, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("PART #test"))
	if findResponse(resp, errNotOnChannel) == nil {
		t.Error("expected 442 ERR_NOTONCHANNEL")
	}
}

func TestPart_NoSuchChannel(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("PART #nosuch"))
	if findResponse(resp, errNoSuchChannel) == nil {
		t.Error("expected 403 ERR_NOSUCHCHANNEL")
	}
}

func TestPart_DestroyEmptyChannel(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("JOIN #test"))
	s.OnMessage(1, parse("PART #test"))

	if s.channels["#test"] != nil {
		t.Error("channel should be destroyed when last member parts")
	}
}

// ---------------------------------------------------------------------------
// PRIVMSG / NOTICE
// ---------------------------------------------------------------------------

func TestPrivmsg_ToChannel(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(1, parse("JOIN #test"))
	s.OnMessage(2, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("PRIVMSG #test :hello"))
	bobGot := false
	for _, r := range resp {
		if r.Msg.Command == "PRIVMSG" && r.ConnID == 2 {
			bobGot = true
		}
	}
	if !bobGot {
		t.Error("bob should receive PRIVMSG in channel")
	}
	for _, r := range resp {
		if r.Msg.Command == "PRIVMSG" && r.ConnID == 1 {
			t.Error("alice should not receive her own PRIVMSG")
		}
	}
}

func TestPrivmsg_ToNick(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")

	resp := s.OnMessage(1, parse("PRIVMSG bob :hey"))
	bobGot := false
	for _, r := range resp {
		if r.Msg.Command == "PRIVMSG" && r.ConnID == 2 {
			bobGot = true
		}
	}
	if !bobGot {
		t.Error("bob should receive direct PRIVMSG")
	}
}

func TestPrivmsg_NoSuchNick(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("PRIVMSG nobody :hello"))
	if findResponse(resp, errNoSuchNick) == nil {
		t.Error("expected 401 ERR_NOSUCHNICK")
	}
}

func TestPrivmsg_AwayAutoReply(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(2, parse("AWAY :Out for lunch"))

	resp := s.OnMessage(1, parse("PRIVMSG bob :hey"))
	if findResponse(resp, rplAway) == nil {
		t.Error("expected 301 RPL_AWAY auto-reply when messaging an away user")
	}
}

func TestNotice_NoAwayReply(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(2, parse("AWAY :Out"))

	resp := s.OnMessage(1, parse("NOTICE bob :hey"))
	if findResponse(resp, rplAway) != nil {
		t.Error("NOTICE should never trigger away auto-reply")
	}
}

// ---------------------------------------------------------------------------
// QUIT
// ---------------------------------------------------------------------------

func TestQuit_BroadcastsToChannelPeers(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(1, parse("JOIN #test"))
	s.OnMessage(2, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("QUIT :goodbye"))
	bobGotQuit := false
	for _, r := range resp {
		if r.Msg.Command == "QUIT" && r.ConnID == 2 {
			bobGotQuit = true
		}
	}
	if !bobGotQuit {
		t.Error("bob should receive QUIT broadcast")
	}
}

func TestQuit_SendsError(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("QUIT"))
	if findResponse(resp, "ERROR") == nil {
		t.Error("expected ERROR message after QUIT")
	}
}

// ---------------------------------------------------------------------------
// OnDisconnect
// ---------------------------------------------------------------------------

func TestOnDisconnect_BroadcastsQuit(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(1, parse("JOIN #test"))
	s.OnMessage(2, parse("JOIN #test"))

	resp := s.OnDisconnect(1)
	bobGotQuit := false
	for _, r := range resp {
		if r.Msg.Command == "QUIT" && r.ConnID == 2 {
			bobGotQuit = true
		}
	}
	if !bobGotQuit {
		t.Error("bob should receive QUIT on alice's disconnect")
	}
}

// ---------------------------------------------------------------------------
// KICK
// ---------------------------------------------------------------------------

func TestKick_OperCanKick(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(1, parse("JOIN #test"))
	s.OnMessage(2, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("KICK #test bob"))
	if findResponse(resp, "KICK") == nil {
		t.Error("expected KICK broadcast")
	}
	ch := s.channels["#test"]
	if ch != nil && ch.members[2] != nil {
		t.Error("bob should be removed from channel after kick")
	}
}

func TestKick_NonOpCantKick(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(1, parse("JOIN #test"))
	s.OnMessage(2, parse("JOIN #test"))

	resp := s.OnMessage(2, parse("KICK #test alice"))
	if findResponse(resp, errChanOpPrivsNeeded) == nil {
		t.Error("expected 482 ERR_CHANOPRIVSNEEDED for non-op kick attempt")
	}
}

// ---------------------------------------------------------------------------
// TOPIC
// ---------------------------------------------------------------------------

func TestTopic_SetAndGet(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("JOIN #test"))
	s.OnMessage(1, parse("TOPIC #test :new topic"))

	resp := s.OnMessage(1, parse("TOPIC #test"))
	topicReply := findResponse(resp, rplTopic)
	if topicReply == nil {
		t.Fatal("expected 332 RPL_TOPIC")
	}
	// RPL_TOPIC params: [nick, channel, topic]
	if len(topicReply.Msg.Params) < 3 || topicReply.Msg.Params[2] != "new topic" {
		t.Errorf("wrong topic: %v", topicReply.Msg.Params)
	}
}

func TestTopic_NoTopic(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("TOPIC #test"))
	if findResponse(resp, rplNotopic) == nil {
		t.Error("expected 331 RPL_NOTOPIC")
	}
}

// ---------------------------------------------------------------------------
// PING / PONG
// ---------------------------------------------------------------------------

func TestPing_ReturnsPong(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("PING :token"))
	if findResponse(resp, "PONG") == nil {
		t.Error("expected PONG response to PING")
	}
}

func TestPong_Ignored(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("PONG :server"))
	if len(resp) != 0 {
		t.Error("expected no response to PONG")
	}
}

// ---------------------------------------------------------------------------
// AWAY
// ---------------------------------------------------------------------------

func TestAway_SetAndClear(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("AWAY :Out for lunch"))
	if findResponse(resp, rplNowaway) == nil {
		t.Error("expected 306 RPL_NOWAWAY")
	}

	resp = s.OnMessage(1, parse("AWAY"))
	if findResponse(resp, rplUnaway) == nil {
		t.Error("expected 305 RPL_UNAWAY")
	}
}

// ---------------------------------------------------------------------------
// WHOIS
// ---------------------------------------------------------------------------

func TestWhois_KnownNick(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("WHOIS alice"))
	if findResponse(resp, rplWhoisUser) == nil {
		t.Error("expected 311 RPL_WHOISUSER")
	}
	if findResponse(resp, rplEndOfWhois) == nil {
		t.Error("expected 318 RPL_ENDOFWHOIS")
	}
}

func TestWhois_UnknownNick(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("WHOIS nobody"))
	if findResponse(resp, errNoSuchNick) == nil {
		t.Error("expected 401 ERR_NOSUCHNICK")
	}
}

// ---------------------------------------------------------------------------
// WHO
// ---------------------------------------------------------------------------

func TestWho_AllUsers(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("WHO"))
	if findResponse(resp, rplWhoReply) == nil {
		t.Error("expected 352 RPL_WHOREPLY")
	}
	if findResponse(resp, rplEndOfWho) == nil {
		t.Error("expected 315 RPL_ENDOFWHO")
	}
}

// ---------------------------------------------------------------------------
// OPER
// ---------------------------------------------------------------------------

func TestOper_CorrectPassword(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("OPER alice operpass"))
	if findResponse(resp, rplYoureOper) == nil {
		t.Error("expected 381 RPL_YOUREOPER")
	}
}

func TestOper_WrongPassword(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("OPER alice wrongpass"))
	if findResponse(resp, errPasswdMismatch) == nil {
		t.Error("expected 464 ERR_PASSWDMISMATCH")
	}
}

// ---------------------------------------------------------------------------
// LIST
// ---------------------------------------------------------------------------

func TestList_EmptyServer(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("LIST"))
	if findResponse(resp, rplListStart) == nil {
		t.Error("expected 321 RPL_LISTSTART")
	}
	if findResponse(resp, rplListEnd) == nil {
		t.Error("expected 323 RPL_LISTEND")
	}
}

func TestList_WithChannels(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("LIST"))
	if findResponse(resp, rplList) == nil {
		t.Error("expected 322 RPL_LIST for existing channel")
	}
}

// ---------------------------------------------------------------------------
// MODE
// ---------------------------------------------------------------------------

func TestMode_ChannelQuery(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("MODE #test"))
	if findResponse(resp, rplChannelModeIs) == nil {
		t.Error("expected 324 RPL_CHANNELMODEIS")
	}
}

func TestMode_UserQuery(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("MODE alice"))
	if findResponse(resp, "221") == nil {
		t.Error("expected 221 RPL_UMODEIS for user mode query")
	}
}

// ---------------------------------------------------------------------------
// INVITE
// ---------------------------------------------------------------------------

func TestInvite_SendsInvite(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")

	resp := s.OnMessage(1, parse("INVITE bob #test"))

	if findResponse(resp, rplInviting) == nil {
		t.Error("expected 341 RPL_INVITING to inviter")
	}
	bobGotInvite := false
	for _, r := range resp {
		if r.Msg.Command == "INVITE" && r.ConnID == 2 {
			bobGotInvite = true
		}
	}
	if !bobGotInvite {
		t.Error("bob should receive INVITE message")
	}
}

// ---------------------------------------------------------------------------
// Unknown command
// ---------------------------------------------------------------------------

func TestUnknownCommand(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("FLORP param"))
	if findResponse(resp, errUnknownCommand) == nil {
		t.Error("expected 421 ERR_UNKNOWNCOMMAND for unknown command")
	}
}

// ---------------------------------------------------------------------------
// Nick validation
// ---------------------------------------------------------------------------

func TestValidNick(t *testing.T) {
	valid := []string{"alice", "Alice", "_bot", "|relay|", "a", "abcdefghi"}
	for _, n := range valid {
		if !validNick(n) {
			t.Errorf("expected %q to be a valid nick", n)
		}
	}

	invalid := []string{"", "123start", "bad nick", "toolongnick0", " space"}
	for _, n := range invalid {
		if validNick(n) {
			t.Errorf("expected %q to be an invalid nick", n)
		}
	}
}

// ---------------------------------------------------------------------------
// CAP command
// ---------------------------------------------------------------------------

func TestCap_ReturnsAck(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")

	resp := s.OnMessage(1, parse("CAP LS"))
	if len(resp) == 0 {
		t.Fatal("expected a CAP response")
	}
	found := false
	for _, r := range resp {
		if r.Msg.Command == "CAP" {
			found = true
			if len(r.Msg.Params) < 2 || r.Msg.Params[1] != "ACK" {
				t.Errorf("expected CAP * ACK, got params %v", r.Msg.Params)
			}
		}
	}
	if !found {
		t.Error("expected CAP ACK response")
	}
}

// ---------------------------------------------------------------------------
// PASS command
// ---------------------------------------------------------------------------

func TestPass_Ignored(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")

	resp := s.OnMessage(1, parse("PASS secret"))
	if len(resp) != 0 {
		t.Errorf("expected no response to PASS, got %d responses", len(resp))
	}
}

// ---------------------------------------------------------------------------
// NAMES command
// ---------------------------------------------------------------------------

func TestNames_SpecificChannel(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("NAMES #test"))
	if findResponse(resp, rplNamReply) == nil {
		t.Error("expected 353 RPL_NAMREPLY")
	}
	if findResponse(resp, rplEndOfNames) == nil {
		t.Error("expected 366 RPL_ENDOFNAMES")
	}
}

func TestNames_NonExistentChannel(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("NAMES #nosuch"))
	if findResponse(resp, rplEndOfNames) == nil {
		t.Error("expected 366 RPL_ENDOFNAMES for nonexistent channel")
	}
}

func TestNames_AllChannels(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("JOIN #chan1"))
	s.OnMessage(1, parse("JOIN #chan2"))

	resp := s.OnMessage(1, parse("NAMES"))
	nameReplies := 0
	for _, r := range resp {
		if r.Msg.Command == rplNamReply {
			nameReplies++
		}
	}
	if nameReplies < 2 {
		t.Errorf("expected at least 2 NAMREPLY responses, got %d", nameReplies)
	}
}

// ---------------------------------------------------------------------------
// MODE command additional coverage
// ---------------------------------------------------------------------------

func TestMode_NoParams(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("MODE"))
	if findResponse(resp, errNeedMoreParams) == nil {
		t.Error("expected 461 ERR_NEEDMOREPARAMS for MODE with no params")
	}
}

func TestMode_ChannelSet(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("MODE #test +m"))
	found := false
	for _, r := range resp {
		if r.Msg.Command == "MODE" {
			found = true
		}
	}
	if !found {
		t.Error("expected MODE broadcast after +m")
	}
}

func TestMode_ChannelUnset(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("JOIN #test"))
	s.OnMessage(1, parse("MODE #test +m"))

	resp := s.OnMessage(1, parse("MODE #test -m"))
	found := false
	for _, r := range resp {
		if r.Msg.Command == "MODE" {
			found = true
		}
	}
	if !found {
		t.Error("expected MODE broadcast after -m")
	}
}

func TestMode_NonExistentChannel(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("MODE #nosuch"))
	if findResponse(resp, errNoSuchChannel) == nil {
		t.Error("expected 403 ERR_NOSUCHCHANNEL for nonexistent channel")
	}
}

func TestMode_UserSet(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("MODE alice +i"))
	found := false
	for _, r := range resp {
		if r.Msg.Command == "MODE" {
			found = true
		}
	}
	if !found {
		t.Error("expected MODE response for user mode set")
	}
}

// ---------------------------------------------------------------------------
// WHOIS additional coverage
// ---------------------------------------------------------------------------

func TestWhois_NoParams(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("WHOIS"))
	if findResponse(resp, errNeedMoreParams) == nil {
		t.Error("expected 461 ERR_NEEDMOREPARAMS for WHOIS with no params")
	}
}

func TestWhois_WithChannels(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("WHOIS alice"))
	if findResponse(resp, rplWhoisChannels) == nil {
		t.Error("expected 319 RPL_WHOISCHANNELS when user is in a channel")
	}
}

func TestWhois_WithAway(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("AWAY :Out for lunch"))

	resp := s.OnMessage(1, parse("WHOIS alice"))
	if findResponse(resp, rplAway) == nil {
		t.Error("expected 301 RPL_AWAY in WHOIS for away user")
	}
}

// ---------------------------------------------------------------------------
// WHO additional coverage
// ---------------------------------------------------------------------------

func TestWho_ChannelMask(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(1, parse("JOIN #test"))
	s.OnMessage(2, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("WHO #test"))
	whoReplies := 0
	for _, r := range resp {
		if r.Msg.Command == rplWhoReply {
			whoReplies++
		}
	}
	if whoReplies < 2 {
		t.Errorf("expected at least 2 WHO replies for #test, got %d", whoReplies)
	}
}

func TestWho_AwayUser(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("AWAY :Out"))

	resp := s.OnMessage(1, parse("WHO *"))
	for _, r := range resp {
		if r.Msg.Command == rplWhoReply {
			// WHO reply params: [nick, channel, username, hostname, server, targetNick, H/G, "0 realname"]
			// The H/G flag is at index 6.
			if len(r.Msg.Params) > 6 && r.Msg.Params[6] != "G" {
				t.Errorf("expected G (gone) for away user, got %q", r.Msg.Params[6])
			}
		}
	}
}

// ---------------------------------------------------------------------------
// INVITE additional coverage
// ---------------------------------------------------------------------------

func TestInvite_NoParams(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("INVITE bob"))
	if findResponse(resp, errNeedMoreParams) == nil {
		t.Error("expected 461 ERR_NEEDMOREPARAMS for INVITE with only 1 param")
	}
}

func TestInvite_NoSuchNick(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("INVITE nobody #test"))
	if findResponse(resp, errNoSuchNick) == nil {
		t.Error("expected 401 ERR_NOSUCHNICK for INVITE to unknown nick")
	}
}

// ---------------------------------------------------------------------------
// KICK additional coverage
// ---------------------------------------------------------------------------

func TestKick_NoParams(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("KICK #test"))
	if findResponse(resp, errNeedMoreParams) == nil {
		t.Error("expected 461 ERR_NEEDMOREPARAMS for KICK with 1 param")
	}
}

func TestKick_NoSuchChannel(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("KICK #nosuch bob"))
	if findResponse(resp, errNoSuchChannel) == nil {
		t.Error("expected 403 ERR_NOSUCHCHANNEL")
	}
}

func TestKick_NotInChannel(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(2, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("KICK #test bob"))
	if findResponse(resp, errNotOnChannel) == nil {
		t.Error("expected 442 ERR_NOTONCHANNEL when kicker is not in channel")
	}
}

func TestKick_NoSuchUser(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(1, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("KICK #test bob"))
	if findResponse(resp, errUserNotInChannel) == nil {
		t.Error("expected 441 ERR_USERNOTINCHANNEL when target is not in channel")
	}
}

func TestKick_WithReason(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(1, parse("JOIN #test"))
	s.OnMessage(2, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("KICK #test bob :bye"))
	kickFound := false
	for _, r := range resp {
		if r.Msg.Command == "KICK" {
			kickFound = true
			if len(r.Msg.Params) < 3 || r.Msg.Params[2] != "bye" {
				t.Errorf("expected reason 'bye', got params %v", r.Msg.Params)
			}
		}
	}
	if !kickFound {
		t.Error("expected KICK broadcast")
	}
}

// ---------------------------------------------------------------------------
// TOPIC additional coverage
// ---------------------------------------------------------------------------

func TestTopic_NoParams(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("TOPIC"))
	if findResponse(resp, errNeedMoreParams) == nil {
		t.Error("expected 461 ERR_NEEDMOREPARAMS for TOPIC with no params")
	}
}

func TestTopic_NoSuchChannel(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("TOPIC #nosuch"))
	if findResponse(resp, errNoSuchChannel) == nil {
		t.Error("expected 403 ERR_NOSUCHCHANNEL")
	}
}

func TestTopic_NotInChannel(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(2, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("TOPIC #test"))
	if findResponse(resp, errNotOnChannel) == nil {
		t.Error("expected 442 ERR_NOTONCHANNEL when not in channel")
	}
}

func TestTopic_BroadcastsToChannel(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(1, parse("JOIN #test"))
	s.OnMessage(2, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("TOPIC #test :new topic"))
	topicCount := 0
	for _, r := range resp {
		if r.Msg.Command == "TOPIC" {
			topicCount++
		}
	}
	if topicCount < 2 {
		t.Errorf("expected TOPIC broadcast to both members, got %d", topicCount)
	}
}

// ---------------------------------------------------------------------------
// PRIVMSG/NOTICE additional coverage
// ---------------------------------------------------------------------------

func TestPrivmsg_ToChannel_NoSuchChannel(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("PRIVMSG #nosuch :hello"))
	if findResponse(resp, errNoSuchChannel) == nil {
		t.Error("expected 403 ERR_NOSUCHCHANNEL for PRIVMSG to unknown channel")
	}
}

// ---------------------------------------------------------------------------
// PART additional coverage
// ---------------------------------------------------------------------------

func TestPart_NoParams(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("PART"))
	if findResponse(resp, errNeedMoreParams) == nil {
		t.Error("expected 461 ERR_NEEDMOREPARAMS for PART with no params")
	}
}

// ---------------------------------------------------------------------------
// JOIN additional coverage
// ---------------------------------------------------------------------------

func TestJoin_NoParams(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("JOIN"))
	if findResponse(resp, errNeedMoreParams) == nil {
		t.Error("expected 461 ERR_NEEDMOREPARAMS for JOIN with no params")
	}
}

func TestJoin_RejoinExisting(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("JOIN #test"))
	_ = resp
}

// ---------------------------------------------------------------------------
// NICK additional coverage
// ---------------------------------------------------------------------------

func TestNick_ChangeBeforeRegistration(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnMessage(1, parse("NICK alice"))
	s.OnMessage(1, parse("NICK alicia"))
	resp := s.OnMessage(1, parse("USER alice 0 * :Alice"))
	if findResponse(resp, rplWelcome) == nil {
		t.Error("expected 001 RPL_WELCOME after completing registration")
	}
}

// ---------------------------------------------------------------------------
// NewIRCServer with nil MOTD
// ---------------------------------------------------------------------------

func TestNewIRCServer_NilMotd(t *testing.T) {
	s := NewIRCServer("irc.test", nil, "")
	s.OnConnect(1, "127.0.0.1")
	s.OnMessage(1, parse("NICK alice"))
	resp := s.OnMessage(1, parse("USER alice 0 * :Alice"))
	if findResponse(resp, rplWelcome) == nil {
		t.Error("expected 001 RPL_WELCOME with nil MOTD")
	}
}

// ---------------------------------------------------------------------------
// OPER with empty password (disabled)
// ---------------------------------------------------------------------------

func TestOper_EmptyPasswordDisabled(t *testing.T) {
	s := NewIRCServer("irc.test", []string{"MOTD"}, "")
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("OPER alice anything"))
	if findResponse(resp, errPasswdMismatch) == nil {
		t.Error("expected 464 ERR_PASSWDMISMATCH when oper password is empty")
	}
}

func TestOper_NoParams(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")

	resp := s.OnMessage(1, parse("OPER onlyone"))
	if findResponse(resp, errNeedMoreParams) == nil {
		t.Error("expected 461 ERR_NEEDMOREPARAMS for OPER with 1 param")
	}
}

// ---------------------------------------------------------------------------
// QUIT with message
// ---------------------------------------------------------------------------

func TestQuit_WithMessage(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(1, parse("JOIN #test"))
	s.OnMessage(2, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("QUIT :goodbye"))
	_ = resp
}

// ---------------------------------------------------------------------------
// LIST additional coverage
// ---------------------------------------------------------------------------

func TestList_TopicInList(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	registerClient(s, 1, "alice", "alice", "Alice")
	s.OnMessage(1, parse("JOIN #test"))
	s.OnMessage(1, parse("TOPIC #test :my topic"))

	resp := s.OnMessage(1, parse("LIST"))
	for _, r := range resp {
		if r.Msg.Command == rplList {
			found := false
			for _, p := range r.Msg.Params {
				if strings.Contains(p, "my topic") {
					found = true
				}
			}
			if !found {
				t.Errorf("expected topic in LIST reply, got params %v", r.Msg.Params)
			}
		}
	}
}

// ---------------------------------------------------------------------------
// NOTICE additional coverage
// ---------------------------------------------------------------------------

func TestNotice_ToChannel(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")
	s.OnMessage(1, parse("JOIN #test"))
	s.OnMessage(2, parse("JOIN #test"))

	resp := s.OnMessage(1, parse("NOTICE #test :attention"))
	bobGot := false
	for _, r := range resp {
		if r.Msg.Command == "NOTICE" && r.ConnID == 2 {
			bobGot = true
		}
	}
	if !bobGot {
		t.Error("bob should receive NOTICE in channel")
	}
}

func TestNotice_ToNick(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnConnect(2, "127.0.0.2")
	registerClient(s, 1, "alice", "alice", "Alice")
	registerClient(s, 2, "bob", "bob", "Bob")

	resp := s.OnMessage(1, parse("NOTICE bob :hey"))
	bobGot := false
	for _, r := range resp {
		if r.Msg.Command == "NOTICE" && r.ConnID == 2 {
			bobGot = true
		}
	}
	if !bobGot {
		t.Error("bob should receive direct NOTICE")
	}
}

// ---------------------------------------------------------------------------
// OnMessage with unregistered client for commands requiring registration
// ---------------------------------------------------------------------------

func TestOnMessage_UnregisteredJoin(t *testing.T) {
	s := newTestServer()
	s.OnConnect(1, "127.0.0.1")
	s.OnMessage(1, parse("NICK alice"))

	resp := s.OnMessage(1, parse("JOIN #test"))
	_ = resp
}
