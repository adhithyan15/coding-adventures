# 00070 - LGE-S3b: kotlin lane threat list

* **Added** `lanes/kotlin.json`: 46 threats (the 29 registry classes plus 17 Kotlin lane threats), 46 rows, all `open`
  (no lane evidence is `live` yet, VER15a 5.2), 73 `considered_not_included` entries. Reviewed at origin/main
  `7a793727a16`; the unmerged branch `claude/kotlin-logic-gates-nary` (LGE-KT-NARY) is cited only as `planned` evidence.
* **Findings recorded in the rows** (three were reproduced with local probes that are not repository tools: `javap`, a Java
  caller, `gradle dependencies`):
  * a null `int[]` from Java raises `NullPointerException` from the compiler-generated parameter check, not the package's
    exception family (`kotlin.cwe-476`);
  * `AND_N`/`OR_N` name the failing element `b`, `XOR_N` names `bit[i]`, and arity is a plain `IllegalArgumentException`;
    LG02 section 7 row 9 still says the kotlin identity is `IllegalArgumentException` (`kotlin.error-identity-and-parameter-attribution`);
  * `XOR_N` builds `"bit[$i]"` eagerly for every element and re-reads the array after validating it (`kotlin.cwe-400`, `kotlin.cwe-367`);
  * the library has a runtime dependency on kotlin-stdlib 2.1.20 (the `kotlin("jvm")` plugin adds it), unlike java/logic-gates;
  * `-Xno-param-assertions` on the LGE-KT-NARY branch is module-wide (`kotlin.null-guards-and-param-assertions`).
* **CWE-400** has an exclusion route (loop-bound analyzer rule plus the LGE-KT-NARY N-ary rows); the cap and lazy name that fix it are LGE-KT-NARY, so the row stays open.
* **Unresolved planned ids:** none; every `planned_item` and `becomes_when` backlog id resolves exactly
  (`LGE-A2`, `LGE-KT-NARY`, `LGE-L-kotlin`, `LGE-W1`, `LGE-S3v`, `LGE-S3b`).
