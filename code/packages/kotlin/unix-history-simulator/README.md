# Unix history simulator (Kotlin)

Reusable in-memory filesystem, lowest-free descriptor table, semantic kernel,
and command model for the HOS00 First Edition Unix teaching scenario. The
package has no CLI or fixture-file dependency. Components are available
through `FileSystem`, `DescriptorTable`, and `Kernel`.

The package is built with Kotlin Gradle plugin 2.1.20 and targets JDK 21.
`gradle test` runs component checks and compiles the external fixture adapter as
test-only source; that adapter reads the canonical nine cases and exercises the
same package API. The model does not emulate a historical CPU or syscall ABI.
