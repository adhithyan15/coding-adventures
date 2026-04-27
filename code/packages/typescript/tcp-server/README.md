# @coding-adventures/tcp-server

Protocol-agnostic TCP server for Node.js.

```ts
import { TcpServer } from "@coding-adventures/tcp-server";

const server = new TcpServer({
  host: "127.0.0.1",
  port: 6380,
  handler: (_connection, data) => data,
});

await server.start();
```
