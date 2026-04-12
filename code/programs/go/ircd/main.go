// Command ircd is the Go IRC server — the top of the IRC stack.
//
// It wires four pure packages into a running TCP server:
//
//	irc-proto     — message parsing and serialisation
//	irc-framing   — CRLF-based byte-stream framing
//	irc-server    — pure IRC state machine
//	irc-net-stdlib — goroutine-per-connection TCP event loop
//
// Architecture
//
// ircd itself contains no IRC protocol logic. It is purely a wiring layer:
//
//	TCP socket
//	   ↓ raw bytes
//	EventLoop                 ← irc-net-stdlib
//	   ↓ connID, bytes
//	DriverHandler.OnData()    ← THIS PROGRAM
//	   ↓ per-conn Framer
//	Framer.Frames()           ← irc-framing
//	   ↓ complete line
//	irc_proto.Parse()         ← irc-proto
//	   ↓ Message
//	IRCServer.OnMessage()     ← irc-server
//	   ↓ []Response
//	irc_proto.Serialize()     ← irc-proto
//	   ↓ bytes
//	loop.SendTo()             ← irc-net-stdlib
//	   ↓ wire
//
// Each layer is a pure function (or close to it). Dependencies only point
// downward. This makes each package independently testable.
package main

import (
	"flag"
	"fmt"
	"os"
	"os/signal"
	"strings"
	"sync"
	"syscall"

	irc_framing "github.com/adhithyan15/coding-adventures/code/packages/go/irc-framing"
	irc_net_stdlib "github.com/adhithyan15/coding-adventures/code/packages/go/irc-net-stdlib"
	irc_proto "github.com/adhithyan15/coding-adventures/code/packages/go/irc-proto"
	irc_server "github.com/adhithyan15/coding-adventures/code/packages/go/irc-server"
)

// ---------------------------------------------------------------------------
// DriverHandler — the adapter between irc-net-stdlib and irc-server
// ---------------------------------------------------------------------------

// DriverHandler implements irc_net_stdlib.Handler.
//
// It bridges the TCP event loop and the IRC state machine by:
//  1. Maintaining a per-connection Framer to reassemble CRLF-delimited lines.
//  2. Forwarding parsed messages to IRCServer.
//  3. Serialising each Response and sending it back down the wire.
type DriverHandler struct {
	server    *irc_server.IRCServer
	loop      *irc_net_stdlib.EventLoop
	framers   map[irc_net_stdlib.ConnID]*irc_framing.Framer
	framersMu sync.RWMutex
}

// newDriverHandler creates a DriverHandler.
func newDriverHandler(server *irc_server.IRCServer, loop *irc_net_stdlib.EventLoop) *DriverHandler {
	return &DriverHandler{
		server:  server,
		loop:    loop,
		framers: make(map[irc_net_stdlib.ConnID]*irc_framing.Framer),
	}
}

// OnConnect creates a per-connection Framer and notifies IRCServer.
func (h *DriverHandler) OnConnect(connID irc_net_stdlib.ConnID, host string) {
	h.framersMu.Lock()
	h.framers[connID] = irc_framing.NewFramer()
	h.framersMu.Unlock()

	// IRCServer.OnConnect returns no responses (clients must speak first).
	h.server.OnConnect(irc_server.ConnID(connID), host)
}

// OnData feeds bytes into the per-connection Framer, extracts complete lines,
// parses each with irc_proto.Parse, dispatches to IRCServer.OnMessage, and
// calls loop.SendTo for each response.
func (h *DriverHandler) OnData(connID irc_net_stdlib.ConnID, data []byte) {
	h.framersMu.RLock()
	framer := h.framers[connID]
	h.framersMu.RUnlock()

	if framer == nil {
		return
	}

	framer.Feed(data)
	for _, frame := range framer.Frames() {
		msg, err := irc_proto.Parse(string(frame))
		if err != nil {
			// Malformed line -- skip it. A real server might log this.
			continue
		}

		responses := h.server.OnMessage(irc_server.ConnID(connID), msg)
		for _, r := range responses {
			h.loop.SendTo(irc_net_stdlib.ConnID(r.ConnID), irc_proto.Serialize(r.Msg))
		}
	}
}

// OnDisconnect broadcasts QUIT via IRCServer.OnDisconnect and removes the framer.
func (h *DriverHandler) OnDisconnect(connID irc_net_stdlib.ConnID) {
	responses := h.server.OnDisconnect(irc_server.ConnID(connID))
	for _, r := range responses {
		h.loop.SendTo(irc_net_stdlib.ConnID(r.ConnID), irc_proto.Serialize(r.Msg))
	}

	h.framersMu.Lock()
	delete(h.framers, connID)
	h.framersMu.Unlock()
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

// Config holds the server configuration parsed from command-line flags.
type Config struct {
	host         string
	port         int
	serverName   string
	motd         []string
	operPassword string
}

// parseArgs parses command-line arguments into a Config.
//
// Supported flags:
//   - --host         : IP address to bind to (default "0.0.0.0")
//   - --port         : TCP port (default 6667, range 0-65535)
//   - --server-name  : Hostname shown in welcome messages (default "irc.local")
//   - --motd         : MOTD lines, comma-separated (default "Welcome.")
//   - --oper-password: Password for the OPER command (default " ")
func parseArgs(args []string) (*Config, error) {
	fs := flag.NewFlagSet("ircd", flag.ContinueOnError)

	host := fs.String("host", "0.0.0.0", "IP address to bind to")
	port := fs.Int("port", 6667, "TCP port to listen on (0-65535)")
	serverName := fs.String("server-name", "irc.local", "Server hostname shown in welcome messages")
	motdRaw := fs.String("motd", "Welcome.", "MOTD lines (comma-separated for multiple)")
	operPassword := fs.String("oper-password", " ", "Password for the OPER command (empty = disabled)")

	if err := fs.Parse(args); err != nil {
		return nil, err
	}

	if *port < 0 || *port > 65535 {
		return nil, fmt.Errorf("port %d out of range (0-65535)", *port)
	}

	motdLines := strings.Split(*motdRaw, ",")

	return &Config{
		host:         *host,
		port:         *port,
		serverName:   *serverName,
		motd:         motdLines,
		operPassword: *operPassword,
	}, nil
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

// runLoop starts the IRC server with the given config and blocks until stopCh
// is closed. This is the testable core — tests can stop the server cleanly.
func runLoop(cfg *Config, stopCh <-chan struct{}) error {
	server := irc_server.NewIRCServer(cfg.serverName, cfg.motd, cfg.operPassword)
	loop := irc_net_stdlib.NewEventLoop()
	handler := newDriverHandler(server, loop)

	addr := fmt.Sprintf("%s:%d", cfg.host, cfg.port)

	// Stop the loop when stopCh is closed.
	go func() {
		<-stopCh
		loop.Stop()
	}()

	return loop.Run(addr, handler)
}

// run is the testable entry point. It parses args, wires up all the packages,
// installs signal handlers, and blocks until SIGINT or SIGTERM.
func run(args []string) error {
	cfg, err := parseArgs(args)
	if err != nil {
		return err
	}

	stopCh := make(chan struct{})

	// Install signal handlers for graceful shutdown.
	// SIGINT (Ctrl-C) and SIGTERM are the standard shutdown signals.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigCh
		fmt.Println("\nircd: shutting down...")
		close(stopCh)
	}()

	addr := fmt.Sprintf("%s:%d", cfg.host, cfg.port)
	fmt.Printf("ircd: listening on %s (server name: %s)\n", addr, cfg.serverName)
	return runLoop(cfg, stopCh)
}

// main is the thin wrapper that calls run and exits with code 1 on error.
func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintf(os.Stderr, "ircd: %v\n", err)
		os.Exit(1)
	}
}
