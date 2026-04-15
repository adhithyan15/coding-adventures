# frozen_string_literal: true

# Tests for CodingAdventures::IrcServer::IRCServer
#
# Coverage strategy:
#   - on_connect: creates client, returns empty array
#   - NICK: validation, uniqueness, registration trigger, nick-change broadcast
#   - USER: registration trigger, re-registration ignored
#   - Welcome sequence: 001 present, 376 present
#   - QUIT: ERROR message sent, channel peers notified
#   - JOIN: channel created, NAMES sent, topic/notopic
#   - PART: broadcast, cleanup
#   - PRIVMSG/NOTICE: channel relay, direct message, errors
#   - NAMES, LIST, TOPIC, KICK, INVITE, MODE, PING, PONG, AWAY, WHOIS, WHO, OPER
#   - on_disconnect: channel cleanup, quit broadcast
#   - Pre-registration gate: ERR_NOTREGISTERED for non-whitelisted commands
#   - Unknown command: ERR_UNKNOWNCOMMAND

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  minimum_coverage 80
end

require "minitest/autorun"
require "coding_adventures/irc_server"

S   = CodingAdventures::IrcServer
Msg = CodingAdventures::IrcProto::Message

# Helper: build a Message and run it through the server.
def cmd(server, conn_id, line)
  msg = CodingAdventures::IrcProto.parse(line)
  server.on_message(conn_id, msg)
end

# Helper: register a client fully (NICK + USER).
def register(server, conn_id, nick = "alice", host = "127.0.0.1")
  server.on_connect(conn_id, host)
  cmd(server, conn_id, "NICK #{nick}")
  cmd(server, conn_id, "USER #{nick} 0 * :#{nick.capitalize}")
end

# ─────────────────────────────────────────────────────────────────────────────
# on_connect
# ─────────────────────────────────────────────────────────────────────────────

class TestOnConnect < Minitest::Test
  def test_returns_empty_array
    s = S::IRCServer.new(server_name: "irc.test")
    assert_equal [], s.on_connect(1, "127.0.0.1")
  end

  def test_unknown_conn_id_on_message_is_noop
    s = S::IRCServer.new(server_name: "irc.test")
    assert_equal [], cmd(s, 99, "PING irc.test")
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# NICK command
# ─────────────────────────────────────────────────────────────────────────────

class TestNick < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    @server.on_connect(1, "127.0.0.1")
  end

  def test_nick_no_params_returns_431
    r = cmd(@server, 1, "NICK")
    assert r.any? { |_, m| m.command == "431" },
           "Expected ERR_NONICKNAMEGIVEN (431), got: #{r.inspect}"
  end

  def test_invalid_nick_returns_432
    r = cmd(@server, 1, "NICK 1badnick")
    assert r.any? { |_, m| m.command == "432" },
           "Expected ERR_ERRONEUSNICKNAME (432)"
  end

  def test_valid_nick_returns_nothing_without_user
    r = cmd(@server, 1, "NICK validnick")
    assert_equal [], r
  end

  def test_nick_in_use_returns_433
    @server.on_connect(2, "127.0.0.2")
    cmd(@server, 1, "NICK alice")
    r = cmd(@server, 2, "NICK alice")
    assert r.any? { |_, m| m.command == "433" },
           "Expected ERR_NICKNAMEINUSE (433)"
  end

  def test_nick_case_insensitive_collision
    cmd(@server, 1, "NICK Alice")
    @server.on_connect(2, "127.0.0.2")
    r = cmd(@server, 2, "NICK alice")
    assert r.any? { |_, m| m.command == "433" }
  end

  def test_nick_change_after_registration_broadcasts
    register(@server, 1, "alice")
    @server.on_connect(2, "127.0.0.2")
    register(@server, 2, "bob")
    cmd(@server, 2, "JOIN #test")
    cmd(@server, 1, "JOIN #test")
    r = cmd(@server, 2, "NICK bobby")
    # The NICK broadcast should reach conn 1 (who is in the same channel).
    assert r.any? { |cid, m| cid == 1 && m.command == "NICK" },
           "Expected NICK broadcast to channel peer"
  end

  def test_nick_same_name_no_broadcast
    register(@server, 1, "alice")
    r = cmd(@server, 1, "NICK alice")
    assert_equal [], r
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# USER command and registration
# ─────────────────────────────────────────────────────────────────────────────

class TestUserRegistration < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test", motd: ["Hello!"])
  end

  def test_full_registration_triggers_welcome
    @server.on_connect(1, "127.0.0.1")
    cmd(@server, 1, "NICK alice")
    r = cmd(@server, 1, "USER alice 0 * :Alice")
    commands = r.map { |_, m| m.command }
    assert_includes commands, "001", "Expected RPL_WELCOME (001)"
    assert_includes commands, "376", "Expected RPL_ENDOFMOTD (376)"
    assert commands.include?("372"), "Expected RPL_MOTD (372)"
  end

  def test_user_before_nick_no_welcome
    @server.on_connect(1, "127.0.0.1")
    r = cmd(@server, 1, "USER alice 0 * :Alice")
    assert_equal [], r
  end

  def test_user_too_few_params_returns_461
    @server.on_connect(1, "127.0.0.1")
    r = cmd(@server, 1, "USER alice")
    assert r.any? { |_, m| m.command == "461" }
  end

  def test_re_registration_ignored
    register(@server, 1)
    r = cmd(@server, 1, "USER bob 0 * :Bob")
    assert_equal [], r
  end

  def test_nick_after_user_triggers_welcome
    @server.on_connect(1, "127.0.0.1")
    cmd(@server, 1, "USER alice 0 * :Alice")
    r = cmd(@server, 1, "NICK alice")
    assert r.any? { |_, m| m.command == "001" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# Pre-registration gate
# ─────────────────────────────────────────────────────────────────────────────

class TestPreRegistrationGate < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    @server.on_connect(1, "127.0.0.1")
    cmd(@server, 1, "NICK alice")
    # Not yet registered (no USER sent)
  end

  def test_join_before_register_returns_451
    r = cmd(@server, 1, "JOIN #test")
    assert r.any? { |_, m| m.command == "451" }
  end

  def test_privmsg_before_register_returns_451
    r = cmd(@server, 1, "PRIVMSG bob :hi")
    assert r.any? { |_, m| m.command == "451" }
  end

  def test_cap_allowed_before_register
    r = cmd(@server, 1, "CAP LS")
    assert r.any? { |_, m| m.command == "CAP" }
  end

  def test_pass_allowed_before_register
    r = cmd(@server, 1, "PASS secret")
    assert_equal [], r
  end

  def test_quit_allowed_before_register
    r = cmd(@server, 1, "QUIT")
    # QUIT returns an ERROR message
    assert r.any? { |_, m| m.command == "ERROR" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# Unknown command
# ─────────────────────────────────────────────────────────────────────────────

class TestUnknownCommand < Minitest::Test
  def test_unknown_command_returns_421
    s = S::IRCServer.new(server_name: "irc.test")
    register(s, 1)
    r = cmd(s, 1, "FOOBAR arg")
    assert r.any? { |_, m| m.command == "421" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# QUIT
# ─────────────────────────────────────────────────────────────────────────────

class TestQuit < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    register(@server, 1, "alice")
    register(@server, 2, "bob")
  end

  def test_quit_sends_error_to_quitter
    r = cmd(@server, 1, "QUIT :bye")
    assert r.any? { |cid, m| cid == 1 && m.command == "ERROR" }
  end

  def test_quit_broadcasts_to_channel_peer
    cmd(@server, 1, "JOIN #test")
    cmd(@server, 2, "JOIN #test")
    r = cmd(@server, 1, "QUIT :leaving")
    assert r.any? { |cid, m| cid == 2 && m.command == "QUIT" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# JOIN
# ─────────────────────────────────────────────────────────────────────────────

class TestJoin < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    register(@server, 1, "alice")
  end

  def test_join_returns_join_message
    r = cmd(@server, 1, "JOIN #ruby")
    assert r.any? { |_, m| m.command == "JOIN" }
  end

  def test_join_sends_notopic_when_no_topic
    r = cmd(@server, 1, "JOIN #ruby")
    assert r.any? { |_, m| m.command == "331" }
  end

  def test_join_sends_names
    r = cmd(@server, 1, "JOIN #ruby")
    assert r.any? { |_, m| m.command == "353" }
    assert r.any? { |_, m| m.command == "366" }
  end

  def test_join_no_params_returns_461
    r = cmd(@server, 1, "JOIN")
    assert r.any? { |_, m| m.command == "461" }
  end

  def test_second_join_is_noop
    cmd(@server, 1, "JOIN #ruby")
    r = cmd(@server, 1, "JOIN #ruby")
    assert_equal [], r
  end

  def test_join_broadcasts_to_existing_member
    register(@server, 2, "bob")
    cmd(@server, 1, "JOIN #ruby")
    r = cmd(@server, 2, "JOIN #ruby")
    # Both alice (conn 1) and bob (conn 2) should get JOIN
    conn_ids = r.select { |_, m| m.command == "JOIN" }.map { |cid, _| cid }
    assert_includes conn_ids, 1
    assert_includes conn_ids, 2
  end

  def test_join_topic_sent_when_set
    cmd(@server, 1, "JOIN #ruby")
    cmd(@server, 1, "TOPIC #ruby :Best language!")
    register(@server, 2, "bob")
    r = cmd(@server, 2, "JOIN #ruby")
    assert r.any? { |_, m| m.command == "332" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# PART
# ─────────────────────────────────────────────────────────────────────────────

class TestPart < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    register(@server, 1, "alice")
    register(@server, 2, "bob")
    cmd(@server, 1, "JOIN #ruby")
    cmd(@server, 2, "JOIN #ruby")
  end

  def test_part_broadcasts_to_members
    r = cmd(@server, 1, "PART #ruby")
    assert r.any? { |cid, m| cid == 2 && m.command == "PART" }
    assert r.any? { |cid, m| cid == 1 && m.command == "PART" }
  end

  def test_part_not_in_channel_returns_442
    r = cmd(@server, 1, "PART #nonexistent")
    assert r.any? { |_, m| m.command == "442" }
  end

  def test_part_no_params_returns_461
    r = cmd(@server, 1, "PART")
    assert r.any? { |_, m| m.command == "461" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# PRIVMSG / NOTICE
# ─────────────────────────────────────────────────────────────────────────────

class TestPrivmsg < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    register(@server, 1, "alice")
    register(@server, 2, "bob")
  end

  def test_direct_message_delivered
    r = cmd(@server, 1, "PRIVMSG bob :hello")
    assert r.any? { |cid, m| cid == 2 && m.command == "PRIVMSG" }
  end

  def test_channel_message_relayed_not_to_sender
    cmd(@server, 1, "JOIN #ruby")
    cmd(@server, 2, "JOIN #ruby")
    r = cmd(@server, 1, "PRIVMSG #ruby :hi all")
    assert r.any? { |cid, m| cid == 2 && m.command == "PRIVMSG" }
    refute r.any? { |cid, m| cid == 1 && m.command == "PRIVMSG" }
  end

  def test_privmsg_nosuchnick_returns_401
    r = cmd(@server, 1, "PRIVMSG nobody :hello")
    assert r.any? { |_, m| m.command == "401" }
  end

  def test_privmsg_nosuchchannel_returns_403
    r = cmd(@server, 1, "PRIVMSG #nonexistent :hello")
    assert r.any? { |_, m| m.command == "403" }
  end

  def test_privmsg_no_params_returns_461
    r = cmd(@server, 1, "PRIVMSG")
    assert r.any? { |_, m| m.command == "461" }
  end

  def test_notice_direct_message_delivered
    r = cmd(@server, 1, "NOTICE bob :ping")
    assert r.any? { |cid, m| cid == 2 && m.command == "NOTICE" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# NAMES
# ─────────────────────────────────────────────────────────────────────────────

class TestNames < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    register(@server, 1, "alice")
    cmd(@server, 1, "JOIN #ruby")
  end

  def test_names_with_channel_param
    r = cmd(@server, 1, "NAMES #ruby")
    assert r.any? { |_, m| m.command == "353" }
  end

  def test_names_no_param_lists_all_channels
    r = cmd(@server, 1, "NAMES")
    assert r.any? { |_, m| m.command == "353" }
  end

  def test_names_nonexistent_channel_returns_403
    r = cmd(@server, 1, "NAMES #fake")
    assert r.any? { |_, m| m.command == "403" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# LIST
# ─────────────────────────────────────────────────────────────────────────────

class TestList < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    register(@server, 1, "alice")
    cmd(@server, 1, "JOIN #ruby")
  end

  def test_list_includes_channel
    r = cmd(@server, 1, "LIST")
    assert r.any? { |_, m| m.command == "322" && m.params.include?("#ruby") }
  end

  def test_list_ends_with_323
    r = cmd(@server, 1, "LIST")
    assert r.last[1].command == "323"
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# TOPIC
# ─────────────────────────────────────────────────────────────────────────────

class TestTopic < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    register(@server, 1, "alice")
    cmd(@server, 1, "JOIN #ruby")
  end

  def test_set_topic_broadcasts
    r = cmd(@server, 1, "TOPIC #ruby :Ruby rocks")
    assert r.any? { |_, m| m.command == "TOPIC" }
  end

  def test_query_topic_after_set
    cmd(@server, 1, "TOPIC #ruby :Ruby rocks")
    r = cmd(@server, 1, "TOPIC #ruby")
    assert r.any? { |_, m| m.command == "332" }
  end

  def test_query_no_topic_returns_331
    r = cmd(@server, 1, "TOPIC #ruby")
    assert r.any? { |_, m| m.command == "331" }
  end

  def test_topic_nosuchchannel_returns_403
    r = cmd(@server, 1, "TOPIC #fake :hello")
    assert r.any? { |_, m| m.command == "403" }
  end

  def test_topic_non_op_returns_482
    register(@server, 2, "bob")
    cmd(@server, 2, "JOIN #ruby")
    r = cmd(@server, 2, "TOPIC #ruby :Bob rules")
    assert r.any? { |_, m| m.command == "482" }
  end

  def test_topic_no_params_returns_461
    r = cmd(@server, 1, "TOPIC")
    assert r.any? { |_, m| m.command == "461" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# KICK
# ─────────────────────────────────────────────────────────────────────────────

class TestKick < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    register(@server, 1, "alice")  # will be operator
    register(@server, 2, "bob")
    cmd(@server, 1, "JOIN #ruby")
    cmd(@server, 2, "JOIN #ruby")
  end

  def test_operator_can_kick
    r = cmd(@server, 1, "KICK #ruby bob")
    assert r.any? { |_, m| m.command == "KICK" }
  end

  def test_non_operator_cannot_kick
    r = cmd(@server, 2, "KICK #ruby alice")
    assert r.any? { |_, m| m.command == "482" }
  end

  def test_kick_no_params_returns_461
    r = cmd(@server, 1, "KICK")
    assert r.any? { |_, m| m.command == "461" }
  end

  def test_kick_nosuchchannel_returns_403
    r = cmd(@server, 1, "KICK #fake bob")
    assert r.any? { |_, m| m.command == "403" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# INVITE
# ─────────────────────────────────────────────────────────────────────────────

class TestInvite < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    register(@server, 1, "alice")
    register(@server, 2, "bob")
  end

  def test_invite_sends_341_and_invite_message
    r = cmd(@server, 1, "INVITE bob #ruby")
    assert r.any? { |cid, m| cid == 1 && m.command == "341" }
    assert r.any? { |cid, m| cid == 2 && m.command == "INVITE" }
  end

  def test_invite_nosuchnick_returns_401
    r = cmd(@server, 1, "INVITE nobody #ruby")
    assert r.any? { |_, m| m.command == "401" }
  end

  def test_invite_no_params_returns_461
    r = cmd(@server, 1, "INVITE")
    assert r.any? { |_, m| m.command == "461" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# MODE
# ─────────────────────────────────────────────────────────────────────────────

class TestMode < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    register(@server, 1, "alice")
    cmd(@server, 1, "JOIN #ruby")
  end

  def test_mode_query_returns_324
    r = cmd(@server, 1, "MODE #ruby")
    assert r.any? { |_, m| m.command == "324" }
  end

  def test_mode_set_simple_flag
    r = cmd(@server, 1, "MODE #ruby +m")
    assert r.any? { |_, m| m.command == "MODE" }
  end

  def test_mode_no_params_returns_461
    r = cmd(@server, 1, "MODE")
    assert r.any? { |_, m| m.command == "461" }
  end

  def test_mode_nosuchchannel_returns_403
    r = cmd(@server, 1, "MODE #fake")
    assert r.any? { |_, m| m.command == "403" }
  end

  def test_user_mode_acked
    r = cmd(@server, 1, "MODE alice +i")
    assert r.any? { |_, m| m.command == "MODE" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# PING / PONG
# ─────────────────────────────────────────────────────────────────────────────

class TestPingPong < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    register(@server, 1, "alice")
  end

  def test_ping_returns_pong
    r = cmd(@server, 1, "PING irc.test")
    assert r.any? { |_, m| m.command == "PONG" }
  end

  def test_pong_returns_empty
    r = cmd(@server, 1, "PONG irc.test")
    assert_equal [], r
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# AWAY
# ─────────────────────────────────────────────────────────────────────────────

class TestAway < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    register(@server, 1, "alice")
  end

  def test_away_sets_message_returns_306
    r = cmd(@server, 1, "AWAY :Be right back")
    assert r.any? { |_, m| m.command == "306" }
  end

  def test_away_clear_returns_305
    cmd(@server, 1, "AWAY :gone")
    r = cmd(@server, 1, "AWAY")
    assert r.any? { |_, m| m.command == "305" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# WHOIS
# ─────────────────────────────────────────────────────────────────────────────

class TestWhois < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    register(@server, 1, "alice")
    register(@server, 2, "bob")
  end

  def test_whois_existing_nick_returns_311
    r = cmd(@server, 1, "WHOIS bob")
    assert r.any? { |_, m| m.command == "311" }
    assert r.any? { |_, m| m.command == "318" }
  end

  def test_whois_unknown_nick_returns_401
    r = cmd(@server, 1, "WHOIS nobody")
    assert r.any? { |_, m| m.command == "401" }
  end

  def test_whois_no_params_returns_461
    r = cmd(@server, 1, "WHOIS")
    assert r.any? { |_, m| m.command == "461" }
  end

  def test_whois_includes_channels
    cmd(@server, 2, "JOIN #ruby")
    r = cmd(@server, 1, "WHOIS bob")
    assert r.any? { |_, m| m.command == "319" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# WHO
# ─────────────────────────────────────────────────────────────────────────────

class TestWho < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    register(@server, 1, "alice")
    register(@server, 2, "bob")
  end

  def test_who_wildcard_lists_all
    r = cmd(@server, 1, "WHO *")
    assert r.any? { |_, m| m.command == "352" }
    assert r.any? { |_, m| m.command == "315" }
  end

  def test_who_channel
    cmd(@server, 1, "JOIN #ruby")
    cmd(@server, 2, "JOIN #ruby")
    r = cmd(@server, 1, "WHO #ruby")
    assert r.any? { |_, m| m.command == "352" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# OPER
# ─────────────────────────────────────────────────────────────────────────────

class TestOper < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test", oper_password: "secret")
    register(@server, 1, "alice")
  end

  def test_correct_password_grants_oper
    r = cmd(@server, 1, "OPER alice secret")
    assert r.any? { |_, m| m.command == "381" }
  end

  def test_wrong_password_returns_464
    r = cmd(@server, 1, "OPER alice wrong")
    assert r.any? { |_, m| m.command == "464" }
  end

  def test_oper_no_params_returns_461
    r = cmd(@server, 1, "OPER")
    assert r.any? { |_, m| m.command == "461" }
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# on_disconnect
# ─────────────────────────────────────────────────────────────────────────────

class TestOnDisconnect < Minitest::Test
  def setup
    @server = S::IRCServer.new(server_name: "irc.test")
    register(@server, 1, "alice")
    register(@server, 2, "bob")
    cmd(@server, 1, "JOIN #ruby")
    cmd(@server, 2, "JOIN #ruby")
  end

  def test_disconnect_broadcasts_quit_to_peers
    r = @server.on_disconnect(1)
    assert r.any? { |cid, m| cid == 2 && m.command == "QUIT" }
  end

  def test_disconnect_unknown_conn_is_noop
    r = @server.on_disconnect(99)
    assert_equal [], r
  end

  def test_disconnect_unregistered_no_quit_broadcast
    @server.on_connect(3, "127.0.0.3")
    r = @server.on_disconnect(3)
    assert_equal [], r
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# valid_nick? helper
# ─────────────────────────────────────────────────────────────────────────────

class TestValidNick < Minitest::Test
  def test_valid_alpha
    assert S.valid_nick?("alice")
  end

  def test_valid_with_digits
    assert S.valid_nick?("a1b2c3")
  end

  def test_valid_special_start
    assert S.valid_nick?("[user]")
  end

  def test_invalid_starts_with_digit
    refute S.valid_nick?("1user")
  end

  def test_invalid_too_long
    refute S.valid_nick?("abcdefghij")  # 10 chars
  end

  def test_invalid_empty
    refute S.valid_nick?("")
  end

  def test_valid_max_length
    assert S.valid_nick?("abcdefghi")   # 9 chars
  end
end
