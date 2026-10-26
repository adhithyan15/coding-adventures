---
category: Testing & coverage
---

# Strict Cpp fixture loops must not copy string elements

NN50's first actual GCC and Clang harness builds rejected `const std::string`
range-loop variables over string fixture lists under `-Werror=range-loop-construct`.
Use `const std::string&` when the loop only inspects each owned fixture. Keep
the repository's warning-as-error flags unchanged and rerun both compilers.

The first local Windows-front launch then failed before compilation because
`powershell` was not resolvable after process-only MSVC environment bootstrap.
Use the explicit installed Windows PowerShell path for this local invocation;
do not claim a package/compiler failure from a shell-discovery error.

The lesson validator later rejected filenames using `cpp` with titles spelling
`C++`: punctuation is removed by slugging, yielding `c`, not `cpp`. Keep titles
and filenames derivable. The first combined PowerShell validation/commit command
also continued after the nonzero validator exit; explicitly check `$LASTEXITCODE`
before committing, rather than assuming a semicolon sequence fails closed.
