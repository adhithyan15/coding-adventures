# JVM JAR Decoder

Reusable in-memory JAR container decoding for the JVM toolchain.

This package deliberately stops at container decoding so a future class loader,
packager, or real JVM runtime can reuse it without depending on the simulator.
