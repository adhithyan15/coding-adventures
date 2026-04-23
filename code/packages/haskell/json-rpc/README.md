# json-rpc

JSON-RPC 2.0 transport and dispatch for Haskell.

## What it does

- parses and serialises JSON-RPC requests, responses, and notifications
- frames messages with `Content-Length` headers
- dispatches request and notification handlers through a small server abstraction
- stays self-contained with an internal JSON value/parser layer so it builds in this repo environment

## Status

This Haskell package now has end-to-end framing and dispatch tests, including parse-error responses.
