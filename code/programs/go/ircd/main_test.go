package main

import (
	"bufio"
	"net"
	"strings"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// parseArgs unit tests
// ---------------------------------------------------------------------------

func TestParseArgs_Defaults(t *testing.T) {
	cfg, err := parseArgs([]string{})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.host != "0.0.0.0" {
		t.Errorf("host: got %q, want %q", cfg.host, "0.0.0.0")
	}
	if cfg.port != 6667 {
		t.Errorf("port: got %d, want 6667", cfg.port)
	}
	if cfg.serverName != "irc.local" {
		t.Errorf("serverName: got %q, want %q", cfg.serverName, "irc.local")
	}
}

func TestParseArgs_CustomValues(t *testing.T) {
	cfg, err := parseArgs([]string{
		"--host", "127.0.0.1",
		"--port", "6668",
		"--server-name", "irc.example.com",
		"--motd", "Hello,World",
		"--oper-password", "secret",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.host != "127.0.0.1" {
		t.Errorf("host: got %q", cfg.host)
	}
	if cfg.port != 6668 {
		t.Errorf("port: got %d", cfg.port)
	}
	if cfg.serverName != "irc.example.com" {
		t.Errorf("serverName: got %q", cfg.serverName)
	}
	if cfg.operPassword != "secret" {
		t.Errorf("operPassword: got %q", cfg.operPassword)
	}
}

func TestParseArgs_InvalidPort(t *testing.T) {
	_, err := parseArgs([]string{"--port", "99999"})
	if err == nil {
		t.Error("expected error for port out of range")
	}
}

func TestParseArgs_MultiLineMOTD(t *testing.T) {
	cfg, err := parseArgs([]string{"--motd", "line1,line2,line3"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(cfg.motd) != 3 {
		t.Errorf("expected 3 MOTD lines, got %d", len(cfg.motd))
	}
	if cfg.motd[0] != "line1" || cfg.motd[1] != "line2" || cfg.motd[2] != "line3" {
		t.Errorf("wrong MOTD lines: %v", cfg.motd)
	}
}

// ---------------------------------------------------------------------------
// Integration test helpers
// ---------------------------------------------------------------------------

// startTestServer starts an ircd on a random free port and returns the
// address and a stop function.
func startTestServer(t *testing.T) (addr string, stop func()) {
	t.Helper()

	// Find a free port.
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to find free port: %v", err)
	}
	addr = ln.Addr().String()
	ln.Close()

	parts := strings.Split(addr, ":")
	port := parts[len(parts)-1]

	cfg, err := parseArgs([]string{
		"--host", "127.0.0.1",
		"--port", port,
		"--server-name", "irc.test",
		"--motd", "Test server",
	})
	if err != nil {
		t.Fatalf("parseArgs: %v", err)
	}

	stopCh := make(chan struct{})
	stopped := make(chan struct{})

	go func() {
		defer close(stopped)
		runLoop(cfg, stopCh)
	}()

	// Wait for the server to come up.
	deadline := time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline) {
		c, err := net.DialTimeout("tcp", addr, 100*time.Millisecond)
		if err == nil {
			c.Close()
			break
		}
		time.Sleep(10 * time.Millisecond)
	}

	return addr, func() {
		close(stopCh)
		<-stopped
	}
}

// ircClient is a minimal IRC client for integration tests.
type ircClient struct {
	conn net.Conn
	r    *bufio.Reader
}

func dial(t *testing.T, addr string) *ircClient {
	t.Helper()
	conn, err := net.DialTimeout("tcp", addr, 2*time.Second)
	if err != nil {
		t.Fatalf("dial: %v", err)
	}
	return &ircClient{conn: conn, r: bufio.NewReader(conn)}
}

func (c *ircClient) send(line string) {
	c.conn.Write([]byte(line + "\r\n"))
}

func (c *ircClient) readLine(t *testing.T) string {
	t.Helper()
	c.conn.SetReadDeadline(time.Now().Add(3 * time.Second))
	line, err := c.r.ReadString('\n')
	if err != nil {
		t.Fatalf("readLine: %v", err)
	}
	return strings.TrimRight(line, "\r\n")
}

// readUntil reads lines until it sees one matching the predicate.
func (c *ircClient) readUntil(t *testing.T, pred func(string) bool) string {
	t.Helper()
	for i := 0; i < 50; i++ {
		line := c.readLine(t)
		if pred(line) {
			return line
		}
	}
	t.Fatal("readUntil: did not find matching line within 50 lines")
	return ""
}

func (c *ircClient) close() { c.conn.Close() }

// register sends NICK+USER and waits for 001 welcome.
func (c *ircClient) register(t *testing.T, nick, user, realname string) {
	t.Helper()
	c.send("NICK " + nick)
	c.send("USER " + user + " 0 * :" + realname)
	c.readUntil(t, func(s string) bool { return strings.Contains(s, " 001 ") })
}

// ---------------------------------------------------------------------------
// Integration tests
// ---------------------------------------------------------------------------

// TestIntegration_Registration verifies the full NICK+USER registration flow.
func TestIntegration_Registration(t *testing.T) {
	addr, stop := startTestServer(t)
	defer stop()

	c := dial(t, addr)
	defer c.close()

	c.send("NICK alice")
	c.send("USER alice 0 * :Alice Smith")

	line := c.readUntil(t, func(s string) bool {
		return strings.Contains(s, " 001 ")
	})
	if !strings.Contains(line, "001") {
		t.Errorf("expected 001 welcome, got: %q", line)
	}
}

// TestIntegration_ServerNameInWelcome verifies the server name appears in 001.
func TestIntegration_ServerNameInWelcome(t *testing.T) {
	addr, stop := startTestServer(t)
	defer stop()

	c := dial(t, addr)
	defer c.close()

	c.register(t, "bob", "bob", "Bob")

	// The 001 line was already consumed by register; but server name appears
	// in the prefix of many messages. Let's check YOURHOST (002).
	line := c.readUntil(t, func(s string) bool {
		return strings.Contains(s, "irc.test")
	})
	if !strings.Contains(line, "irc.test") {
		t.Errorf("expected server name, got: %q", line)
	}
}

// TestIntegration_JoinPrivmsg verifies two clients can JOIN and exchange PRIVMSG.
func TestIntegration_JoinPrivmsg(t *testing.T) {
	addr, stop := startTestServer(t)
	defer stop()

	alice := dial(t, addr)
	defer alice.close()
	bob := dial(t, addr)
	defer bob.close()

	alice.register(t, "alice", "alice", "Alice")
	bob.register(t, "bob", "bob", "Bob")

	// Both join #test.
	alice.send("JOIN #test")
	alice.readUntil(t, func(s string) bool { return strings.Contains(s, "JOIN") })

	bob.send("JOIN #test")
	bob.readUntil(t, func(s string) bool { return strings.Contains(s, "JOIN") })
	// Alice gets bob's join notification.
	alice.readUntil(t, func(s string) bool { return strings.Contains(s, "bob") && strings.Contains(s, "JOIN") })

	// Alice sends a message.
	alice.send("PRIVMSG #test :hello bob")

	// Bob should receive it.
	line := bob.readUntil(t, func(s string) bool { return strings.Contains(s, "hello bob") })
	if !strings.Contains(line, "PRIVMSG") {
		t.Errorf("expected PRIVMSG, got: %q", line)
	}
}

// TestIntegration_PingPong verifies PING gets a PONG response.
func TestIntegration_PingPong(t *testing.T) {
	addr, stop := startTestServer(t)
	defer stop()

	c := dial(t, addr)
	defer c.close()

	c.register(t, "alice", "alice", "Alice")

	c.send("PING :mytoken")
	line := c.readUntil(t, func(s string) bool { return strings.Contains(s, "PONG") })
	if !strings.Contains(line, "mytoken") {
		t.Errorf("expected PONG with token, got: %q", line)
	}
}

// TestIntegration_NickInUse verifies 433 ERR_NICKNAMEINUSE.
func TestIntegration_NickInUse(t *testing.T) {
	addr, stop := startTestServer(t)
	defer stop()

	alice := dial(t, addr)
	defer alice.close()
	bob := dial(t, addr)
	defer bob.close()

	alice.register(t, "alice", "alice", "Alice")

	bob.send("NICK alice")
	line := bob.readUntil(t, func(s string) bool { return strings.Contains(s, " 433 ") })
	if !strings.Contains(line, "433") {
		t.Errorf("expected 433 ERR_NICKNAMEINUSE, got: %q", line)
	}
}

// TestIntegration_MalformedLine verifies the server handles malformed input gracefully.
func TestIntegration_MalformedLine(t *testing.T) {
	addr, stop := startTestServer(t)
	defer stop()

	c := dial(t, addr)
	defer c.close()

	// Send a whitespace-only line (malformed -- should be silently ignored).
	c.send("   ")
	// Then do a valid registration to verify the server is still alive.
	c.register(t, "alice", "alice", "Alice")
}

// TestIntegration_QuitWithError verifies QUIT sends an ERROR response.
func TestIntegration_QuitWithError(t *testing.T) {
	addr, stop := startTestServer(t)
	defer stop()

	c := dial(t, addr)
	defer c.close()

	c.register(t, "alice", "alice", "Alice")

	c.send("QUIT :bye")
	line := c.readUntil(t, func(s string) bool { return strings.Contains(s, "ERROR") })
	if !strings.Contains(line, "ERROR") {
		t.Errorf("expected ERROR after QUIT, got: %q", line)
	}
}
