# Changelog — irc-server

All notable changes to this crate will be documented here.

## [0.1.0] — 2026-04-12

### Added

- `ConnId(u64)` newtype wrapper for connection identifiers
- `Response { conn_id, msg }` struct for outbound messages
- `Client` struct with full state: nick, username, realname, hostname, channels, away_message, is_oper
- `ChannelMember` struct with is_operator, has_voice flags
- `Channel` struct with members, topic, modes, ban_list
- `valid_nick(nick: &str) -> bool` — RFC 1459 nickname validation
- `IRCServer::new(server_name, motd, oper_password)` constructor
- `IRCServer::on_connect(conn_id, host) -> Vec<Response>`
- `IRCServer::on_message(conn_id, msg) -> Vec<Response>` — dispatches to:
  - CAP, PASS, NICK, USER, QUIT
  - JOIN, PART, PRIVMSG, NOTICE, NAMES, LIST, TOPIC, KICK, INVITE
  - MODE, PING, PONG, AWAY, WHOIS, WHO, OPER
- `IRCServer::on_disconnect(conn_id) -> Vec<Response>`
- RFC 1459 welcome sequence (001–004, 251, 375/372/376)
- ERR_NOTREGISTERED gate for pre-registration clients
- Comprehensive unit tests covering all major commands and error paths
