# irc-server (Go)

Pure IRC state machine — channels, nicks, command dispatch.

## Overview

`irc-server` is the "brain" of the IRC stack. It implements RFC 1459 command
handling as a pure state machine: given a connection ID and a parsed message, it
returns a list of responses to send.

It knows nothing about TCP sockets, goroutines, or file I/O. Thread-safety is
the caller's responsibility (irc-net-stdlib serialises calls with a mutex).

## Architecture

```
(State, Input) -> (State', []Response)
```

State is mutated in place but the observable behaviour is pure-function-like:
the same input sequence always produces the same output sequence.

## API

```go
s := irc_server.NewIRCServer("irc.example.com", []string{"Welcome!"}, "operpassword")

// Client connects.
_ = s.OnConnect(connID, "127.0.0.1")

// Client sends a message; server returns responses to deliver.
responses := s.OnMessage(connID, msg)
for _, r := range responses {
    loop.SendTo(r.ConnID, irc_proto.Serialize(r.Msg))
}

// Client disconnects.
quitResponses := s.OnDisconnect(connID)
```

## Commands implemented

NICK, USER, CAP, PASS, QUIT, JOIN, PART, PRIVMSG, NOTICE, NAMES, LIST,
TOPIC, KICK, INVITE, MODE, PING, PONG, AWAY, WHOIS, WHO, OPER.

## Coverage

93% statement coverage.
