# conduit-hello (F#)

Demonstration of [CodingAdventures.Conduit.FSharp](../../packages/fsharp/conduit/) (WEB16).

## Running

```sh
CONDUIT_CAPI_PATH=<path/to/libconduit_capi.dylib> dotnet run
# or, using run-tests.sh which builds conduit-capi automatically:
sh tools/run-tests.sh   # runs smoke tests
```

Environment variables:
- `HOST` — bind address (default: `127.0.0.1`)
- `PORT` — TCP port (default: `3000`)
- `APP_ENV` — e.g. `development` (default) or `production`
- `CONDUIT_CAPI_PATH` — explicit path to the native cdylib

## Routes

| Method | Path | Description |
|---|---|---|
| GET | `/` | HTML home page |
| GET | `/health` | JSON health check |
| GET | `/api/greet/:name` | Personalised greeting |
| GET | `/api/search?q=…&limit=…` | Search stub |
| POST | `/api/echo` | Echo body with content-type normalisation |
| GET | `/old-home` | 302 redirect to `/` |
| GET | `/tpot` | 418 teapot (HaltException demo) |

## Middleware

- **Before-filter**: requires `x-api-key` header in non-development environments.
- **After-hook**: stamps `x-served-by` and `x-env` on every response.
