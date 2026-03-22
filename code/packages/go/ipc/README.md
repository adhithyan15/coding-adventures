# IPC (Inter-Process Communication)

Simulates three classic IPC mechanisms found in Unix-like operating systems:

1. **Pipes** -- unidirectional byte streams using a circular buffer
2. **Message Queues** -- FIFO queues of typed messages
3. **Shared Memory** -- named memory regions accessible by multiple processes

## Where It Fits

This package sits in the OS kernel layer between user-space syscalls and the
underlying memory/file systems.

```
User Programs
    |   pipe(), write(), read()
    |   msgget(), msgsnd(), msgrcv()
    |   shmget(), shmat(), shmdt()
    v
OS Kernel -- Syscall Dispatcher
    v
IPC Manager  <-- THIS PACKAGE
    |-- Pipe (circular buffer)
    |-- MessageQueue (FIFO of typed messages)
    +-- SharedMemoryRegion (named memory segments)
```

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/ipc"

// Pipe
p := ipc.NewPipe(4096)
p.Write([]byte("hello"))
buf := p.Read(5) // []byte("hello")

// Message Queue
mq := ipc.NewMessageQueue(256, 4096)
mq.Send(1, []byte("request"))
msgType, data := mq.Receive(0) // any type

// Shared Memory
shm := ipc.NewSharedMemoryRegion("cache", 4096, 1)
shm.Attach(1)
shm.WriteAt(0, []byte("data"))
result := shm.ReadAt(0, 4)

// IPC Manager
mgr := ipc.NewIPCManager()
pipeID, readFD, writeFD := mgr.CreatePipe(4096)
```

## Testing

```bash
go test ./... -v -cover
```
