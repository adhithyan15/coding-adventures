# conduit-hello

Rust demo for the Conduit web framework. It mirrors the eight-route demos used by
the Ruby, Python, Lua, TypeScript, and Elixir ports.

## Routes

| Method | Path | Behavior |
|--------|------|----------|
| `GET` | `/` | HTML greeting |
| `GET` | `/hello/:name` | JSON greeting using a route param |
| `POST` | `/echo` | Echoes the request body as JSON |
| `GET` | `/redirect` | `301` redirect to `/` |
| `GET` | `/halt` | `403` response via `halt` |
| `GET` | `/down` | `503` from the before filter |
| `GET` | `/error` | Panic recovered by the custom error handler |
| `GET` | `/missing` | Custom not-found handler |

## Run

```bash
cd code/programs/rust/conduit-hello
cargo run
```

Then try:

```bash
curl http://127.0.0.1:3000/
curl http://127.0.0.1:3000/hello/Adhithya
curl -X POST http://127.0.0.1:3000/echo -H 'Content-Type: application/json' -d '{"ping":"pong"}'
curl -L http://127.0.0.1:3000/redirect
curl http://127.0.0.1:3000/halt
curl http://127.0.0.1:3000/down
curl http://127.0.0.1:3000/error
curl http://127.0.0.1:3000/missing
```

## Development

```bash
bash BUILD
```
