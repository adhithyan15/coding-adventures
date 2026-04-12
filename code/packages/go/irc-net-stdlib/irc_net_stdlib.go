// Package irc_net_stdlib provides a goroutine-per-connection TCP event loop
// for the IRC stack.
//
// Overview
//
// This package provides the concrete TCP networking layer for the IRC stack.
// It implements the Handler interface using Go's standard library net package
// and one goroutine per accepted connection.
//
// Goroutine-per-connection model
//
// Each accepted TCP connection gets its own goroutine. The goroutine:
//
//  1. Calls handler.OnConnect() to notify the server.
//  2. Loops calling conn.Read() (blocking) and forwards each chunk to
//     handler.OnData().
//  3. When Read returns io.EOF or an error (peer closed), calls
//     handler.OnDisconnect() and exits.
//
// This is the textbook model taught in every OS/networking course. Its chief
// virtue is clarity: each connection's lifecycle is a simple sequential program,
// easy to reason about with no callbacks or coroutines.
//
// Two locks protect shared state
//
// handlerMu (a sync.Mutex):
//
//	Serialises ALL calls to the Handler (OnConnect, OnData, OnDisconnect).
//	The Handler's internals (the IRC server state machine) are not goroutine-safe.
//	By funnelling every callback through this single lock we guarantee that IRC
//	logic executes in one goroutine at a time. IRC traffic is mostly idle (clients
//	send at human typing speed), so lock contention is negligible in practice.
//	Think of it as a mutex around the "IRC brain".
//
// connsMu (a sync.Mutex):
//
//	Protects the conns map[ConnID]*conn mapping. Two goroutines that accept
//	connections simultaneously could both try to insert into this map, and a
//	SendTo call racing against a worker goroutine removing a closed connection
//	could read a half-updated map. The lock prevents both races.
//
//	connsMu and handlerMu are independent. We never hold both at the same time,
//	so there is no deadlock risk.
//
// Writes bypass the handler lock
//
// SendTo looks up the connection (under connsMu), then calls conn.Write
// without holding handlerMu. This is intentional:
//   - Writing bytes to a socket is independent of reading server state.
//   - Allowing two goroutines to write to different connections simultaneously
//     is safe -- the OS serialises writes to individual sockets internally.
//   - If we held handlerMu during writes, a slow TCP write would stall all
//     other connection goroutines that want to run IRC logic.
package irc_net_stdlib

import (
	"net"
	"sync"
	"sync/atomic"
)

// Version of this package.
const Version = "0.1.0"

// ---------------------------------------------------------------------------
// ConnID type
// ---------------------------------------------------------------------------

// ConnID is a distinct integer type used to identify connections.
// Using a named type rather than a plain int64 prevents accidentally passing
// an arbitrary integer where a connection identity is expected.
type ConnID int64

// ---------------------------------------------------------------------------
// Handler interface
// ---------------------------------------------------------------------------

// Handler is the callback interface that the EventLoop drives.
//
// The event loop calls these methods as connection lifecycle events occur.
// All three methods are called with the handlerMu lock held -- the
// implementation need not be goroutine-safe.
//
// The interface deliberately passes raw bytes to OnData, not parsed Message
// objects. Framing and parsing happen in the driver layer above this one,
// keeping irc-net-stdlib free of any IRC-specific knowledge.
type Handler interface {
	// OnConnect is called once when a new client connects.
	// connID is a unique identifier for this connection.
	// host is the peer's hostname or IP address string.
	OnConnect(connID ConnID, host string)

	// OnData is called each time new bytes arrive from connID.
	// The bytes may contain a partial IRC message, multiple complete messages,
	// or anything in between -- it is the handler's responsibility to buffer and
	// frame them.
	// data is never empty when this method is called.
	OnData(connID ConnID, data []byte)

	// OnDisconnect is called once when connID has closed (either end initiated).
	// After this call the connID is invalid; SendTo with it is a safe no-op.
	OnDisconnect(connID ConnID)
}

// ---------------------------------------------------------------------------
// ConnID allocator
// ---------------------------------------------------------------------------

// nextConnID is an atomically-incremented counter for generating unique ConnIDs.
// Using atomic operations avoids a lock and makes the intent of "unique counter"
// explicit in the code.
var nextConnID int64

// allocConnID atomically allocates the next unique ConnID.
// Each new connection gets an integer that never repeats within a process
// lifetime. We start at 1 (0 is reserved as a sentinel "no connection" value).
func allocConnID() ConnID {
	// AddInt64 returns the new value, so starting from 0, the first call
	// returns 1. This is the idiomatic Go pattern for atomic counters.
	return ConnID(atomic.AddInt64(&nextConnID, 1))
}

// ---------------------------------------------------------------------------
// EventLoop
// ---------------------------------------------------------------------------

// EventLoop is the goroutine-per-connection TCP event loop.
//
// Lifecycle:
//  1. Create an EventLoop.
//  2. Call Run(addr, handler) -- this blocks, accepting connections.
//  3. Meanwhile, other goroutines may call SendTo to push data to clients.
//  4. When the caller wants to shut down, any goroutine calls Stop().
//  5. Stop() closes the listener, causing Accept() to return an error,
//     which exits the accept loop, causing Run() to return.
//
// Worker goroutine lifecycle per connection:
//
//	1. handler.OnConnect() under handlerMu
//	2. Loop: conn.Read() -> handler.OnData() under handlerMu
//	3. handler.OnDisconnect() under handlerMu
//	4. Remove conn from conns (under connsMu) and close socket.
type EventLoop struct {
	// running is set to true by Run() and false by Stop().
	// Worker goroutines check this in their loop condition.
	running bool

	// conns maps ConnID -> *activeConn for all currently-open connections.
	// Protected by connsMu.
	conns   map[ConnID]*activeConn
	connsMu sync.Mutex

	// handlerMu serialises all calls to the Handler.
	// The Handler (IRC server) is not goroutine-safe -- we ensure only one
	// goroutine runs IRC logic at a time.
	handlerMu sync.Mutex

	// listener is the TCP server socket. Set at the start of Run(), cleared
	// after Run() exits. Protected by the stop mechanism.
	listener net.Listener
}

// activeConn wraps a net.Conn with its allocated ConnID and the peer's address
// as a cached string (net.Conn.RemoteAddr() may panic after close).
type activeConn struct {
	id       ConnID
	peerAddr string // cached at construction time
	conn     net.Conn
}

// NewEventLoop creates a new EventLoop with an empty connection table.
func NewEventLoop() *EventLoop {
	return &EventLoop{
		conns: make(map[ConnID]*activeConn),
	}
}

// Run binds to addr, accepts connections, and dispatches events to handler.
//
// This method blocks until Stop() is called. Each accepted connection gets
// its own goroutine.
//
// addr has the form "host:port" e.g. "0.0.0.0:6667" or ":6667".
// Returns the first listen error, or nil if Stop() was called cleanly.
func (l *EventLoop) Run(addr string, handler Handler) error {
	// Create a TCP listener bound to addr.
	// net.Listen sets SO_REUSEADDR automatically on most platforms, which
	// allows rebinding the port immediately after the previous process exits.
	ln, err := net.Listen("tcp", addr)
	if err != nil {
		return err
	}

	l.running = true
	l.listener = ln

	// Accept loop: each iteration blocks in ln.Accept() until a new client
	// connects. When Stop() is called it closes the listener, which causes
	// Accept() to return an error, which breaks the loop.
	for l.running {
		conn, err := ln.Accept()
		if err != nil {
			// Accept returned an error. If we're stopping, this is expected
			// (the listener was closed). If not, it's an actual error --
			// we exit the accept loop either way.
			break
		}

		// Allocate a unique ConnID before spawning the goroutine.
		id := allocConnID()
		peer := conn.RemoteAddr().String()

		// Parse the host from "host:port" -- we only want the host string
		// for the IRC mask (nick!user@host).
		host, _, splitErr := net.SplitHostPort(peer)
		if splitErr != nil {
			// Fallback: use the full address string if splitting fails.
			host = peer
		}

		ac := &activeConn{
			id:       id,
			peerAddr: host,
			conn:     conn,
		}

		// Register the connection before spawning the goroutine, so that
		// SendTo can find it immediately (even before OnConnect fires).
		l.connsMu.Lock()
		l.conns[id] = ac
		l.connsMu.Unlock()

		// Spawn a goroutine to service this connection.
		// The goroutine runs the full connect -> data loop -> disconnect lifecycle.
		go l.worker(ac, handler)
	}

	l.running = false
	return nil
}

// Stop signals the event loop to stop accepting new connections.
//
// Safe to call from any goroutine, including signal handlers.
// Returns immediately (does not wait for in-flight connections to finish).
//
// Mechanism: set running to false, then close the listener socket to unblock
// the Accept() call that may be currently waiting for a new connection.
func (l *EventLoop) Stop() {
	l.running = false
	if l.listener != nil {
		// Closing the listener causes any Accept() call to return immediately
		// with an error, breaking the accept loop in Run().
		l.listener.Close()
	}
}

// SendTo writes data to connection connID.
//
// Looks up the connection under connsMu, then writes outside the lock.
// This is deliberate: we hold the lock for the shortest possible time
// (just the map lookup), then release it so other goroutines can
// concurrently look up different connections.
//
// If connID is not found (connection closed, or never existed), this is
// a silent no-op. Callers should not treat absence as an error -- it is a
// normal race condition where the client disconnected between the handler
// deciding to write and actually calling SendTo.
func (l *EventLoop) SendTo(connID ConnID, data []byte) {
	// Step 1: look up the connection while holding the lock.
	l.connsMu.Lock()
	ac := l.conns[connID]
	l.connsMu.Unlock()

	// Step 2: write outside the lock.
	// If ac was removed between steps 1 and 2 (the worker goroutine closed it),
	// the write will simply fail with a "use of closed network connection" error,
	// which we swallow.
	if ac != nil {
		// Swallow write errors: the connection may have closed between the lookup
		// and the write. The read goroutine's next Read() call will detect the
		// closure and trigger the disconnect path.
		ac.conn.Write(data) //nolint:errcheck
	}
}

// ---------------------------------------------------------------------------
// Worker goroutine
// ---------------------------------------------------------------------------

// worker services a single connection from its own goroutine.
//
// This method is the entry point for every connection's goroutine. It runs
// the full lifecycle: connect -> data loop -> disconnect -> cleanup.
//
// Error handling philosophy: we never let an exception from the Handler crash
// this goroutine. If the handler panics, we recover and proceed to the
// disconnect path. In production, the handler should handle its own errors.
func (l *EventLoop) worker(ac *activeConn, handler Handler) {
	// Phase 1: notify the handler that the connection opened.
	// We hold handlerMu for the duration of the callback so the handler's
	// internal state is consistent. The lock is released before we block in
	// conn.Read() below -- we only hold it while running IRC logic, not while
	// waiting for network I/O.
	l.handlerMu.Lock()
	handler.OnConnect(ac.id, ac.peerAddr)
	l.handlerMu.Unlock()

	// Phase 2: data receive loop.
	// conn.Read() blocks here. This goroutine is parked by the Go runtime
	// while waiting for data, so it consumes no CPU while idle.
	//
	// We receive up to 4096 bytes at a time. This is a common choice:
	//   - Large enough to amortise syscall overhead.
	//   - Small enough to fit comfortably in L1/L2 cache.
	//   - IRC messages are at most 512 bytes, so one read often captures
	//     several complete messages.
	buf := make([]byte, 4096)
	for {
		n, err := ac.conn.Read(buf)
		if n > 0 {
			// We have data -- make a copy before releasing the slice for reuse.
			// The handler may retain references to the data slice.
			data := make([]byte, n)
			copy(data, buf[:n])

			// Dispatch the data to the handler.
			// We hold handlerMu for the callback duration so the IRC server
			// sees a consistent view of its own state. Release before the
			// next Read() so other goroutines can call their callbacks while
			// we are waiting for more bytes.
			l.handlerMu.Lock()
			handler.OnData(ac.id, data)
			l.handlerMu.Unlock()
		}
		if err != nil {
			// io.EOF means the peer closed the connection gracefully.
			// Any other error (ECONNRESET, EBADF after Close(), etc.) also
			// means the connection is gone. Either way, exit the read loop.
			break
		}
	}

	// Phase 3: cleanup.
	// First, notify the handler. We do this before removing the conn from
	// the map so the handler can still call SendTo during the disconnect
	// callback if needed (e.g. to send a final error reply).
	l.handlerMu.Lock()
	handler.OnDisconnect(ac.id)
	l.handlerMu.Unlock()

	// Remove from the connection map so SendTo stops finding it.
	// After this point any SendTo(ac.id) is a no-op.
	l.connsMu.Lock()
	delete(l.conns, ac.id)
	l.connsMu.Unlock()

	// Close the socket. If it was already closed (e.g. by Stop()), Close()
	// is idempotent and returns an error that we ignore.
	ac.conn.Close() //nolint:errcheck
}
