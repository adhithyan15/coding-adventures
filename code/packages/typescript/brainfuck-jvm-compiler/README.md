# brainfuck-jvm-compiler

`brainfuck-jvm-compiler` is the end-to-end TypeScript pipeline for compiling
Brainfuck source into JVM `.class` bytes.

Pipeline:

1. Parse Brainfuck source into an AST
2. Lower the AST into generic IR
3. Run the standard IR optimizer
4. Lower IR into a conservative JVM class file
5. Parse the generated class file as a structural self-check
