# Changelog

## [0.1.0] - 2026-04-12

### Added
- Initial implementation: `IRCServer` with full RFC 1459 command support
- `Client`, `Channel`, `ChannelMember` state structs
- All numeric reply constants (001–482)
- 20+ command handlers: NICK, USER, JOIN, PART, PRIVMSG, NOTICE, TOPIC,
  KICK, INVITE, MODE, PING, PONG, AWAY, WHOIS, WHO, OPER, QUIT, NAMES, LIST, CAP, PASS
