# silicon-rust-java

Java package wrapping the Rust silicon simulation stack via JNI.

## Stack position

```
Java caller
  ↓ import com.codingadventures.silicon.SiliconSim
SiliconSim.java (native method declarations, this package)
  ↓ System.loadLibrary("silicon_rust_jni")
silicon-rust-jni (Rust cdylib, Java_* symbols)
  ↓ Rust
device-physics  mosfet-models  fab-process-simulation
```

## Quick start

```java
import com.codingadventures.silicon.SiliconSim;
import com.codingadventures.silicon.MosResult;

// Physical constants
System.out.println(SiliconSim.kBoltzmann());        // 1.380649e-23 J/K
System.out.println(SiliconSim.thermalVoltage(300)); // 0.025852 V

// PN junction
double vbi = SiliconSim.pnJunctionBuiltInVoltage(1e23, 1e22, 300.0);
double w   = SiliconSim.pnJunctionDepletionWidth(1e23, 1e22, 300.0, 0.0);

// Level-1 MOSFET (default 130 nm NMOS)
MosResult r = SiliconSim.evaluateLevel1Defaults(1.8, 1.8, 0.0, 300.15);
System.out.println(r.region);  // "saturation"
System.out.println(r.id);      // drain current [A]

// Process simulation
String cs = SiliconSim.deposit("", "Si", 500.0);     // Si substrate
cs = SiliconSim.dealGroveOxidation(cs, 5.0);         // grow gate oxide
cs = SiliconSim.deposit(cs, "Poly", 50.0);            // deposit poly gate
// cs == "Poly:50.0|SiO2:<x>|Si:<y>"

double[] rng = SiliconSim.implantRange("B", 30.0);   // {92.0, 38.0} nm
double   d   = SiliconSim.diffusivityCm2PerS("B", 1000.0); // ~1e-14 cm²/s
```

## API

### Physical constants

| Method | Value | Unit |
|---|---|---|
| `kBoltzmann()` | 1.380649×10⁻²³ | J/K |
| `qElectron()` | 1.602176634×10⁻¹⁹ | C |
| `eps0()` | 8.8541878×10⁻¹² | F/m |
| `epsSi()` | 1.0359×10⁻¹⁰ | F/m |
| `epsOx()` | 3.4531×10⁻¹¹ | F/m |
| `niAt300K()` | 1×10¹⁶ | /m³ |
| `egSiAt300K()` | 1.12 | eV |
| `muN300K()` | 0.1350 | m²/V·s |
| `muP300K()` | 0.0480 | m²/V·s |

### device-physics

```java
double thermalVoltage(double tKelvin)
double intrinsicConcentration(double tKelvin) throws SiliconException
double fermiPotential(double nDoping, String kind, double tKelvin) throws SiliconException
    // kind: "p" or "n"
double pnJunctionBuiltInVoltage(double na, double nd, double t) throws SiliconException
double pnJunctionDepletionWidth(double na, double nd, double t, double vApplied) throws SiliconException
double pnJunctionSaturationCurrent(double na, double nd, double a, double t, double tauN, double tauP) throws SiliconException
double pnJunctionCurrent(double na, double nd, double a, double t, double tauN, double tauP, double v) throws SiliconException
double mosfetThresholdVoltage(String deviceType, double l, double w, double tOx, double nBody, double phiMs, double qOx, double t, double vSb) throws SiliconException
    // deviceType: "NMOS" or "PMOS"
```

### mosfet-models

```java
class MosResult {
    double id, gm, gds, gmb, cgs, cgd, cgb, cbs, cbd;  // SI units
    String region;  // "cutoff"|"subthreshold"|"triode"|"saturation"
}

MosResult evaluateLevel1(double vt0, double kp, double lambda, double gamma, double phi,
                          double w, double l, double nSub,
                          double vGs, double vDs, double vBs, double t)
MosResult evaluateLevel1Defaults(double vGs, double vDs, double vBs, double t)
```

### fab-process-simulation

```java
String deposit(String cs, String material, double thicknessNm) throws SiliconException
String etch(String cs, String target, double depthNm) throws SiliconException
String implant(String cs, String species, double energyKev, double doseCm2) throws SiliconException
String diffuse(String cs, double timeMin) throws SiliconException
String diffuseWithTemp(String cs, double timeMin, double temperatureC) throws SiliconException
String dealGroveOxidation(String cs, double timeMin) throws SiliconException
String dealGroveOxidationCustom(String cs, double timeMin, double aUm, double bUm2PerHr) throws SiliconException
double[] implantRange(String species, double energyKev) throws SiliconException  // [Rp, straggle] nm
double diffusivityCm2PerS(String species, double temperatureC)
```

## Cross-section wire format

```
""                               empty cross-section
"Si:500.0"                       bare silicon substrate, 500 nm
"SiO2:4.8|Si:500.0"             gate oxide on silicon
"Poly:50.0|SiO2:4.8|Si:500.0"  poly gate on gate oxide on silicon
```

Material names must not contain `|` or `:`.

## Build

```bash
# Build the Rust cdylib first
cargo build -p silicon-rust-jni --release

# Then run Java tests (library path is configured in build.gradle.kts)
gradle test
```

## Platform notes

| Platform | Library file | Notes |
|---|---|---|
| Linux | `libsilicon_rust_jni.so` | rpath not needed; java.library.path used |
| macOS | `libsilicon_rust_jni.dylib` | same |
| Windows | `silicon_rust_jni.dll` | MinGW/MSVC Rust toolchain required |
