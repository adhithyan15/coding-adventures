package networkstack

// Socket API — The BSD Socket Interface
//
// The socket API is the bridge between applications and the networking stack.
// Applications don't talk to TCP or UDP directly — they use sockets, which
// are identified by file descriptors (integers).
//
// Think of a socket like a phone:
//  1. Create (socket)  — pick up a new phone
//  2. Bind             — get a phone number
//  3. Listen           — turn on the ringer (server)
//  4. Accept           — pick up when someone calls (server)
//  5. Connect          — dial a number (client)
//  6. Send/Recv        — talk and listen
//  7. Close            — hang up
//
// Two types: STREAM (TCP, like a phone call) and DGRAM (UDP, like a text).

// SocketType identifies the transport protocol for a socket.
type SocketType int

const (
	SocketStream SocketType = 1 // TCP — reliable, ordered, connection-oriented
	SocketDgram  SocketType = 2 // UDP — unreliable, connectionless datagrams
)

// Socket represents one endpoint of network communication. Applications
// interact with sockets through the SocketManager using file descriptors.
type Socket struct {
	Type          SocketType
	FD            int
	BoundIP       uint32
	BoundPort     uint16
	TCPConn       *TCPConnection
	UDPSock       *UDPSocket
	Listening     bool
	AcceptQueue   []*TCPConnection
}

// SocketManager manages all sockets — this is the kernel's socket table.
// It assigns file descriptors and routes operations to protocol handlers.
//
// File descriptors start at 10 to avoid colliding with stdin (0),
// stdout (1), stderr (2).
type SocketManager struct {
	sockets map[int]*Socket
	nextFD  int
}

// NewSocketManager creates a new manager with an empty socket table.
func NewSocketManager() *SocketManager {
	return &SocketManager{
		sockets: make(map[int]*Socket),
		nextFD:  10,
	}
}

// CreateSocket creates a new socket and returns its file descriptor.
func (m *SocketManager) CreateSocket(sockType SocketType) int {
	fd := m.nextFD
	m.nextFD++
	m.sockets[fd] = &Socket{
		Type: sockType,
		FD:   fd,
	}
	return fd
}

// Bind assigns an IP address and port to a socket.
// Returns 0 on success, -1 on error.
func (m *SocketManager) Bind(fd int, ip uint32, port uint16) int {
	sock := m.GetSocket(fd)
	if sock == nil {
		return -1
	}
	sock.BoundIP = ip
	sock.BoundPort = port

	if sock.Type == SocketDgram {
		sock.UDPSock = NewUDPSocket(port)
	} else if sock.Type == SocketStream {
		sock.TCPConn = NewTCPConnection(port, 0, 0)
	}
	return 0
}

// Listen marks a TCP socket as a passive listener.
// Returns 0 on success, -1 on error.
func (m *SocketManager) Listen(fd int, backlog int) int {
	sock := m.GetSocket(fd)
	if sock == nil || sock.Type != SocketStream {
		return -1
	}
	sock.Listening = true
	if sock.TCPConn != nil {
		sock.TCPConn.InitiateListen()
	}
	return 0
}

// Accept dequeues a pending connection. Returns the new fd, or -1 if none.
func (m *SocketManager) Accept(fd int) int {
	sock := m.GetSocket(fd)
	if sock == nil || !sock.Listening || len(sock.AcceptQueue) == 0 {
		return -1
	}
	conn := sock.AcceptQueue[0]
	sock.AcceptQueue = sock.AcceptQueue[1:]

	newFD := m.CreateSocket(SocketStream)
	newSock := m.sockets[newFD]
	newSock.TCPConn = conn
	newSock.BoundIP = sock.BoundIP
	newSock.BoundPort = sock.BoundPort
	return newFD
}

// Connect initiates a TCP connection to a remote host.
// Returns 0 on success, -1 on error.
func (m *SocketManager) Connect(fd int, ip uint32, port uint16) int {
	sock := m.GetSocket(fd)
	if sock == nil || sock.Type != SocketStream {
		return -1
	}
	if sock.TCPConn == nil {
		sock.TCPConn = NewTCPConnection(sock.BoundPort, ip, port)
	} else {
		sock.TCPConn.RemoteIP = ip
		sock.TCPConn.RemotePort = port
	}
	sock.TCPConn.InitiateConnect()
	return 0
}

// Send queues data on a TCP socket. Returns bytes sent or -1 on error.
func (m *SocketManager) Send(fd int, data []byte) int {
	sock := m.GetSocket(fd)
	if sock == nil || sock.TCPConn == nil {
		return -1
	}
	sock.TCPConn.Send(data)
	return len(data)
}

// Recv reads from a TCP socket's receive buffer.
func (m *SocketManager) Recv(fd int, count int) []byte {
	sock := m.GetSocket(fd)
	if sock == nil || sock.TCPConn == nil {
		return nil
	}
	return sock.TCPConn.Receive(count)
}

// SendTo sends a UDP datagram. Returns bytes sent or -1 on error.
func (m *SocketManager) SendTo(fd int, data []byte, ip uint32, port uint16) int {
	sock := m.GetSocket(fd)
	if sock == nil || sock.UDPSock == nil {
		return -1
	}
	sock.UDPSock.SendTo(data, ip, port)
	return len(data)
}

// RecvFrom receives a UDP datagram. Returns nil if none available.
func (m *SocketManager) RecvFrom(fd int) *Datagram {
	sock := m.GetSocket(fd)
	if sock == nil || sock.UDPSock == nil {
		return nil
	}
	return sock.UDPSock.ReceiveFrom()
}

// Close closes a socket and removes it from the table.
// Returns 0 on success, -1 on error.
func (m *SocketManager) Close(fd int) int {
	sock := m.GetSocket(fd)
	if sock == nil {
		return -1
	}
	if sock.TCPConn != nil {
		sock.TCPConn.InitiateClose()
	}
	delete(m.sockets, fd)
	return 0
}

// GetSocket looks up a socket by file descriptor. Returns nil if not found.
func (m *SocketManager) GetSocket(fd int) *Socket {
	return m.sockets[fd]
}
