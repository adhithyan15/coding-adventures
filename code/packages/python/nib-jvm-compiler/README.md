# nib-jvm-compiler

End-to-end Nib to JVM class-file compiler.

Pipeline:

```
Nib source
  -> nib-parser
  -> nib-type-checker
  -> nib-ir-compiler
  -> ir-optimizer
  -> ir-to-jvm-class-file
  -> jvm-class-file parser
  -> .class bytes
```
