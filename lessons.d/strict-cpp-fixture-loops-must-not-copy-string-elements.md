---
category: Testing & coverage
---

# Strict C++ fixture loops must not copy string elements

NN50's first actual GCC and Clang harness builds rejected `const std::string`
range-loop variables over string fixture lists under `-Werror=range-loop-construct`.
Use `const std::string&` when the loop only inspects each owned fixture. Keep
the repository's warning-as-error flags unchanged and rerun both compilers.

The first local Windows-front launch then failed before compilation because
`powershell` was not resolvable after process-only MSVC environment bootstrap.
Use the explicit installed Windows PowerShell path for this local invocation;
do not claim a package/compiler failure from a shell-discovery error.
