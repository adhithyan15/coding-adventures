// Package irc_proto provides pure IRC message parsing and serialization (RFC 1459).
package irc_proto

import (
	"fmt"
	"strings"
)

const Version = "0.1.0"
const maxParams = 15

// Message is a single parsed IRC protocol message.
type Message struct {
	Prefix  string
	Command string
	Params  []string
}

// ParseError is returned when a raw line cannot be understood as an IRC message.
type ParseError struct{ msg string }

func (e *ParseError) Error() string { return e.msg }

func newParseError(format string, args ...interface{}) *ParseError {
	return &ParseError{msg: fmt.Sprintf(format, args...)}
}

// Parse parses a single IRC message line into a *Message.
// The line must already have its trailing CRLF stripped.
// Returns ParseError for empty, whitespace-only, or command-less input.
func Parse(line string) (*Message, error) {
	if len(strings.TrimSpace(line)) == 0 {
		return nil, newParseError("empty or whitespace-only line: %q", line)
	}
	rest := line
	var prefix string
	if strings.HasPrefix(rest, ":") {
		sp := strings.Index(rest, " ")
		if sp == -1 {
			return nil, newParseError("line has prefix but no command: %q", line)
		}
		prefix = rest[1:sp]
		rest = rest[sp+1:]
	}
	si := strings.Index(rest, " ")
	var command string
	if si == -1 {
		command = strings.ToUpper(rest)
		rest = ""
	} else {
		command = strings.ToUpper(rest[:si])
		rest = rest[si+1:]
	}
	if command == "" {
		return nil, newParseError("could not extract command from line: %q", line)
	}
	var params []string
	for rest != "" {
		if strings.HasPrefix(rest, ":") {
			params = append(params, rest[1:])
			break
		}
		sp := strings.Index(rest, " ")
		if sp == -1 {
			params = append(params, rest)
			break
		}
		params = append(params, rest[:sp])
		rest = rest[sp+1:]
		if len(params) == maxParams {
			break
		}
	}
	return &Message{Prefix: prefix, Command: command, Params: params}, nil
}

// Serialize serializes a *Message back to IRC wire format (CRLF-terminated).
func Serialize(msg *Message) []byte {
	var parts []string
	if msg.Prefix != "" {
		parts = append(parts, ":"+msg.Prefix)
	}
	parts = append(parts, msg.Command)
	for i, param := range msg.Params {
		if i == len(msg.Params)-1 && strings.Contains(param, " ") {
			parts = append(parts, ":"+param)
		} else {
			parts = append(parts, param)
		}
	}
	return []byte(strings.Join(parts, " ") + "\r\n")
}
