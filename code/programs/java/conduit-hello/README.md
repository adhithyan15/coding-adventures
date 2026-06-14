# conduit-hello (Java)

Eight-route demo for the Java Conduit framework. Mirrors the demos in the other
language ports.

## Run

```sh
cargo build --manifest-path ../../../packages/rust/Cargo.toml -p conduit-jni --release
gradle run
```

Then:

```sh
curl http://127.0.0.1:3000/
curl http://127.0.0.1:3000/hello/Adhithya
curl -X POST -d 'ping=pong' http://127.0.0.1:3000/echo
curl -i http://127.0.0.1:3000/redirect
curl http://127.0.0.1:3000/halt
curl http://127.0.0.1:3000/down
curl http://127.0.0.1:3000/error
curl http://127.0.0.1:3000/missing
```

## Test

```sh
gradle test
```

The demo depends on the `conduit` package via a Gradle composite build
(`includeBuild` in `settings.gradle.kts`).
