# http-core

`http-core` defines the version-neutral HTTP message model shared by higher-level HTTP implementations.

## What It Provides

- `Header` values that preserve arrival order and duplicate header lines
- `HttpVersion` semantic version parsing and formatting
- `BodyKind` framing hints for `none`, `content-length`, `until-eof`, and `chunked`
- `RequestHead` and `ResponseHead` semantic message heads
- Helpers for case-insensitive header lookup plus `Content-Length` and `Content-Type`

## How It Fits The Stack

HTTP/1, HTTP/2, and HTTP/3 use different wire formats, but callers still want the same semantic objects once a head has been parsed. This package is that shared layer.

## Development

```bash
bash BUILD
```
