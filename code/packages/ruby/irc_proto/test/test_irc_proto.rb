# frozen_string_literal: true

# Tests for CodingAdventures::IrcProto
#
# Coverage strategy:
#   - Parse: command-only, prefix+command, params, trailing param,
#             numeric commands, error cases
#   - Serialize: command-only, with prefix, params, trailing param with spaces
#   - Message: equality, inspect

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  minimum_coverage 95
end

require "minitest/autorun"
require "coding_adventures/irc_proto"

Lib = CodingAdventures::IrcProto

# ─────────────────────────────────────────────────────────────────────────────
# Parse tests
# ─────────────────────────────────────────────────────────────────────────────

class TestParse < Minitest::Test
  def test_command_only
    m = Lib.parse("PING")
    assert_equal "PING",   m.command
    assert_nil             m.prefix
    assert_equal [],       m.params
  end

  def test_command_lowercased_becomes_upper
    m = Lib.parse("ping")
    assert_equal "PING", m.command
  end

  def test_command_mixed_case
    m = Lib.parse("Nick")
    assert_equal "NICK", m.command
  end

  def test_prefix_and_command
    m = Lib.parse(":irc.example.com NOTICE")
    assert_equal "irc.example.com", m.prefix
    assert_equal "NOTICE",          m.command
    assert_equal [],                m.params
  end

  def test_nick_user_host_prefix
    m = Lib.parse(":alice!alice@192.168.1.1 PRIVMSG #general :hello")
    assert_equal "alice!alice@192.168.1.1", m.prefix
    assert_equal "PRIVMSG",                 m.command
    assert_equal ["#general", "hello"],     m.params
  end

  def test_single_param
    m = Lib.parse("NICK alice")
    assert_equal "NICK",    m.command
    assert_equal ["alice"], m.params
  end

  def test_multiple_params
    m = Lib.parse("USER alice 0 * :Alice Smith")
    assert_equal "USER",                            m.command
    assert_equal ["alice", "0", "*", "Alice Smith"], m.params
  end

  def test_trailing_param_with_colon
    m = Lib.parse("PRIVMSG #general :hello world")
    assert_equal ["#general", "hello world"], m.params
  end

  def test_trailing_param_empty_colon
    m = Lib.parse("QUIT :")
    assert_equal [""], m.params
  end

  def test_numeric_command
    m = Lib.parse(":irc.test 001 alice :Welcome!")
    assert_equal "001",            m.command
    assert_equal "irc.test",       m.prefix
    assert_equal ["alice", "Welcome!"], m.params
  end

  def test_parse_error_empty_line
    assert_raises(Lib::ParseError) { Lib.parse("") }
  end

  def test_parse_error_whitespace_only
    assert_raises(Lib::ParseError) { Lib.parse("   ") }
  end

  def test_parse_error_nil
    assert_raises(Lib::ParseError) { Lib.parse(nil) }
  end

  def test_parse_error_prefix_no_command
    assert_raises(Lib::ParseError) { Lib.parse(":prefix") }
  end

  def test_ping_with_server
    m = Lib.parse("PING irc.test")
    assert_equal "PING",       m.command
    assert_equal ["irc.test"], m.params
  end

  def test_join_channel
    m = Lib.parse("JOIN #ruby")
    assert_equal "JOIN",    m.command
    assert_equal ["#ruby"], m.params
  end

  def test_mode_command
    m = Lib.parse("MODE #ruby +o alice")
    assert_equal "MODE",                    m.command
    assert_equal ["#ruby", "+o", "alice"],  m.params
  end

  def test_topic_with_spaces
    m = Lib.parse("TOPIC #ruby :Ruby Programming Language")
    assert_equal "TOPIC",                           m.command
    assert_equal ["#ruby", "Ruby Programming Language"], m.params
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# Serialize tests
# ─────────────────────────────────────────────────────────────────────────────

class TestSerialize < Minitest::Test
  def test_command_only
    m = Lib::Message.new(command: "PING")
    assert_equal "PING", Lib.serialize(m)
  end

  def test_command_with_prefix
    m = Lib::Message.new(prefix: "irc.test", command: "NOTICE")
    assert_equal ":irc.test NOTICE", Lib.serialize(m)
  end

  def test_command_with_params
    m = Lib::Message.new(command: "NICK", params: ["alice"])
    assert_equal "NICK alice", Lib.serialize(m)
  end

  def test_trailing_param_with_spaces_gets_colon
    m = Lib::Message.new(command: "PRIVMSG", params: ["#general", "hello world"])
    assert_equal "PRIVMSG #general :hello world", Lib.serialize(m)
  end

  def test_single_word_last_param_no_colon
    # "Welcome!" has no space → no leading colon needed
    m = Lib::Message.new(prefix: "irc.test", command: "001",
                         params: ["alice", "Welcome!"])
    assert_equal ":irc.test 001 alice Welcome!", Lib.serialize(m)
  end

  def test_empty_trailing_param_gets_colon
    m = Lib::Message.new(command: "QUIT", params: [""])
    assert_equal "QUIT :", Lib.serialize(m)
  end

  def test_full_welcome_message
    m = Lib::Message.new(
      prefix:  "irc.test",
      command: "001",
      params:  ["alice", "Welcome to the IRC Network, alice!alice@127.0.0.1"]
    )
    wire = Lib.serialize(m)
    assert wire.start_with?(":irc.test 001 alice :")
    assert wire.include?("Welcome to the IRC Network")
  end

  def test_pong_response
    m = Lib::Message.new(prefix: "irc.test", command: "PONG",
                         params: ["irc.test", "irc.test"])
    assert_equal ":irc.test PONG irc.test irc.test", Lib.serialize(m)
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# Message tests
# ─────────────────────────────────────────────────────────────────────────────

class TestMessage < Minitest::Test
  def test_equality_same
    a = Lib::Message.new(command: "NICK", params: ["alice"])
    b = Lib::Message.new(command: "NICK", params: ["alice"])
    assert_equal a, b
  end

  def test_equality_different_command
    a = Lib::Message.new(command: "NICK", params: ["alice"])
    b = Lib::Message.new(command: "USER", params: ["alice"])
    refute_equal a, b
  end

  def test_equality_different_params
    a = Lib::Message.new(command: "NICK", params: ["alice"])
    b = Lib::Message.new(command: "NICK", params: ["bob"])
    refute_equal a, b
  end

  def test_equality_prefix_matters
    a = Lib::Message.new(prefix: "irc.test", command: "NOTICE")
    b = Lib::Message.new(prefix: nil,         command: "NOTICE")
    refute_equal a, b
  end

  def test_inspect_contains_command
    m = Lib::Message.new(command: "PING", params: ["irc.test"])
    assert_match(/PING/, m.inspect)
  end

  def test_params_default_empty
    m = Lib::Message.new(command: "PING")
    assert_equal [], m.params
  end

  def test_not_equal_to_non_message
    m = Lib::Message.new(command: "PING")
    refute_equal m, "PING"
    refute_equal m, nil
  end
end
