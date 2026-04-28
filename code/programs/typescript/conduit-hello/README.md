# conduit-hello (TypeScript)

Demo program for the [coding-adventures-conduit](../../packages/typescript/conduit)
package — the TypeScript/Node.js port of the Conduit web framework.

## Routes

| Method | Path | Response |
|--------|------|----------|
| GET | `/` | HTML greeting |
| GET | `/hello/:name` | JSON `{ message: "Hello <name>!" }` |
| POST | `/echo` | echoes JSON body |
| GET | `/redirect` | 301 → `/` |
| GET | `/halt` | 403 Forbidden (via `halt()`) |
| GET | `/down` | 503 (intercepted by before filter) |
| GET | `/error` | 500 (triggers custom error handler) |
| GET | `/<anything else>` | 404 (custom not-found handler) |

## Run

```bash
npm ci
npx tsx hello.ts
```

Then:

```bash
curl http://localhost:3000/
curl http://localhost:3000/hello/Adhithya
curl -X POST http://localhost:3000/echo -H 'Content-Type: application/json' -d '{"ping":"pong"}'
curl -iL http://localhost:3000/redirect   # follows the 301
curl http://localhost:3000/halt
curl http://localhost:3000/down
curl http://localhost:3000/error
curl http://localhost:3000/missing
```

## Tests

```bash
npx vitest run
```
