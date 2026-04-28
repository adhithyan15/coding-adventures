package tcpserver

import (
	"fmt"
	"net"
	"strings"
	"testing"
	"time"
)

func makeConnection() *Connection {
	return &Connection{
		ID:         1,
		PeerAddr:   &net.TCPAddr{IP: net.ParseIP("127.0.0.1"), Port: 45001},
		LocalAddr:  &net.TCPAddr{IP: net.ParseIP("127.0.0.1"), Port: 63079},
		ReadBuffer: nil,
	}
}

func startServer(t *testing.T, server *TcpServer) (uint16, func()) {
	t.Helper()
	done := make(chan error, 1)
	go func() {
		done <- server.Serve()
	}()

	var addr net.Addr
	for i := 0; i < 50; i++ {
		addr = server.Address()
		if addr != nil {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}
	if addr == nil {
		t.Fatal("server did not start")
	}
	port := uint16(addr.(*net.TCPAddr).Port)

	cleanup := func() {
		if err := server.Stop(); err != nil && !strings.Contains(err.Error(), "closed") {
			t.Fatalf("stop failed: %v", err)
		}
		select {
		case err := <-done:
			if err != nil {
				t.Fatalf("serve returned error: %v", err)
			}
		case <-time.After(time.Second):
			t.Fatal("server did not stop")
		}
	}
	return port, cleanup
}

func sendRecv(t *testing.T, port uint16, data []byte) []byte {
	t.Helper()
	conn, err := net.DialTimeout("tcp", fmt.Sprintf("127.0.0.1:%d", port), time.Second)
	if err != nil {
		t.Fatalf("dial failed: %v", err)
	}
	defer conn.Close()

	if _, err := conn.Write(data); err != nil {
		t.Fatalf("write failed: %v", err)
	}
	if err := conn.SetReadDeadline(time.Now().Add(time.Second)); err != nil {
		t.Fatalf("deadline failed: %v", err)
	}
	buf := make([]byte, 4096)
	n, err := conn.Read(buf)
	if err != nil {
		t.Fatalf("read failed: %v", err)
	}
	return buf[:n]
}

func TestDefaultHandlerEchoesWithoutTCP(t *testing.T) {
	server := New("127.0.0.1", 0)
	conn := makeConnection()
	response := server.Handle(conn, []byte("hello"))
	if string(response) != "hello" {
		t.Fatalf("response = %q", response)
	}
}

func TestStatefulHandlerUsesConnection(t *testing.T) {
	server := NewWithHandler("127.0.0.1", 0, func(conn *Connection, data []byte) []byte {
		conn.ReadBuffer = append(conn.ReadBuffer, data...)
		if len(conn.ReadBuffer) < 6 {
			return nil
		}
		conn.SelectedDB = 2
		response := append([]byte(nil), conn.ReadBuffer...)
		conn.ReadBuffer = nil
		return response
	})
	conn := makeConnection()

	if got := server.Handle(conn, []byte("buf")); len(got) != 0 {
		t.Fatalf("expected empty response, got %q", got)
	}
	if got := server.Handle(conn, []byte("fer")); string(got) != "buffer" {
		t.Fatalf("expected buffer response, got %q", got)
	}
	if conn.SelectedDB != 2 || len(conn.ReadBuffer) != 0 {
		t.Fatalf("connection state not updated: %#v", conn)
	}
}

func TestStartAddressAndStop(t *testing.T) {
	server := NewWithOptions("127.0.0.1", 0, 0, 0, nil)
	if server.IsRunning() {
		t.Fatal("new server should not be running")
	}
	if server.Address() != nil {
		t.Fatal("address should be nil before start")
	}
	if _, err := server.TryAddress(); err == nil {
		t.Fatal("expected TryAddress to fail before start")
	}

	if err := server.Start(); err != nil {
		t.Fatalf("start failed: %v", err)
	}
	defer server.Stop()
	if err := server.Start(); err != nil {
		t.Fatalf("second start failed: %v", err)
	}
	if !server.IsRunning() || server.Address() == nil {
		t.Fatal("server should report running address")
	}
	if _, err := server.TryAddress(); err != nil {
		t.Fatalf("TryAddress failed: %v", err)
	}
	if !strings.Contains(server.String(), "running") {
		t.Fatalf("string should include running status: %s", server.String())
	}
}

func TestLoopbackEcho(t *testing.T) {
	server := New("127.0.0.1", 0)
	port, cleanup := startServer(t, server)
	defer cleanup()

	if got := sendRecv(t, port, []byte("hello world")); string(got) != "hello world" {
		t.Fatalf("got %q", got)
	}
}

func TestMultipleSequentialClients(t *testing.T) {
	server := NewWithHandler("127.0.0.1", 0, func(_ *Connection, data []byte) []byte {
		return []byte(strings.ToUpper(string(data)))
	})
	port, cleanup := startServer(t, server)
	defer cleanup()

	if got := sendRecv(t, port, []byte("one")); string(got) != "ONE" {
		t.Fatalf("first response = %q", got)
	}
	if got := sendRecv(t, port, []byte("two")); string(got) != "TWO" {
		t.Fatalf("second response = %q", got)
	}
}
