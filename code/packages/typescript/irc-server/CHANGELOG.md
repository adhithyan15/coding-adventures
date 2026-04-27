# Changelog — @coding-adventures/irc-server

All notable changes to this package will be documented here.

## [0.1.0] — 2026-04-12

### Added

- Initial TypeScript port of the Python `irc-server` package
- `ConnId` branded type and `Response = [ConnId, Message]` tuple type
- `IRCServer` class with `onConnect()`, `onMessage()`, `onDisconnect()` methods
- Full RFC 1459 IRC state machine: clients, channels, nick index
- Command handlers: NICK, USER, CAP, QUIT, PASS, JOIN, PART, PRIVMSG, NOTICE, NAMES, LIST, TOPIC, KICK, INVITE, MODE, PING, PONG, AWAY, WHOIS, WHO, OPER
- All IRC numeric constants (RPL_WELCOME=001 through ERR_CHANOPRIVSNEEDED=482)
- Pre-registration gate: unregistered clients may only send NICK/USER/CAP/QUIT/PASS
- Nick validation via RFC 1459 regex (1-9 chars, valid character set)
- Channel operator assignment (first member of new channel becomes op)
- Welcome sequence (001, 002, 003, 004, 251, 375, 372..., 376)
- Comprehensive test suite: 79 tests, 92%+ line coverage
