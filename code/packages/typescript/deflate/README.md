# @coding-adventures/deflate

**CMP05 — DEFLATE lossless compression (1996)**

## Usage

```typescript
import { compress, decompress } from "@coding-adventures/deflate";

const data = new TextEncoder().encode("hello hello hello world");
const compressed = compress(data);
const original = decompress(compressed);
```

## Wire Format

```
[4B] original_length    big-endian uint32
[2B] ll_entry_count     big-endian uint16
[2B] dist_entry_count   big-endian uint16
[ll_entry_count × 3B]   (symbol uint16 BE, code_length uint8)
[dist_entry_count × 3B] same format
[remaining bytes]       LSB-first packed bit stream
```
