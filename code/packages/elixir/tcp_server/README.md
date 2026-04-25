# coding_adventures_tcp_server

Protocol-agnostic TCP server for Elixir.

```elixir
server = CodingAdventures.TcpServer.new(port: 6380)
{:ok, server} = CodingAdventures.TcpServer.start(server)
CodingAdventures.TcpServer.serve(server)
```
