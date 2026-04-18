# nib-jvm-compiler

`nib-jvm-compiler` is the end-to-end TypeScript pipeline for compiling Nib
source into JVM `.class` bytes.

Pipeline:

1. Parse Nib source into an AST
2. Type-check the AST
3. Lower the typed AST into generic IR
4. Run the standard IR optimizer
5. Lower IR into a conservative JVM class file
6. Parse the generated class file as a structural self-check
