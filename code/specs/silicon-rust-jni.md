# Spec: silicon-rust-jni — JNI Java Bindings for the Silicon Simulation Stack

## Overview

`silicon-rust-jni` exposes `device-physics`, `mosfet-models`, and
`fab-process-simulation` to Java (and any JVM language) through the Java
Native Interface (JNI).  It is a Rust `cdylib` that exports functions with the
`Java_*` naming convention.  The companion Java package `silicon-rust-java`
(at `code/packages/java/silicon-rust-java/`) declares the corresponding
`native` methods in `com.codingadventures.silicon.SiliconSim` and loads the
library with `System.loadLibrary`.

## Stack Position

```
Java caller
  ↓ import com.codingadventures.silicon.SiliconSim
SiliconSim.java (native method declarations)
  ↓ JNI (System.loadLibrary)
silicon-rust-jni (Rust cdylib, Java_* functions)
  ↓ Rust function calls
device-physics   mosfet-models   fab-process-simulation
     ↑                ↑                   ↑
     └──────── jni-bridge (zero-dep JNI helpers) ─────────┘
```

## Rust Crates

### `jni-bridge`

Zero-dependency Rust crate that provides:

- JNI primitive types: `jboolean`, `jbyte`, `jchar`, `jshort`, `jint`,
  `jlong`, `jfloat`, `jdouble`, `jsize`
- JNI reference types (all `*mut c_void`): `jobject`, `jclass`, `jstring`,
  `jarray`, `jthrowable`, `jmethodID`, `jfieldID`
- `JNIEnv` — pointer to the JVM's function dispatch table
  (`*mut *const *const c_void`)
- `jvalue` union — used with `NewObjectA` for variadic-free constructor calls
- Helper functions that access the function table at fixed JNI-spec offsets:
  - `jni_find_class(env, name) -> jclass`
  - `jni_throw_new(env, class_name, msg)` — throws a pending exception
  - `jni_get_string_utf(env, s) -> Option<String>`
  - `jni_new_string_utf(env, s) -> jstring`
  - `jni_get_method_id(env, cls, name, sig) -> jmethodID`
  - `jni_new_object_a(env, cls, ctor, args) -> jobject`
  - `jni_new_double_array(env, len) -> jarray`
  - `jni_set_double_array_region(env, arr, start, len, buf)`

Implementation note: JNI's `JNINativeInterface_` is a struct of 232
function pointers.  Rather than declaring the entire struct, `jni-bridge`
reads function pointers by offset using raw pointer arithmetic:

```
let fn_ptr = *(*env).add(OFFSET);
```

Offsets follow the JNI 21 specification exactly; they are constants defined
in `jni-bridge/src/lib.rs`.

### `silicon-rust-jni`

Rust `cdylib` that depends on `jni-bridge`, `device-physics`,
`mosfet-models`, and `fab-process-simulation`.  Exports 29 JNI functions.

#### Function Naming

Java class `com.codingadventures.silicon.SiliconSim`, method `deposit`:
→ Rust symbol: `Java_com_codingadventures_silicon_SiliconSim_deposit`

Dots in the Java package path become underscores.  The class name and
method name are separated by one more underscore.

#### Wire Format for CrossSection

A `CrossSection` is serialised as a pipe-separated list of
`material:thickness_nm` pairs, ordered top-to-bottom (same format as all
other bindings):

```
""                               empty cross-section
"Si:500.0"                       bare silicon substrate, 500 nm
"SiO2:4.8|Si:500.0"             gate oxide on silicon
"Poly:50.0|SiO2:4.8|Si:500.0"  poly gate on gate oxide on silicon
```

`{:?}` formatting (Rust Debug) is used for `f64` to preserve the decimal
point on whole-number thicknesses (500.0 → "500.0", not "500").

`cs_from_wire` validates every material name via `validate_name`.  Any
entry containing `|` or `:` causes the entire wire string to be rejected
with an error.

#### Exported Functions

| Java signature | Rust symbol suffix | Notes |
|---|---|---|
| `double kBoltzmann()` | `kBoltzmann` | constant |
| `double qElectron()` | `qElectron` | constant |
| `double eps0()` | `eps0` | constant |
| `double epsSi()` | `epsSi` | constant |
| `double epsOx()` | `epsOx` | constant |
| `double niAt300K()` | `niAt300K` | constant |
| `double egSiAt300K()` | `egSiAt300K` | constant |
| `double muN300K()` | `muN300K` | constant |
| `double muP300K()` | `muP300K` | constant |
| `double thermalVoltage(double t)` | `thermalVoltage` | infallible |
| `double intrinsicConcentration(double t) throws SiliconException` | `intrinsicConcentration` | |
| `double fermiPotential(double n, String kind, double t) throws SiliconException` | `fermiPotential` | |
| `double pnJunctionBuiltInVoltage(double na, double nd, double t) throws SiliconException` | `pnJunctionBuiltInVoltage` | |
| `double pnJunctionDepletionWidth(double na, double nd, double t, double v) throws SiliconException` | `pnJunctionDepletionWidth` | |
| `double pnJunctionSaturationCurrent(double na, double nd, double a, double t, double tauN, double tauP) throws SiliconException` | `pnJunctionSaturationCurrent` | |
| `double pnJunctionCurrent(double na, double nd, double a, double t, double tauN, double tauP, double v) throws SiliconException` | `pnJunctionCurrent` | |
| `double mosfetThresholdVoltage(String dtype, double l, double w, double tOx, double nBody, double phiMs, double qOx, double t, double vSb) throws SiliconException` | `mosfetThresholdVoltage` | |
| `MosResult evaluateLevel1(double vt0, double kp, double lambda, double gamma, double phi, double w, double l, double nSub, double vGs, double vDs, double vBs, double t) throws SiliconException` | `evaluateLevel1` | returns Java object |
| `MosResult evaluateLevel1Defaults(double vGs, double vDs, double vBs, double t) throws SiliconException` | `evaluateLevel1Defaults` | returns Java object |
| `String deposit(String cs, String material, double thicknessNm) throws SiliconException` | `deposit` | |
| `String etch(String cs, String target, double depthNm) throws SiliconException` | `etch` | |
| `String implant(String cs, String species, double energyKev, double doseCm2) throws SiliconException` | `implant` | |
| `String diffuse(String cs, double timeMin) throws SiliconException` | `diffuse` | |
| `String diffuseWithTemp(String cs, double timeMin, double tempC) throws SiliconException` | `diffuseWithTemp` | |
| `String dealGroveOxidation(String cs, double timeMin) throws SiliconException` | `dealGroveOxidation` | |
| `String dealGroveOxidationCustom(String cs, double timeMin, double aUm, double bUm2PerHr) throws SiliconException` | `dealGroveOxidationCustom` | |
| `double[] implantRange(String species, double energyKev) throws SiliconException` | `implantRange` | returns `double[2]`: {rp, straggle} |
| `double diffusivityCm2PerS(String species, double temperatureC)` | `diffusivityCm2PerS` | infallible |

#### MosResult Object Creation

`evaluateLevel1` and `evaluateLevel1Defaults` create a
`com.codingadventures.silicon.MosResult` instance via JNI:

1. `FindClass(env, "com/codingadventures/silicon/MosResult")` → `cls`
2. `GetMethodID(env, cls, "<init>", "(DDDDDDDDDLjava/lang/String;)V")` → `ctor`
3. `NewStringUTF(env, region_str)` → `region_jstr`
4. Build `jvalue[10]` array: 9 doubles + region string as `jvalue { l }`
5. `NewObjectA(env, cls, ctor, args.as_ptr())` → return

#### Exception Handling

For functions that declare `throws SiliconException`:
- On error: `jni_throw_new(env, "com/codingadventures/silicon/SiliconException", &msg)`
  then return null / 0.0
- The pending exception is propagated to Java by the JVM after the native
  function returns.

#### Null Safety

- Null `jstring` arguments are treated as `""` (empty cross-section wire)
  for `cs` parameters, and rejected with `SiliconException` for required
  material/species/kind strings.
- A null `jclass` from `FindClass` (class not found) causes the function to
  return null without a pending exception; the JVM will have raised its own
  `ClassNotFoundException`.

## Java Package: `silicon-rust-java`

### Location

`code/packages/java/silicon-rust-java/`

### Classes

#### `com.codingadventures.silicon.SiliconSim`

Public class with 29 `public static native` methods corresponding to the
29 JNI functions above.  Static initializer loads the library:

```java
static {
    System.loadLibrary("silicon_rust_jni");
}
```

#### `com.codingadventures.silicon.MosResult`

Plain data class:

```java
public final class MosResult {
    public final double id, gm, gds, gmb, cgs, cgd, cgb, cbs, cbd;
    public final String region;

    public MosResult(double id, double gm, double gds, double gmb,
                     double cgs, double cgd, double cgb, double cbs, double cbd,
                     String region) { ... }
}
```

JNI constructor signature: `"(DDDDDDDDDLjava/lang/String;)V"`

#### `com.codingadventures.silicon.SiliconException`

```java
public class SiliconException extends RuntimeException {
    public SiliconException(String message) { super(message); }
}
```

Note: `RuntimeException` (unchecked) rather than `Exception` (checked) so
callers are not forced to declare `throws`.  Declared as `throws
SiliconException` in method signatures anyway to document the error
contract.

### Build

`build.gradle.kts` configures:

- Java 21 source and target compatibility
- JUnit Jupiter 5 for tests
- `jvmArgs("-Djava.library.path=...")` pointing at
  `code/packages/rust/target/release`

BUILD script (executed from the package directory):

```
cargo build --manifest-path ../../rust/Cargo.toml -p silicon-rust-jni --release && gradle test
```

### Tests

`SiliconSimTest.java` (JUnit Jupiter) covers:

- 9 constant accessors (bounds checks)
- `thermalVoltage` at 300 K
- `intrinsicConcentration` valid + invalid temperature
- `fermiPotential` for both "p" and "n"
- `pnJunctionBuiltInVoltage`, `pnJunctionDepletionWidth`
- `pnJunctionSaturationCurrent`, `pnJunctionCurrent`
- `mosfetThresholdVoltage` for NMOS
- `evaluateLevel1Defaults` — checks region, Id > 0
- `evaluateLevel1` — checks all 9 numeric fields non-NaN
- `deposit`, `etch`, `implant`, `diffuse`, `diffuseWithTemp`
- `dealGroveOxidation`, `dealGroveOxidationCustom`
- `implantRange` — checks double[2] length and positive values
- `diffusivityCm2PerS` — checks positive result
- Injection guard: `deposit` with "|" in material name throws `SiliconException`
- Null cs: `deposit(null, "Si", 500.0)` treated as empty cross-section

## Library Loading and Library Path

At runtime, `System.loadLibrary("silicon_rust_jni")` searches
`java.library.path` for:
- Linux: `libsilicon_rust_jni.so`
- macOS: `libsilicon_rust_jni.dylib`
- Windows: `silicon_rust_jni.dll`

Tests configure `java.library.path` via `jvmArgs` in `build.gradle.kts`
to point at the Rust release build directory relative to the project.

## Differences from Other Bindings

| Binding | Host runtime | FFI mechanism |
|---|---|---|
| Python (`silicon-rust-python`) | CPython | Python C API, function table in `libpython` |
| Node.js (`silicon-rust-napi`) | Node.js | N-API stable ABI, offset-based dispatch |
| Ruby (`silicon-rust-ruby-native`) | MRI Ruby | `libruby` C API, direct extern "C" |
| Go (`silicon-rust-go`) | Go runtime | CGo, plain C ABI via `silicon-rust-cgo` cdylib |
| Java (`silicon-rust-jni`) | JVM | JNI, `Java_*` naming, offset-based env dispatch |

All bindings share the same wire format for `CrossSection`, the same
`validate_name` injection guard, and `{:?}` formatting for `f64` in
wire serialization.
