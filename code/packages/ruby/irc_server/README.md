# coding_adventures_irc_server

RFC 1459 IRC server state machine. Level 2 of the coding-adventures IRC stack.

Pure state machine with no I/O — consumes `Message` values and produces
`[conn_id, Message]` response pairs. Handles all standard IRC commands:
NICK, USER, JOIN, PART, PRIVMSG, NOTICE, TOPIC, KICK, INVITE, MODE,
PING, PONG, AWAY, WHOIS, WHO, OPER, QUIT, LIST, NAMES.

## Usage

```ruby
require "coding_adventures/irc_server"

server = CodingAdventures::IrcServer::IRCServer.new(
  server_name: "irc.example.com",
  motd: ["Welcome!"]
)

server.on_connect(1, "192.168.1.10")
server.on_message(1, CodingAdventures::IrcProto.parse("NICK alice"))
responses = server.on_message(1, CodingAdventures::IrcProto.parse("USER alice 0 * :Alice"))
# responses contains 001 RPL_WELCOME and full welcome sequence
```

## Running tests

```
bundle install && bundle exec rake test
```
