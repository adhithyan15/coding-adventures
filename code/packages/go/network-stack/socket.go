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
	result, _ := StartNew[*SocketManager]("network-stack.NewSocketManager", nil,
		func(op *Operation[*SocketManager], rf *ResultFactory[*SocketManager]) *OperationResult[*SocketManager] {
			return rf.Generate(true, false, &SocketManager{
				sockets: make(map[int]*Socket),
				nextFD:  10,
			})
		}).GetResult()
	return result
}

// CreateSocket creates a new socket and returns its file descriptor.
func (m *SocketManager) CreateSocket(sockType SocketType) int {
	result, _ := StartNew[int]("network-stack.SocketManager.CreateSocket", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			fd := m.nextFD
			m.nextFD++
			m.sockets[fd] = &Socket{
				Type: sockType,
				FD:   fd,
			}
			return rf.Generate(true, false, fd)
		}).GetResult()
	return result
}

// Bind assigns an IP address and port to a socket.
// Returns 0 on success, -1 on error.
func (m *SocketManager) Bind(fd int, ip uint32, port uint16) int {
	result, _ := StartNew[int]("network-stack.SocketManager.Bind", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("fd", fd)
			sock := m.GetSocket(fd)
			if sock == nil {
				return rf.Generate(true, false, -1)
			}
			sock.BoundIP = ip
			sock.BoundPort = port
			if sock.Type == SocketDgram {
				sock.UDPSock = NewUDPSocket(port)
			} else if sock.Type == SocketStream {
				sock.TCPConn = NewTCPConnection(port, 0, 0)
			}
			return rf.Generate(true, false, 0)
		}).GetResult()
	return result
}

// Listen marks a TCP socket as a passive listener.
// Returns 0 on success, -1 on error.
func (m *SocketManager) Listen(fd int, backlog int) int {
	result, _ := StartNew[int]("network-stack.SocketManager.Listen", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("fd", fd)
			op.AddProperty("backlog", backlog)
			sock := m.GetSocket(fd)
			if sock == nil || sock.Type != SocketStream {
				return rf.Generate(true, false, -1)
			}
			sock.Listening = true
			if sock.TCPConn != nil {
				sock.TCPConn.InitiateListen()
			}
			return rf.Generate(true, false, 0)
		}).GetResult()
	return result
}

// Accept dequeues a pending connection. Returns the new fd, or -1 if none.
func (m *SocketManager) Accept(fd int) int {
	result, _ := StartNew[int]("network-stack.SocketManager.Accept", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("fd", fd)
			sock := m.GetSocket(fd)
			if sock == nil || !sock.Listening || len(sock.AcceptQueue) == 0 {
				return rf.Generate(true, false, -1)
			}
			conn := sock.AcceptQueue[0]
			sock.AcceptQueue = sock.AcceptQueue[1:]

			newFD := m.CreateSocket(SocketStream)
			newSock := m.sockets[newFD]
			newSock.TCPConn = conn
			newSock.BoundIP = sock.BoundIP
			newSock.BoundPort = sock.BoundPort
			return rf.Generate(true, false, newFD)
		}).GetResult()
	return result
}

// Connect initiates a TCP connection to a remote host.
// Returns 0 on success, -1 on error.
func (m *SocketManager) Connect(fd int, ip uint32, port uint16) int {
	result, _ := StartNew[int]("network-stack.SocketManager.Connect", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("fd", fd)
			sock := m.GetSocket(fd)
			if sock == nil || sock.Type != SocketStream {
				return rf.Generate(true, false, -1)
			}
			if sock.TCPConn == nil {
				sock.TCPConn = NewTCPConnection(sock.BoundPort, ip, port)
			} else {
				sock.TCPConn.RemoteIP = ip
				sock.TCPConn.RemotePort = port
			}
			sock.TCPConn.InitiateConnect()
			return rf.Generate(true, false, 0)
		}).GetResult()
	return result
}

// Send queues data on a TCP socket. Returns bytes sent or -1 on error.
func (m *SocketManager) Send(fd int, data []byte) int {
	result, _ := StartNew[int]("network-stack.SocketManager.Send", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("fd", fd)
			sock := m.GetSocket(fd)
			if sock == nil || sock.TCPConn == nil {
				return rf.Generate(true, false, -1)
			}
			sock.TCPConn.Send(data)
			return rf.Generate(true, false, len(data))
		}).GetResult()
	return result
}

// Recv reads from a TCP socket's receive buffer.
func (m *SocketManager) Recv(fd int, count int) []byte {
	result, _ := StartNew[[]byte]("network-stack.SocketManager.Recv", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("fd", fd)
			op.AddProperty("count", count)
			sock := m.GetSocket(fd)
			if sock == nil || sock.TCPConn == nil {
				return rf.Generate(true, false, nil)
			}
			return rf.Generate(true, false, sock.TCPConn.Receive(count))
		}).GetResult()
	return result
}

// SendTo sends a UDP datagram. Returns bytes sent or -1 on error.
func (m *SocketManager) SendTo(fd int, data []byte, ip uint32, port uint16) int {
	result, _ := StartNew[int]("network-stack.SocketManager.SendTo", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("fd", fd)
			sock := m.GetSocket(fd)
			if sock == nil || sock.UDPSock == nil {
				return rf.Generate(true, false, -1)
			}
			sock.UDPSock.SendTo(data, ip, port)
			return rf.Generate(true, false, len(data))
		}).GetResult()
	return result
}

// RecvFrom receives a UDP datagram. Returns nil if none available.
func (m *SocketManager) RecvFrom(fd int) *Datagram {
	result, _ := StartNew[*Datagram]("network-stack.SocketManager.RecvFrom", nil,
		func(op *Operation[*Datagram], rf *ResultFactory[*Datagram]) *OperationResult[*Datagram] {
			op.AddProperty("fd", fd)
			sock := m.GetSocket(fd)
			if sock == nil || sock.UDPSock == nil {
				return rf.Generate(true, false, nil)
			}
			return rf.Generate(true, false, sock.UDPSock.ReceiveFrom())
		}).GetResult()
	return result
}

// Close closes a socket and removes it from the table.
// Returns 0 on success, -1 on error.
func (m *SocketManager) Close(fd int) int {
	result, _ := StartNew[int]("network-stack.SocketManager.Close", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("fd", fd)
			sock := m.GetSocket(fd)
			if sock == nil {
				return rf.Generate(true, false, -1)
			}
			if sock.TCPConn != nil {
				sock.TCPConn.InitiateClose()
			}
			delete(m.sockets, fd)
			return rf.Generate(true, false, 0)
		}).GetResult()
	return result
}

// GetSocket looks up a socket by file descriptor. Returns nil if not found.
func (m *SocketManager) GetSocket(fd int) *Socket {
	result, _ := StartNew[*Socket]("network-stack.SocketManager.GetSocket", nil,
		func(op *Operation[*Socket], rf *ResultFactory[*Socket]) *OperationResult[*Socket] {
			op.AddProperty("fd", fd)
			return rf.Generate(true, false, m.sockets[fd])
		}).GetResult()
	return result
}
