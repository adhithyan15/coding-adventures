# @coding-adventures/in-memory-data-store-protocol

Command translation layer for the in-memory data store stack.

This package converts RESP arrays into normalized command objects that the
engine can execute without knowing anything about the wire format.

## Example

```ts
import { array, bulkString } from "@coding-adventures/resp-protocol";
import { commandFromResp } from "@coding-adventures/in-memory-data-store-protocol";

const command = commandFromResp(
  array([bulkString("SET"), bulkString("counter"), bulkString("1")]),
);
```
