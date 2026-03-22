package ipc

import (
	"bytes"
	"errors"
	"testing"
)

// ========================================================================
// Pipe management
// ========================================================================

func TestIPCManagerCreatePipe(t *testing.T) {
	mgr := NewIPCManager()
	pipeID, readFD, writeFD := mgr.CreatePipe(4096)
	if pipeID < 0 {
		t.Error("pipe ID should be non-negative")
	}
	if readFD < 3 || writeFD < 3 {
		t.Error("FDs should be >= 3")
	}
	if readFD == writeFD {
		t.Error("read and write FDs should differ")
	}
}

func TestIPCManagerGetPipe(t *testing.T) {
	mgr := NewIPCManager()
	id, _, _ := mgr.CreatePipe(4096)
	p := mgr.GetPipe(id)
	if p == nil {
		t.Fatal("expected pipe, got nil")
	}
}

func TestIPCManagerGetPipeNotFound(t *testing.T) {
	mgr := NewIPCManager()
	if mgr.GetPipe(999) != nil {
		t.Error("should be nil for non-existent pipe")
	}
}

func TestIPCManagerPipeWriteRead(t *testing.T) {
	mgr := NewIPCManager()
	id, _, _ := mgr.CreatePipe(4096)
	p := mgr.GetPipe(id)
	p.Write([]byte("through manager"))
	data := p.Read(15)
	if !bytes.Equal(data, []byte("through manager")) {
		t.Errorf("expected 'through manager', got %q", data)
	}
}

func TestIPCManagerCreatePipeCustomCapacity(t *testing.T) {
	mgr := NewIPCManager()
	id, _, _ := mgr.CreatePipe(128)
	if mgr.GetPipe(id).Capacity() != 128 {
		t.Error("expected capacity 128")
	}
}

func TestIPCManagerClosePipeRead(t *testing.T) {
	mgr := NewIPCManager()
	id, _, _ := mgr.CreatePipe(4096)
	mgr.ClosePipeRead(id)
	_, err := mgr.GetPipe(id).Write([]byte("broken"))
	if !errors.Is(err, ErrBrokenPipe) {
		t.Errorf("expected ErrBrokenPipe, got %v", err)
	}
}

func TestIPCManagerClosePipeWrite(t *testing.T) {
	mgr := NewIPCManager()
	id, _, _ := mgr.CreatePipe(4096)
	mgr.ClosePipeWrite(id)
	if !mgr.GetPipe(id).IsEOF() {
		t.Error("expected EOF")
	}
}

func TestIPCManagerClosePipeNonexistent(t *testing.T) {
	mgr := NewIPCManager()
	mgr.ClosePipeRead(999)  // should not panic
	mgr.ClosePipeWrite(999) // should not panic
}

func TestIPCManagerDestroyPipe(t *testing.T) {
	mgr := NewIPCManager()
	id, _, _ := mgr.CreatePipe(4096)
	if !mgr.DestroyPipe(id) {
		t.Error("destroy should return true")
	}
	if mgr.GetPipe(id) != nil {
		t.Error("destroyed pipe should be nil")
	}
}

func TestIPCManagerDestroyPipeNonexistent(t *testing.T) {
	mgr := NewIPCManager()
	if mgr.DestroyPipe(999) {
		t.Error("should return false for non-existent")
	}
}

func TestIPCManagerMultiplePipes(t *testing.T) {
	mgr := NewIPCManager()
	id1, _, _ := mgr.CreatePipe(4096)
	id2, _, _ := mgr.CreatePipe(4096)
	id3, _, _ := mgr.CreatePipe(4096)
	if id1 == id2 || id2 == id3 {
		t.Error("pipe IDs should be unique")
	}
	if len(mgr.ListPipes()) != 3 {
		t.Errorf("expected 3 pipes, got %d", len(mgr.ListPipes()))
	}
}

func TestIPCManagerUniqueFDs(t *testing.T) {
	mgr := NewIPCManager()
	_, r1, w1 := mgr.CreatePipe(4096)
	_, r2, w2 := mgr.CreatePipe(4096)
	fds := map[int]bool{r1: true, w1: true, r2: true, w2: true}
	if len(fds) != 4 {
		t.Error("all FDs should be unique")
	}
}

// ========================================================================
// Message queue management
// ========================================================================

func TestIPCManagerCreateMessageQueue(t *testing.T) {
	mgr := NewIPCManager()
	mq := mgr.CreateMessageQueue("work", 256, 4096)
	if mq == nil {
		t.Fatal("expected message queue")
	}
}

func TestIPCManagerGetMessageQueue(t *testing.T) {
	mgr := NewIPCManager()
	mgr.CreateMessageQueue("work", 256, 4096)
	mq := mgr.GetMessageQueue("work")
	if mq == nil {
		t.Fatal("expected message queue")
	}
}

func TestIPCManagerGetMessageQueueNotFound(t *testing.T) {
	mgr := NewIPCManager()
	if mgr.GetMessageQueue("nope") != nil {
		t.Error("should be nil for non-existent")
	}
}

func TestIPCManagerIdempotentCreateQueue(t *testing.T) {
	mgr := NewIPCManager()
	mq1 := mgr.CreateMessageQueue("work", 256, 4096)
	mq2 := mgr.CreateMessageQueue("work", 128, 2048)
	if mq1 != mq2 {
		t.Error("idempotent create should return same object")
	}
}

func TestIPCManagerSendReceiveThroughManager(t *testing.T) {
	mgr := NewIPCManager()
	mq := mgr.CreateMessageQueue("tasks", 256, 4096)
	mq.Send(1, []byte("do this"))
	mt, data, ok := mq.Receive(0)
	if !ok || mt != 1 || !bytes.Equal(data, []byte("do this")) {
		t.Error("send/receive through manager failed")
	}
}

func TestIPCManagerDeleteMessageQueue(t *testing.T) {
	mgr := NewIPCManager()
	mgr.CreateMessageQueue("work", 256, 4096)
	if !mgr.DeleteMessageQueue("work") {
		t.Error("delete should return true")
	}
	if mgr.GetMessageQueue("work") != nil {
		t.Error("deleted queue should be nil")
	}
}

func TestIPCManagerDeleteQueueNonexistent(t *testing.T) {
	mgr := NewIPCManager()
	if mgr.DeleteMessageQueue("nope") {
		t.Error("should return false")
	}
}

// ========================================================================
// Shared memory management
// ========================================================================

func TestIPCManagerCreateSharedMemory(t *testing.T) {
	mgr := NewIPCManager()
	r := mgr.CreateSharedMemory("cache", 4096, 1)
	if r == nil || r.Name() != "cache" || r.Size() != 4096 {
		t.Error("create shared memory failed")
	}
}

func TestIPCManagerGetSharedMemory(t *testing.T) {
	mgr := NewIPCManager()
	mgr.CreateSharedMemory("cache", 4096, 1)
	if mgr.GetSharedMemory("cache") == nil {
		t.Error("expected shared memory region")
	}
}

func TestIPCManagerGetSharedMemoryNotFound(t *testing.T) {
	mgr := NewIPCManager()
	if mgr.GetSharedMemory("nope") != nil {
		t.Error("should be nil")
	}
}

func TestIPCManagerIdempotentCreateSharedMemory(t *testing.T) {
	mgr := NewIPCManager()
	r1 := mgr.CreateSharedMemory("buf", 1024, 1)
	r2 := mgr.CreateSharedMemory("buf", 2048, 2)
	if r1 != r2 {
		t.Error("should return same object")
	}
	if r1.Size() != 1024 {
		t.Error("original size should be preserved")
	}
}

func TestIPCManagerWriteReadThroughManager(t *testing.T) {
	mgr := NewIPCManager()
	r := mgr.CreateSharedMemory("data", 256, 1)
	r.WriteAt(0, []byte("managed"))
	data, _ := r.ReadAt(0, 7)
	if !bytes.Equal(data, []byte("managed")) {
		t.Error("write/read through manager failed")
	}
}

func TestIPCManagerDeleteSharedMemory(t *testing.T) {
	mgr := NewIPCManager()
	mgr.CreateSharedMemory("cache", 4096, 1)
	if !mgr.DeleteSharedMemory("cache") {
		t.Error("delete should return true")
	}
	if mgr.GetSharedMemory("cache") != nil {
		t.Error("should be nil after delete")
	}
}

func TestIPCManagerDeleteSharedMemoryNonexistent(t *testing.T) {
	mgr := NewIPCManager()
	if mgr.DeleteSharedMemory("nope") {
		t.Error("should return false")
	}
}

// ========================================================================
// List operations
// ========================================================================

func TestIPCManagerListPipesEmpty(t *testing.T) {
	mgr := NewIPCManager()
	if len(mgr.ListPipes()) != 0 {
		t.Error("should be empty")
	}
}

func TestIPCManagerListPipes(t *testing.T) {
	mgr := NewIPCManager()
	id1, _, _ := mgr.CreatePipe(4096)
	id2, _, _ := mgr.CreatePipe(4096)
	pipes := mgr.ListPipes()
	found := map[int]bool{}
	for _, id := range pipes {
		found[id] = true
	}
	if !found[id1] || !found[id2] {
		t.Error("list should contain both pipe IDs")
	}
}

func TestIPCManagerListMessageQueuesEmpty(t *testing.T) {
	mgr := NewIPCManager()
	if len(mgr.ListMessageQueues()) != 0 {
		t.Error("should be empty")
	}
}

func TestIPCManagerListMessageQueues(t *testing.T) {
	mgr := NewIPCManager()
	mgr.CreateMessageQueue("a", 256, 4096)
	mgr.CreateMessageQueue("b", 256, 4096)
	names := mgr.ListMessageQueues()
	found := map[string]bool{}
	for _, n := range names {
		found[n] = true
	}
	if !found["a"] || !found["b"] {
		t.Error("list should contain both queue names")
	}
}

func TestIPCManagerListSharedRegionsEmpty(t *testing.T) {
	mgr := NewIPCManager()
	if len(mgr.ListSharedRegions()) != 0 {
		t.Error("should be empty")
	}
}

func TestIPCManagerListSharedRegions(t *testing.T) {
	mgr := NewIPCManager()
	mgr.CreateSharedMemory("x", 128, 1)
	mgr.CreateSharedMemory("y", 256, 1)
	names := mgr.ListSharedRegions()
	found := map[string]bool{}
	for _, n := range names {
		found[n] = true
	}
	if !found["x"] || !found["y"] {
		t.Error("list should contain both region names")
	}
}

func TestIPCManagerListAfterDeletion(t *testing.T) {
	mgr := NewIPCManager()
	mgr.CreateMessageQueue("keep", 256, 4096)
	mgr.CreateMessageQueue("remove", 256, 4096)
	mgr.DeleteMessageQueue("remove")
	names := mgr.ListMessageQueues()
	if len(names) != 1 || names[0] != "keep" {
		t.Errorf("expected [keep], got %v", names)
	}
}
