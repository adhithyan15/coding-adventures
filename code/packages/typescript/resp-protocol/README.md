# @coding-adventures/resp-protocol

RESP2 encoder/decoder used by the datastore stack.

This package is intentionally small, dependency-free, and byte-oriented. It
handles:

- simple strings
- errors
- integers
- bulk strings
- arrays
- inline commands

## Example

```ts
import { decode, encode, array, bulkString } from "@coding-adventures/resp-protocol";

const frame = array([
  bulkString("SET"),
  bulkString("counter"),
  bulkString("1"),
]);

const bytes = encode(frame);
const decoded = decode(bytes);
```
