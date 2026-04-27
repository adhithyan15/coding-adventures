# Changelog — irc_server (Elixir)

## [0.1.0] — 2026-04-12

### Added

- `CodingAdventures.IrcServer.new/4` — initialise server state with server_name,
  version, MOTD lines, and optional OPER password
- `IrcServer.on_connect/3` — register a new unregistered client by conn_id
- `IrcServer.on_message/3` — dispatch an `IrcProto.Message` to the appropriate
  command handler and return `{new_state, [{conn_id, Message}]}`
- `IrcServer.on_disconnect/2` — clean up client state and notify channel peers
- Full RFC 1459 command support: CAP, NICK, USER, PASS, QUIT, JOIN, PART,
  PRIVMSG, NOTICE, NAMES, LIST, TOPIC, KICK, INVITE, MODE, PING, PONG,
  AWAY, WHOIS, WHO, OPER
- Registration flow: clients must send NICK + USER before any other commands
- Welcome burst (001–004) and MOTD (375/372/376) on registration
- Channel creation, membership tracking with `MapSet`, and topic storage
- KICK and INVITE restricted to channel operators
- ERR_UNKNOWNCOMMAND (421) for unrecognised commands
- ERR_NOSUCHNICK (401), ERR_NOSUCHCHANNEL (403), ERR_NOTONCHANNEL (442),
  ERR_CHANOPRIVSNEEDED (482) error replies
- `smsg/3` helper for building server-prefix messages
- `mask/1` helper for building `nick!user@host` user masks
- Comprehensive ExUnit test suite — 87 tests, 96.66% coverage
- Pure functional design — no side effects, no processes
- Port of Python reference implementation to idiomatic Elixir
