## 0.122.0 — 2026-06-27 — BASIC string inequality drives control flow (LANG-FULL BA4/E4)

The BASIC matrix now proves the other standard equality-family string branch:

```basic
10 LET A$ = "N"
20 IF A$ <> "Y" THEN 50
30 PRINT "BAD"
40 END
50 PRINT "OK"
60 END
```

The frontend lowers `<>` by reusing E4 `str_eq` and branching with
`jmp_if_false`, so stdout `OK` is produced on native-AOT + LLVM + WASM + JVM +
CLR + VM + JIT without adding a bespoke `str_ne` opcode.

