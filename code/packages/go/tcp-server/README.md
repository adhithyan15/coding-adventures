# tcp-server

Protocol-agnostic TCP server for Go.

```go
server := tcpserver.New("127.0.0.1", 6380)
if err := server.ServeForever(); err != nil {
    panic(err)
}
```
