# Changelog

All notable changes to `irc-server` (Go) will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial Go implementation ported from the Python reference implementation.
- `IRCServer` struct with `clients`, `channels`, and `nicks` maps.
- `NewIRCServer(serverName, motd, operPassword)` constructor.
- `OnConnect(connID, host)` — creates a client record; returns nothing.
- `OnMessage(connID, msg)` — dispatches to per-command handler; returns `[]Response`.
  - Pre-registration gate: only NICK, USER, CAP, PASS, QUIT allowed until 001.
  - Full welcome sequence (001, 002, 003, 004, MOTD or 422).
- `OnDisconnect(connID)` — broadcasts QUIT to channel peers; cleans up state.
- 21 command handlers: NICK, USER, CAP, PASS, QUIT, JOIN, PART, PRIVMSG,
  NOTICE, NAMES, LIST, TOPIC, KICK, INVITE, MODE, PING, PONG, AWAY, WHOIS,
  WHO, OPER.
- Nick validation via compiled RFC 1459 regex (max 9 chars).
- Away auto-reply: PRIVMSG to away user triggers 301 RPL_AWAY back to sender.
- NOTICE never triggers auto-replies (per spec).
- Channel operator tracking: first member to JOIN becomes op.
- KICK enforces channel operator requirement.
- 93% statement coverage across 80+ unit tests.
