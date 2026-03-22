package networkstack

import (
	"testing"
)

func TestCreateStreamSocket(t *testing.T) {
	mgr := NewSocketManager()
	fd := mgr.CreateSocket(SocketStream)
	if fd < 10 {
		t.Errorf("fd should be >= 10")
	}
	sock := mgr.GetSocket(fd)
	if sock == nil {
		t.Fatal("socket not found")
	}
	if sock.Type != SocketStream {
		t.Errorf("wrong type")
	}
}

func TestCreateDgramSocket(t *testing.T) {
	mgr := NewSocketManager()
	fd := mgr.CreateSocket(SocketDgram)
	sock := mgr.GetSocket(fd)
	if sock == nil || sock.Type != SocketDgram {
		t.Errorf("expected DGRAM socket")
	}
}

func TestUniqueFDs(t *testing.T) {
	mgr := NewSocketManager()
	fd1 := mgr.CreateSocket(SocketStream)
	fd2 := mgr.CreateSocket(SocketDgram)
	fd3 := mgr.CreateSocket(SocketStream)
	if fd1 == fd2 || fd2 == fd3 || fd1 == fd3 {
		t.Errorf("fds should be unique")
	}
}

func TestGetNonexistentSocket(t *testing.T) {
	mgr := NewSocketManager()
	if mgr.GetSocket(999) != nil {
		t.Error("expected nil")
	}
}

func TestBindStream(t *testing.T) {
	mgr := NewSocketManager()
	fd := mgr.CreateSocket(SocketStream)
	result := mgr.Bind(fd, 0x0A000001, 80)
	if result != 0 {
		t.Errorf("bind failed")
	}
	sock := mgr.GetSocket(fd)
	if sock.BoundIP != 0x0A000001 || sock.BoundPort != 80 {
		t.Errorf("bind values wrong")
	}
	if sock.TCPConn == nil {
		t.Error("TCP connection not created")
	}
}

func TestBindDgram(t *testing.T) {
	mgr := NewSocketManager()
	fd := mgr.CreateSocket(SocketDgram)
	mgr.Bind(fd, 0, 53)
	sock := mgr.GetSocket(fd)
	if sock.UDPSock == nil {
		t.Error("UDP socket not created")
	}
}

func TestBindInvalidFD(t *testing.T) {
	mgr := NewSocketManager()
	if mgr.Bind(999, 0, 80) != -1 {
		t.Error("expected -1")
	}
}

func TestListen(t *testing.T) {
	mgr := NewSocketManager()
	fd := mgr.CreateSocket(SocketStream)
	mgr.Bind(fd, 0, 80)
	result := mgr.Listen(fd, 5)
	if result != 0 {
		t.Errorf("listen failed")
	}
	sock := mgr.GetSocket(fd)
	if !sock.Listening {
		t.Error("should be listening")
	}
}

func TestListenDgramFails(t *testing.T) {
	mgr := NewSocketManager()
	fd := mgr.CreateSocket(SocketDgram)
	if mgr.Listen(fd, 5) != -1 {
		t.Error("listen on DGRAM should fail")
	}
}

func TestConnect(t *testing.T) {
	mgr := NewSocketManager()
	fd := mgr.CreateSocket(SocketStream)
	mgr.Bind(fd, 0x0A000001, 49152)
	result := mgr.Connect(fd, 0x0A000002, 80)
	if result != 0 {
		t.Errorf("connect failed")
	}
	sock := mgr.GetSocket(fd)
	if sock.TCPConn.State != StateSynSent {
		t.Errorf("expected SYN_SENT")
	}
}

func TestSendAndRecv(t *testing.T) {
	mgr := NewSocketManager()
	fd := mgr.CreateSocket(SocketStream)
	mgr.Bind(fd, 0, 80)
	sock := mgr.GetSocket(fd)
	sock.TCPConn.State = StateEstablished

	sent := mgr.Send(fd, []byte("Hello"))
	if sent != 5 {
		t.Errorf("sent: got %d, want 5", sent)
	}
	// Data is in send buffer, recv buffer is empty
	data := mgr.Recv(fd, 100)
	if len(data) != 0 {
		t.Errorf("expected empty recv")
	}
}

func TestAcceptWithQueue(t *testing.T) {
	mgr := NewSocketManager()
	fd := mgr.CreateSocket(SocketStream)
	mgr.Bind(fd, 0, 80)
	mgr.Listen(fd, 5)

	sock := mgr.GetSocket(fd)
	incoming := NewTCPConnection(80, 0, 0)
	incoming.State = StateEstablished
	sock.AcceptQueue = append(sock.AcceptQueue, incoming)

	newFD := mgr.Accept(fd)
	if newFD == -1 {
		t.Fatal("accept failed")
	}
	newSock := mgr.GetSocket(newFD)
	if newSock.TCPConn != incoming {
		t.Error("wrong connection")
	}
}

func TestAcceptEmptyQueue(t *testing.T) {
	mgr := NewSocketManager()
	fd := mgr.CreateSocket(SocketStream)
	mgr.Bind(fd, 0, 80)
	mgr.Listen(fd, 5)
	if mgr.Accept(fd) != -1 {
		t.Error("expected -1 for empty queue")
	}
}

func TestSendToRecvFrom(t *testing.T) {
	mgr := NewSocketManager()
	fd := mgr.CreateSocket(SocketDgram)
	mgr.Bind(fd, 0, 12345)

	sent := mgr.SendTo(fd, []byte("hello"), 0x08080808, 53)
	if sent != 5 {
		t.Errorf("sent: got %d, want 5", sent)
	}

	sock := mgr.GetSocket(fd)
	sock.UDPSock.Deliver([]byte("response"), 0x08080808, 53)

	d := mgr.RecvFrom(fd)
	if d == nil {
		t.Fatal("expected datagram")
	}
	if string(d.Data) != "response" {
		t.Errorf("data mismatch")
	}
}

func TestRecvFromEmpty(t *testing.T) {
	mgr := NewSocketManager()
	fd := mgr.CreateSocket(SocketDgram)
	mgr.Bind(fd, 0, 12345)
	if mgr.RecvFrom(fd) != nil {
		t.Error("expected nil")
	}
}

func TestSendToInvalidFD(t *testing.T) {
	mgr := NewSocketManager()
	if mgr.SendTo(999, []byte("data"), 0, 0) != -1 {
		t.Error("expected -1")
	}
}

func TestCloseRemovesSocket(t *testing.T) {
	mgr := NewSocketManager()
	fd := mgr.CreateSocket(SocketStream)
	if mgr.Close(fd) != 0 {
		t.Error("close failed")
	}
	if mgr.GetSocket(fd) != nil {
		t.Error("socket should be removed")
	}
}

func TestCloseInvalidFD(t *testing.T) {
	mgr := NewSocketManager()
	if mgr.Close(999) != -1 {
		t.Error("expected -1")
	}
}
