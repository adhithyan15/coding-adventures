package irc_net_stdlib

import (
	"fmt"
	"net"
	"sync"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

// testHandler is a simple Handler implementation that records lifecycle events.
// It is safe for use in tests where the EventLoop ensures serialised callbacks.
type testHandler struct {
	mu           sync.Mutex
	connected    map[ConnID]string // connID -> host
	disconnected map[ConnID]bool
	received     []testEvent
}

type testEvent struct {
	connID ConnID
	data   []byte
}

func newTestHandler() *testHandler {
	return &testHandler{
		connected:    make(map[ConnID]string),
		disconnected: make(map[ConnID]bool),
	}
}

func (h *testHandler) OnConnect(connID ConnID, host string) {
	h.mu.Lock()
	defer h.mu.Unlock()
	h.connected[connID] = host
}

func (h *testHandler) OnData(connID ConnID, data []byte) {
	h.mu.Lock()
	defer h.mu.Unlock()
	// Make a copy since the event loop may reuse the buffer.
	d := make([]byte, len(data))
	copy(d, data)
	h.received = append(h.received, testEvent{connID, d})
}

func (h *testHandler) OnDisconnect(connID ConnID) {
	h.mu.Lock()
	defer h.mu.Unlock()
	h.disconnected[connID] = true
}

func (h *testHandler) connCount() int {
	h.mu.Lock()
	defer h.mu.Unlock()
	return len(h.connected)
}

func (h *testHandler) isDisconnected(id ConnID) bool {
	h.mu.Lock()
	defer h.mu.Unlock()
	return h.disconnected[id]
}

func (h *testHandler) dataReceived() []testEvent {
	h.mu.Lock()
	defer h.mu.Unlock()
	result := make([]testEvent, len(h.received))
	copy(result, h.received)
	return result
}

// startTestLoop starts an event loop on an OS-assigned port (":0") and returns
// the loop, the handler, and the port it bound to.
// The caller must call loop.Stop() when done.
func startTestLoop(t *testing.T) (*EventLoop, *testHandler, int) {
	t.Helper()

	loop := NewEventLoop()
	handler := newTestHandler()

	// Listen on port 0 to get an OS-assigned ephemeral port.
	// We need to bind first to know the port, then pass the address to Run.
	// We use a net.Listener just to find a free port, then let Run bind again.
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to find free port: %v", err)
	}
	addr := ln.Addr().String()
	ln.Close() // release it so Run can bind

	errCh := make(chan error, 1)
	go func() {
		errCh <- loop.Run(addr, handler)
	}()

	// Parse the port out of addr.
	_, portStr, _ := net.SplitHostPort(addr)
	var port int
	fmt.Sscanf(portStr, "%d", &port)

	// Wait until the loop is actually listening.
	// We poll with a short dial to detect when the port is accepting connections.
	deadline := time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline) {
		c, err := net.DialTimeout("tcp", addr, 100*time.Millisecond)
		if err == nil {
			c.Close()
			break
		}
		time.Sleep(10 * time.Millisecond)
	}

	// Wait for the probe connection to disconnect so we start clean.
	deadline2 := time.Now().Add(1 * time.Second)
	for time.Now().Before(deadline2) {
		handler.mu.Lock()
		nDisconn := len(handler.disconnected)
		handler.mu.Unlock()
		if nDisconn > 0 {
			break
		}
		time.Sleep(5 * time.Millisecond)
	}

	return loop, handler, port
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

// TestEventLoop_AcceptsConnection verifies that the event loop accepts a TCP
// connection and fires OnConnect with the correct host.
func TestEventLoop_AcceptsConnection(t *testing.T) {
	loop, handler, port := startTestLoop(t)
	defer loop.Stop()

	conn, err := net.Dial("tcp", fmt.Sprintf("127.0.0.1:%d", port))
	if err != nil {
		t.Fatalf("failed to connect: %v", err)
	}
	defer conn.Close()

	// Wait for OnConnect to fire.
	deadline := time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline) {
		if handler.connCount() >= 2 { // probe + our connection
			break
		}
		time.Sleep(10 * time.Millisecond)
	}

	if handler.connCount() < 2 {
		t.Fatal("OnConnect was not called within timeout")
	}
}

// TestEventLoop_SendsDataToHandler verifies OnData is called when the client
// sends bytes.
func TestEventLoop_SendsDataToHandler(t *testing.T) {
	loop, handler, port := startTestLoop(t)
	defer loop.Stop()

	conn, err := net.Dial("tcp", fmt.Sprintf("127.0.0.1:%d", port))
	if err != nil {
		t.Fatalf("failed to connect: %v", err)
	}
	defer conn.Close()

	want := []byte("NICK alice\r\n")
	conn.Write(want)

	// Wait for the data to arrive at the handler.
	deadline := time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline) {
		events := handler.dataReceived()
		for _, e := range events {
			if string(e.data) == string(want) {
				return // success
			}
		}
		time.Sleep(10 * time.Millisecond)
	}
	t.Error("handler did not receive the sent data within timeout")
}

// TestEventLoop_OnDisconnect verifies OnDisconnect is called when the client
// closes its connection.
func TestEventLoop_OnDisconnect(t *testing.T) {
	loop, handler, port := startTestLoop(t)
	defer loop.Stop()

	conn, err := net.Dial("tcp", fmt.Sprintf("127.0.0.1:%d", port))
	if err != nil {
		t.Fatalf("failed to connect: %v", err)
	}

	// Wait for OnConnect.
	var connID ConnID
	deadline := time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline) {
		handler.mu.Lock()
		for id, _ := range handler.connected {
			if !handler.disconnected[id] {
				connID = id
			}
		}
		handler.mu.Unlock()
		if connID != 0 {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}

	if connID == 0 {
		t.Fatal("no active connection found within timeout")
	}

	// Close the connection and wait for disconnect.
	conn.Close()

	deadline2 := time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline2) {
		if handler.isDisconnected(connID) {
			return // success
		}
		time.Sleep(10 * time.Millisecond)
	}
	t.Error("OnDisconnect was not called within timeout")
}

// TestEventLoop_SendTo verifies that SendTo delivers data to the connected client.
func TestEventLoop_SendTo(t *testing.T) {
	loop, handler, port := startTestLoop(t)
	defer loop.Stop()

	conn, err := net.Dial("tcp", fmt.Sprintf("127.0.0.1:%d", port))
	if err != nil {
		t.Fatalf("failed to connect: %v", err)
	}
	defer conn.Close()

	// Wait for the event loop to register OUR connection (not the probe).
	// We identify it by waiting until there's an active (not disconnected) connection.
	var connID ConnID
	deadline := time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline) {
		handler.mu.Lock()
		for id := range handler.connected {
			if !handler.disconnected[id] {
				connID = id
			}
		}
		handler.mu.Unlock()
		if connID != 0 {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}

	if connID == 0 {
		t.Fatal("connection not registered within timeout")
	}

	// Send data via SendTo.
	want := []byte(":irc.test 001 alice :Welcome\r\n")
	loop.SendTo(connID, want)

	// Read it back from the client side.
	conn.SetReadDeadline(time.Now().Add(3 * time.Second))
	got := make([]byte, len(want))
	n, err := conn.Read(got)
	if err != nil {
		t.Fatalf("failed to read from conn: %v", err)
	}
	got = got[:n]

	if string(got) != string(want) {
		t.Errorf("SendTo delivered %q, expected %q", got, want)
	}
}

// TestEventLoop_SendTo_UnknownConnID verifies that SendTo with an unknown
// ConnID is a safe no-op (does not panic or return an error).
func TestEventLoop_SendTo_UnknownConnID(t *testing.T) {
	loop := NewEventLoop()
	// Should not panic.
	loop.SendTo(9999, []byte("data"))
}

// TestEventLoop_MultipleConnections verifies that multiple simultaneous
// connections are each tracked independently.
func TestEventLoop_MultipleConnections(t *testing.T) {
	loop, handler, port := startTestLoop(t)
	defer loop.Stop()

	const numConns = 5
	conns := make([]net.Conn, numConns)
	for i := 0; i < numConns; i++ {
		c, err := net.Dial("tcp", fmt.Sprintf("127.0.0.1:%d", port))
		if err != nil {
			t.Fatalf("failed to connect #%d: %v", i, err)
		}
		conns[i] = c
	}
	defer func() {
		for _, c := range conns {
			c.Close()
		}
	}()

	// Wait for all connections to be registered (probe + 5 test conns = at least 6).
	deadline := time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline) {
		if handler.connCount() >= numConns+1 {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}

	if handler.connCount() < numConns+1 {
		t.Errorf("expected at least %d connections, got %d", numConns+1, handler.connCount())
	}
}

// TestConnID_Unique verifies that each connection gets a unique ConnID.
func TestConnID_Unique(t *testing.T) {
	loop, handler, port := startTestLoop(t)
	defer loop.Stop()

	const numConns = 3
	conns := make([]net.Conn, numConns)
	for i := 0; i < numConns; i++ {
		c, err := net.Dial("tcp", fmt.Sprintf("127.0.0.1:%d", port))
		if err != nil {
			t.Fatalf("failed to connect #%d: %v", i, err)
		}
		conns[i] = c
	}
	defer func() {
		for _, c := range conns {
			c.Close()
		}
	}()

	// Wait for all connections.
	deadline := time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline) {
		if handler.connCount() >= numConns+1 {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}

	// Verify uniqueness.
	handler.mu.Lock()
	ids := make(map[ConnID]bool)
	for id := range handler.connected {
		if ids[id] {
			t.Errorf("duplicate ConnID: %d", id)
		}
		ids[id] = true
	}
	handler.mu.Unlock()
}

// TestNewEventLoop_EmptyConns verifies the initial state of a new EventLoop.
func TestNewEventLoop_EmptyConns(t *testing.T) {
	loop := NewEventLoop()
	loop.connsMu.Lock()
	n := len(loop.conns)
	loop.connsMu.Unlock()
	if n != 0 {
		t.Errorf("expected empty conns map, got %d entries", n)
	}
}
