# silicon-rust-jni

JNI native library (`cdylib`) that exposes the Rust silicon simulation stack
to Java (and any JVM language) via the Java Native Interface.  The companion
Java package is `silicon-rust-java`.

## Stack position

```
Java caller
  ↓ import com.codingadventures.silicon.SiliconSim
SiliconSim.java (native method declarations)
  ↓ System.loadLibrary("silicon_rust_jni")
silicon-rust-jni (this crate, Java_* symbols)
  ↓ Rust calls
device-physics  mosfet-models  fab-process-simulation
                       ↑
               jni-bridge (zero-dep JNI helpers)
```

## What this crate does

- Exports 29 `Java_com_codingadventures_silicon_SiliconSim_*` symbols
- Converts between `jstring` / `jdouble` / `jarray` / `jobject` and Rust
  types using `jni-bridge`
- Serialises `CrossSection` as a pipe-separated wire string (same format
  as all other silicon bindings)
- Throws `com.codingadventures.silicon.SiliconException` for errors
- Creates `com.codingadventures.silicon.MosResult` Java objects via JNI
  `NewObjectA`

## Cross-section wire format

```
""                               empty cross-section
"Si:500.0"                       bare silicon substrate, 500 nm
"SiO2:4.8|Si:500.0"             gate oxide on silicon
"Poly:50.0|SiO2:4.8|Si:500.0"  poly gate on gate oxide on silicon
```

`{:?}` formatting preserves the decimal point on whole-number f64 values.

## Build

The Rust tests cover pure-Rust helpers (no JVM needed):

```bash
cargo test --lib -p silicon-rust-jni
```

To build the actual cdylib (loaded by the JVM):

```bash
cargo build -p silicon-rust-jni --release
```

End-to-end testing is done by the `silicon-rust-java` Java package.

## Platform output

| Platform | File |
|---|---|
| Linux | `libsilicon_rust_jni.so` |
| macOS | `libsilicon_rust_jni.dylib` |
| Windows | `silicon_rust_jni.dll` |
