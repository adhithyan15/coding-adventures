# chief-of-staff-agent-stdio-host

Host-side runtime for D18 Level 4 agents written in any language. It launches
an already-authorized executable without a shell, keeps one ordered request in
flight over piped stdin/stdout, and uses the versioned
`chief-agent-stdio-v1` codec.

`LevelFourHost::run_once` deliberately performs:

1. receive one verified channel message;
2. obtain one correlated subprocess response;
3. publish the response to the output channel;
4. acknowledge the input message.

Protocol, process, and publication failures therefore leave the input cursor
unchanged. The concrete session kills and reaps its owned child after an I/O or
protocol failure and when dropped. Package verification, sandbox selection,
restart policy, and timeout supervision remain responsibilities of the caller.

## Validation

```sh
sh chief-of-staff-agent-stdio-host/BUILD
```
