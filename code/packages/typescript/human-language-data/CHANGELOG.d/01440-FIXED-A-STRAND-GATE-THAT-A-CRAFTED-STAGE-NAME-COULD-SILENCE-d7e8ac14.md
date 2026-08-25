### Fixed - a strand gate that a crafted stage name could silence

Security review of the above, verified against the built module rather than
reasoned from the diff. `byStage` was built with `Object.fromEntries`, so it
inherited from `Object.prototype`, and membership was tested with `in`, which
walks the prototype chain. A node declaring `stage: "toString"` therefore passed
the check, read the inherited **function**, and `+= 1` wrote the string
`"function toString() { [native code] }1"` into the counts. That string is not
`=== 0`, so `missingStages` reported the stage as **covered**.

A gate whose whole job is making curriculum defects visible, reporting clean
*because of* a crafted stage name, is worse than no gate. Buckets are now
`Object.create(null)` with an own-property check.

Five malformed-JSON shapes also threw uncaught `TypeError`s out of the CLI --
`strands` as an object or string, `stages` as a string, `nodes` absent, `nodes`
holding `null` -- surfacing as Node stack traces with absolute filesystem paths
where `report-cli` otherwise catches and returns exit 2. `Array.isArray` guards
now match the shape validation `loadChapterPolicy` already performs.

Confirmed not exploitable and deliberately unchanged: `Object.fromEntries` uses
`CreateDataPropertyOrThrow`, so a `__proto__` key becomes an ordinary own
property and never a prototype write.

Specified in `code/specs/HL10-spanish-pre-a1-to-c2-course-architecture.md`.


