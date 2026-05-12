# spice-engine

SPICE-compatible analog circuit simulator. Modified Nodal Analysis (MNA) with
Newton-Raphson DC, trapezoidal transient, AC small-signal sweep, DC transfer
function (`.TF`), DC parameter sweep (`.DC`), sensitivity analysis (`.SENS`),
Monte Carlo (`.MC`), noise analysis (`.NOISE`), and all four SPICE controlled
sources (VCVS / VCCS / CCCS / CCVS).

See [`code/specs/spice-engine.md`](../../../specs/spice-engine.md).

## Quick start

```python
from spice_engine import Circuit, Resistor, VoltageSource, dc_op

# Voltage divider: V1 = 10V, R1 = 1k, R2 = 1k -> V_mid = 5V
circuit = Circuit()
circuit.add(VoltageSource("V1", "vin", "0", voltage=10.0))
circuit.add(Resistor("R1", "vin", "vmid", 1000.0))
circuit.add(Resistor("R2", "vmid", "0", 1000.0))

result = dc_op(circuit)
print(result.node_voltages)        # {"vin": 10.0, "vmid": 5.0}
print(result.branch_currents)      # {"I(V1)": -0.005}
print(result.converged)            # True
```

## Supported elements

| Class | SPICE | Description |
|-------|-------|-------------|
| `Resistor` | R | Ohmic resistor |
| `Capacitor` | C | Linear capacitor (with optional initial voltage) |
| `Inductor` | L | Linear inductor |
| `VoltageSource` | V | Independent voltage source |
| `CurrentSource` | I | Independent current source |
| `Diode` | D | Shockley diode model |
| `Mosfet` | M | MOSFET (uses `mosfet_models.MOSFET`) |
| `BJT` | Q | Bipolar transistor (simplified Ebers-Moll) |
| `VCVS` | E | Voltage-Controlled Voltage Source |
| `VCCS` | G | Voltage-Controlled Current Source |
| `CCCS` | F | Current-Controlled Current Source |
| `CCVS` | H | Current-Controlled Voltage Source |

## Supported analyses

| Function | SPICE | Description |
|----------|-------|-------------|
| `dc_op` | `.OP` | DC operating point (Newton-Raphson) |
| `transient` | `.TRAN` | Time-domain transient (trapezoidal/BE, adaptive timestep) |
| `ac_sweep` | `.AC` | Small-signal AC frequency sweep |
| `tf` | `.TF` | DC transfer function, input/output impedance |
| `dc_sweep` | `.DC` | DC parameter sweep |
| `sens_dc` | `.SENS` | DC sensitivity analysis |
| `mc_dc` | `.MC` | Monte Carlo DC analysis |
| `noise_ac` | `.NOISE` | Small-signal noise PSD (adjoint method) |

## Controlled source examples

### VCVS — unity-gain buffer

```python
from spice_engine import Circuit, VoltageSource, VCVS, Resistor, dc_op

c = Circuit([
    VoltageSource("Vin", "in", "0", 5.0),
    VCVS("E1", "out", "0", ctrl_plus="in", ctrl_minus="0", gain=1.0),
    Resistor("Rload", "out", "0", 1000.0),
])
r = dc_op(c)
print(r.node_voltages["out"])   # 5.0 V — perfect buffer
```

### VCCS — transconductance amplifier

```python
from spice_engine import Circuit, VoltageSource, VCCS, Resistor, dc_op

c = Circuit([
    VoltageSource("Vin", "in", "0", 1.0),
    VCCS("G1", "out", "0", ctrl_plus="in", ctrl_minus="0", gm=0.01),
    Resistor("Rout", "out", "0", 1000.0),
])
r = dc_op(c)
print(r.node_voltages["out"])   # 10.0 V  (gm * Vin * Rout = 0.01 * 1 * 1000)
```

### CCCS — current mirror

```python
from spice_engine import Circuit, VoltageSource, Resistor, CCCS, dc_op

c = Circuit([
    VoltageSource("Vin", "in", "0", 1.0),
    Resistor("Rin", "in", "mid", 1000.0),
    VoltageSource("Vsense", "mid", "0", 0.0),   # 0 V ammeter
    CCCS("F1", "out", "0", ctrl_source="Vsense", beta=2.0),
    Resistor("Rload", "out", "0", 500.0),
])
r = dc_op(c)
# I_ctrl = 1V/1kΩ = 1mA; I_out = 2 * 1mA = 2mA; V_out = 2mA * 500Ω = 1V
print(r.node_voltages["out"])   # 1.0 V
```

### CCVS — transresistance amplifier

```python
from spice_engine import Circuit, VoltageSource, Resistor, CCVS, dc_op

c = Circuit([
    VoltageSource("Vin", "in", "0", 1.0),
    Resistor("Rin", "in", "mid", 1000.0),
    VoltageSource("Vsense", "mid", "0", 0.0),
    CCVS("H1", "out", "0", ctrl_source="Vsense", transresistance=500.0),
    Resistor("Rload", "out", "0", 100.0),
])
r = dc_op(c)
# V_out = rm * I_ctrl = 500 * 1mA = 0.5V
print(r.node_voltages["out"])   # 0.5 V
```

## Node conventions

- Ground is any of `"0"`, `"gnd"`, or `"GND"`.
- CCCS and CCVS use a `VoltageSource` named `ctrl_source` as an ideal ammeter
  (set its voltage to `0.0`).
- CCCS node convention: `F1 n+ n-` → positive current exits `n_plus` into the
  external circuit (same as SPICE F element).

MIT.
