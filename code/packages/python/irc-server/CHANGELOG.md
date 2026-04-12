# Changelog

## [0.1.0] - 2026-04-12

### Added

- `IRCServer` state machine with full RFC 1459 command set
- `Client`, `Channel`, `ChannelMember` dataclasses with literate docstrings
- `ConnId` NewType and `Response` type alias for type-safe transport integration
- Nick registration handshake: NICK + USER triggers 001–376 welcome sequence
  (001 RPL_WELCOME, 002 RPL_YOURHOST, 003 RPL_CREATED, 004 RPL_MYINFO,
  251 RPL_LUSERCLIENT, 375/372/376 MOTD block)
- Nick validation per RFC 1459 §2.3.1 (1–9 chars, restricted character set)
- Case-insensitive nick uniqueness enforcement (433 ERR_NICKNAMEINUSE)
- NICK change broadcasting to all channel peers
- Channel operations: JOIN (with operator auto-grant for first member),
  PART (with channel destruction when empty), NAMES (353+366),
  LIST (321+322+323), TOPIC (331/332 get, broadcast on set),
  KICK (operator-only, 482 guard), INVITE, MODE (query 324/221, set broadcast)
- Messaging: PRIVMSG and NOTICE with channel fanout (sender excluded),
  direct nick delivery, 401/403 error handling, 412 no-text guard,
  301 RPL_AWAY auto-reply for PRIVMSG to away users
- Server commands: PING→PONG, PONG (ignored), AWAY (306/305),
  WHOIS (311+312+319+301+318), WHO (352+315), OPER (381/464)
- Clean disconnect via QUIT (ERROR to quitter, QUIT broadcast to peers)
- Unexpected disconnect via `on_disconnect` (same broadcast cleanup)
- Pre-registration gate: 451 ERR_NOTREGISTERED for disallowed commands
- 421 ERR_UNKNOWNCOMMAND for unrecognised commands
- Full test suite (85 tests) covering all commands and edge cases
- 94% line coverage (exceeds 90% target)
- ruff-clean, mypy --strict clean
