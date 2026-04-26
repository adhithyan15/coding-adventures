// Package tcpserver provides a protocol-agnostic TCP server.
package tcpserver

import (
	"errors"
	"fmt"
	"io"
	"net"
	"strconv"
	"sync"
	"sync/atomic"
)

type Connection struct {
	ID         int
	PeerAddr   net.Addr
	LocalAddr  net.Addr
	ReadBuffer []byte
	SelectedDB int
}

type Handler func(*Connection, []byte) []byte

type TcpServer struct {
	host       string
	port       uint16
	backlog    int
	bufferSize int
	handler    Handler

	mu          sync.Mutex
	listener    net.Listener
	connections map[int]net.Conn
	nextID      atomic.Int64
	running     atomic.Bool
}

func New(host string, port uint16) *TcpServer {
	return NewWithHandler(host, port, func(_ *Connection, data []byte) []byte {
		return append([]byte(nil), data...)
	})
}

func NewWithHandler(host string, port uint16, handler Handler) *TcpServer {
	return NewWithOptions(host, port, 128, 4096, handler)
}

func NewWithOptions(host string, port uint16, backlog int, bufferSize int, handler Handler) *TcpServer {
	if backlog < 1 {
		backlog = 1
	}
	if bufferSize < 1 {
		bufferSize = 1
	}
	if handler == nil {
		handler = func(_ *Connection, data []byte) []byte { return append([]byte(nil), data...) }
	}
	server := &TcpServer{
		host:        host,
		port:        port,
		backlog:     backlog,
		bufferSize:  bufferSize,
		handler:     handler,
		connections: make(map[int]net.Conn),
	}
	server.nextID.Store(1)
	return server
}

func (s *TcpServer) Start() error {
	s.mu.Lock()
	defer s.mu.Unlock()

	if s.listener != nil {
		s.running.Store(true)
		return nil
	}

	listener, err := net.Listen("tcp", net.JoinHostPort(s.host, strconv.Itoa(int(s.port))))
	if err != nil {
		return err
	}
	s.listener = listener
	s.running.Store(true)
	return nil
}

func (s *TcpServer) Serve() error {
	if err := s.Start(); err != nil {
		return err
	}

	for {
		listener := s.currentListener()
		if listener == nil {
			return nil
		}

		conn, err := listener.Accept()
		if err != nil {
			if !s.IsRunning() || errors.Is(err, net.ErrClosed) {
				s.cleanup()
				return nil
			}
			return err
		}
		go s.serveConn(conn)
	}
}

func (s *TcpServer) ServeForever() error {
	return s.Serve()
}

func (s *TcpServer) Handle(connection *Connection, data []byte) []byte {
	return s.handler(connection, data)
}

func (s *TcpServer) Stop() error {
	s.mu.Lock()
	listener := s.listener
	s.listener = nil
	for id, conn := range s.connections {
		_ = conn.Close()
		delete(s.connections, id)
	}
	s.running.Store(false)
	s.mu.Unlock()

	if listener != nil {
		return listener.Close()
	}
	return nil
}

func (s *TcpServer) IsRunning() bool {
	return s.running.Load()
}

func (s *TcpServer) Address() net.Addr {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.listener == nil {
		return nil
	}
	return s.listener.Addr()
}

func (s *TcpServer) TryAddress() (net.Addr, error) {
	addr := s.Address()
	if addr == nil {
		return nil, errors.New("server has not been started")
	}
	return addr, nil
}

func (s *TcpServer) String() string {
	status := "stopped"
	if s.IsRunning() {
		status = "running"
	}
	return fmt.Sprintf("TcpServer{host:%q, port:%d, status:%s}", s.host, s.port, status)
}

func (s *TcpServer) currentListener() net.Listener {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.listener
}

func (s *TcpServer) serveConn(conn net.Conn) {
	id := int(s.nextID.Add(1) - 1)
	connection := &Connection{
		ID:         id,
		PeerAddr:   conn.RemoteAddr(),
		LocalAddr:  conn.LocalAddr(),
		ReadBuffer: nil,
		SelectedDB: 0,
	}

	s.mu.Lock()
	s.connections[id] = conn
	s.mu.Unlock()
	defer func() {
		_ = conn.Close()
		s.mu.Lock()
		delete(s.connections, id)
		s.mu.Unlock()
	}()

	buffer := make([]byte, s.bufferSize)
	for {
		n, err := conn.Read(buffer)
		if n > 0 {
			response := s.Handle(connection, buffer[:n])
			if len(response) > 0 {
				if _, writeErr := conn.Write(response); writeErr != nil {
					return
				}
			}
		}
		if err != nil {
			if errors.Is(err, io.EOF) {
				return
			}
			return
		}
	}
}

func (s *TcpServer) cleanup() {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.listener = nil
	for id, conn := range s.connections {
		_ = conn.Close()
		delete(s.connections, id)
	}
	s.running.Store(false)
}
