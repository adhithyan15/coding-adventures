# http1

`http1` parses HTTP/1 request and response heads and tells callers how to consume the body bytes that follow.

## What It Provides

- Request-line and status-line parsing
- Ordered header parsing that preserves duplicates
- Head/body boundary detection with `body_offset`
- Body framing detection for `none`, `content-length`, `until-eof`, and `chunked`
- Validation errors for malformed start lines, headers, status codes, and `Content-Length`

## How It Fits The Stack

This package sits on top of `http-core`. A TCP layer hands it bytes, it returns semantic request/response heads plus body framing instructions, and a higher-level client can then pass the extracted body to HTML, image, or other content decoders.

## Development

```bash
bash BUILD
```
