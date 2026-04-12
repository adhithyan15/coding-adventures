package irc_proto

import (
	"bytes"
	"testing"
)

func TestParse_EmptyLine(t *testing.T) {
	for _, c := range []string{"", "   ", "\t"} {
		_, err := Parse(c)
		if err == nil {
			t.Errorf("Parse(%q) expected error", c)
		}
		if _, ok := err.(*ParseError); !ok {
			t.Errorf("expected *ParseError, got %T", err)
		}
	}
}

func TestParse_PrefixOnly(t *testing.T) {
	if _, err := Parse(":irc.local"); err == nil {
		t.Error("expected error for prefix-only line")
	}
}

func TestParse_SimpleCommand(t *testing.T) {
	msg, err := Parse("PING")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if msg.Command != "PING" || len(msg.Params) != 0 {
		t.Errorf("unexpected: %+v", msg)
	}
}

func TestParse_CommandUppercase(t *testing.T) {
	msg, err := Parse("join #foo")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if msg.Command != "JOIN" {
		t.Errorf("expected JOIN, got %q", msg.Command)
	}
}

func TestParse_Nick(t *testing.T) {
	msg, err := Parse("NICK alice")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if msg.Command != "NICK" || len(msg.Params) != 1 || msg.Params[0] != "alice" {
		t.Errorf("unexpected: %+v", msg)
	}
}

func TestParse_WithPrefix(t *testing.T) {
	msg, err := Parse(":irc.local 001 alice :Welcome!")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if msg.Prefix != "irc.local" || msg.Command != "001" {
		t.Errorf("wrong prefix/cmd: %+v", msg)
	}
	if len(msg.Params) != 2 || msg.Params[0] != "alice" || msg.Params[1] != "Welcome!" {
		t.Errorf("wrong params: %v", msg.Params)
	}
}

func TestParse_NickMaskPrefix(t *testing.T) {
	msg, err := Parse(":alice!alice@host PRIVMSG #c :hello world")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if msg.Prefix != "alice!alice@host" {
		t.Errorf("wrong prefix: %q", msg.Prefix)
	}
	if len(msg.Params) != 2 || msg.Params[1] != "hello world" {
		t.Errorf("wrong params: %v", msg.Params)
	}
}

func TestParse_TrailingParamWithSpaces(t *testing.T) {
	msg, err := Parse("PRIVMSG #chan :this has spaces")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(msg.Params) != 2 || msg.Params[1] != "this has spaces" {
		t.Errorf("wrong params: %v", msg.Params)
	}
}

func TestParse_UserCommand(t *testing.T) {
	msg, err := Parse("USER alice 0 * :Alice Smith")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(msg.Params) != 4 || msg.Params[3] != "Alice Smith" {
		t.Fatalf("wrong params: %v", msg.Params)
	}
}

func TestParse_MaxParams(t *testing.T) {
	line := "CMD"
	for i := 0; i < 20; i++ {
		line += " param"
	}
	msg, err := Parse(line)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(msg.Params) > maxParams {
		t.Errorf("too many params: %d", len(msg.Params))
	}
}

func TestParse_NumericCommand(t *testing.T) {
	msg, err := Parse(":irc.local 433 * alice :Nick in use")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if msg.Command != "433" {
		t.Errorf("expected 433, got %q", msg.Command)
	}
}

func TestParse_EmptyTrailingParam(t *testing.T) {
	msg, err := Parse("CMD :")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(msg.Params) != 1 || msg.Params[0] != "" {
		t.Errorf("expected empty string param, got %v", msg.Params)
	}
}

func TestSerialize_BareCommand(t *testing.T) {
	got := Serialize(&Message{Command: "PING"})
	want := []byte("PING\r\n")
	if !bytes.Equal(got, want) {
		t.Errorf("got %q, want %q", got, want)
	}
}

func TestSerialize_WithParam(t *testing.T) {
	got := Serialize(&Message{Command: "NICK", Params: []string{"alice"}})
	want := []byte("NICK alice\r\n")
	if !bytes.Equal(got, want) {
		t.Errorf("got %q, want %q", got, want)
	}
}

func TestSerialize_WithPrefix(t *testing.T) {
	msg := &Message{Prefix: "irc.local", Command: "001", Params: []string{"alice", "Welcome!"}}
	got := Serialize(msg)
	want := []byte(":irc.local 001 alice Welcome!\r\n")
	if !bytes.Equal(got, want) {
		t.Errorf("got %q, want %q", got, want)
	}
}

func TestSerialize_TrailingParamWithSpaces(t *testing.T) {
	msg := &Message{Command: "PRIVMSG", Params: []string{"#chan", "hello world"}}
	got := Serialize(msg)
	want := []byte("PRIVMSG #chan :hello world\r\n")
	if !bytes.Equal(got, want) {
		t.Errorf("got %q, want %q", got, want)
	}
}

func TestRoundTrip(t *testing.T) {
	cases := []*Message{
		{Command: "PING"},
		{Command: "NICK", Params: []string{"bob"}},
		{Prefix: "irc.example.com", Command: "001", Params: []string{"bob", "Welcome!"}},
		{Prefix: "alice!a@host", Command: "PRIVMSG", Params: []string{"#chan", "hello world"}},
	}
	for _, tc := range cases {
		wire := Serialize(tc)
		// Strip the trailing \r\n before parsing.
		lineStr := string(wire[:len(wire)-2])
		got, err := Parse(lineStr)
		if err != nil {
			t.Fatalf("Parse failed: %v", err)
		}
		if got.Prefix != tc.Prefix || got.Command != tc.Command {
			t.Errorf("round-trip: got %+v want %+v", got, tc)
		}
		for i := range got.Params {
			if got.Params[i] != tc.Params[i] {
				t.Errorf("param[%d]: got %q want %q", i, got.Params[i], tc.Params[i])
			}
		}
	}
}

func TestParseError_Message(t *testing.T) {
	err := newParseError("test: %q", "input")
	if err.Error() != `test: "input"` {
		t.Errorf("unexpected: %q", err.Error())
	}
}
