// Package transistors implements transistor-level circuit simulation —
// the layer between raw physics and digital logic gates.
//
// # Why transistors matter
//
// Logic gates (AND, OR, NOT) are abstractions. In real hardware, each gate
// is built from transistors — tiny electrically-controlled switches. This
// package simulates those transistors and shows how gates emerge from them.
//
// There are two main transistor families:
//
//   - MOSFET (Metal-Oxide-Semiconductor Field-Effect Transistor):
//     Voltage-controlled. Used in all modern chips (CMOS technology).
//     Near-zero static power consumption.
//
//   - BJT (Bipolar Junction Transistor):
//     Current-controlled. Used in historical TTL logic (7400 series).
//     Higher static power, but historically faster.
//
// # Package organization
//
//   - types.go:       Constants, parameter structs, result types
//   - mosfet.go:      NMOS and PMOS transistor simulation
//   - bjt.go:         NPN and PNP transistor simulation
//   - cmos_gates.go:  Logic gates built from MOSFET pairs (CMOS)
//   - ttl_gates.go:   Logic gates built from BJT transistors (TTL/RTL)
//   - amplifier.go:   Transistors as analog signal amplifiers
//   - analysis.go:    Noise margins, power, timing, technology comparison
package transistors

// ===========================================================================
// OPERATING REGION CONSTANTS
// ===========================================================================
// A transistor is an analog device that operates differently depending on
// the voltages applied to its terminals. The three "regions" describe these
// different operating modes.

// MOSFET operating regions.
//
// Think of a MOSFET like a water faucet with three positions:
//
//	CUTOFF:     Faucet is fully closed. No water flows.
//	            (Vgs < Vth — gate voltage too low to turn on)
//
//	LINEAR:     Faucet is open, and water flow increases as you
//	            turn the handle more. Flow is proportional to
//	            both handle position AND water pressure.
//	            (Vgs > Vth, Vds < Vgs - Vth — acts like a resistor)
//
//	SATURATION: Faucet is wide open, but the pipe is the bottleneck.
//	            Adding more pressure doesn't increase flow much.
//	            (Vgs > Vth, Vds >= Vgs - Vth — current is roughly constant)
//
// For digital circuits, we only use CUTOFF (OFF) and deep LINEAR (ON).
// For analog amplifiers, we operate in SATURATION.
const (
	MOSFETCutoff     = "cutoff"
	MOSFETLinear     = "linear"
	MOSFETSaturation = "saturation"
)

// BJT operating regions.
//
// Similar to MOSFET regions but with different names and physics:
//
//	CUTOFF:      No base current -> no collector current. Switch OFF.
//	             (Vbe < ~0.7V)
//
//	ACTIVE:      Small base current, large collector current.
//	             Ic = beta * Ib. This is the AMPLIFIER region.
//	             (Vbe >= ~0.7V, Vce > ~0.2V)
//
//	SATURATION:  Both junctions forward-biased. Collector current
//	             is maximum — transistor is fully ON as a switch.
//	             (Vbe >= ~0.7V, Vce <= ~0.2V)
//
// Confusing naming alert: MOSFET "saturation" = constant current (amplifier).
// BJT "saturation" = fully ON (switch). These are DIFFERENT behaviors despite
// sharing a name. Hardware engineers have been confusing students with this
// for decades.
const (
	BJTCutoff     = "cutoff"
	BJTActive     = "active"
	BJTSaturation = "saturation"
)

// ===========================================================================
// ELECTRICAL PARAMETERS
// ===========================================================================
// These structs hold the physical characteristics of transistors.
// Default values represent common, well-documented transistor types
// so that users can start experimenting immediately without needing
// to look up datasheets.

// MOSFETParams holds electrical parameters for a MOSFET transistor.
//
// Default values (from DefaultMOSFETParams) represent a typical 180nm CMOS
// process — the last "large" process node that is still widely used in
// education and analog/mixed-signal chips.
//
// Key parameters:
//
//	Vth:    Threshold voltage — the minimum Vgs to turn the transistor ON.
//	        Lower Vth = faster switching but more leakage current.
//	        Modern CPUs use Vth around 0.2-0.4V.
//
//	K:      Transconductance parameter — controls how much current flows
//	        for a given Vgs. Higher K = more current = faster but more power.
//	        K = mu * Cox * (W/L) where mu is carrier mobility and Cox is
//	        oxide capacitance per unit area.
//
//	W, L:   Channel width and length. The W/L ratio is the main knob
//	        chip designers use to tune transistor strength.
//
//	CGate:  Gate capacitance — determines switching speed.
//	CDrain: Drain junction capacitance — contributes to output load.
type MOSFETParams struct {
	Vth    float64
	K      float64
	W      float64
	L      float64
	CGate  float64
	CDrain float64
}

// DefaultMOSFETParams returns parameters for a typical 180nm CMOS process.
func DefaultMOSFETParams() MOSFETParams {
	result, _ := StartNew[MOSFETParams]("transistors.DefaultMOSFETParams", MOSFETParams{},
		func(op *Operation[MOSFETParams], rf *ResultFactory[MOSFETParams]) *OperationResult[MOSFETParams] {
			return rf.Generate(true, false, MOSFETParams{
				Vth:    0.4,
				K:      0.001,
				W:      1e-6,
				L:      180e-9,
				CGate:  1e-15,
				CDrain: 0.5e-15,
			})
		}).GetResult()
	return result
}

// BJTParams holds electrical parameters for a BJT transistor.
//
// Default values (from DefaultBJTParams) represent a typical small-signal
// NPN transistor like the 2N2222 — one of the most common transistors ever
// made, used in everything from hobby projects to early spacecraft.
//
// Key parameters:
//
//	Beta:   Current gain (hfe) — the ratio Ic/Ib. A beta of 100 means
//	        1mA of base current controls 100mA of collector current.
//
//	VbeOn:  Base-emitter voltage when conducting. For silicon BJTs,
//	        this is always around 0.6-0.7V.
//
//	VceSat: Collector-emitter voltage when fully saturated (switch ON).
//	        Ideally 0V, practically about 0.1-0.3V.
//
//	Is:     Reverse saturation current — the tiny leakage current
//	        that flows even when the transistor is OFF.
//
//	CBase:  Base capacitance — limits switching speed.
type BJTParams struct {
	Beta   float64
	VbeOn  float64
	VceSat float64
	Is     float64
	CBase  float64
}

// DefaultBJTParams returns parameters for a typical 2N2222-style NPN transistor.
func DefaultBJTParams() BJTParams {
	result, _ := StartNew[BJTParams]("transistors.DefaultBJTParams", BJTParams{},
		func(op *Operation[BJTParams], rf *ResultFactory[BJTParams]) *OperationResult[BJTParams] {
			return rf.Generate(true, false, BJTParams{
				Beta:   100.0,
				VbeOn:  0.7,
				VceSat: 0.2,
				Is:     1e-14,
				CBase:  5e-12,
			})
		}).GetResult()
	return result
}

// CircuitParams holds parameters for a complete logic gate circuit.
//
//	Vdd:         Supply voltage. Modern CMOS uses 0.7-1.2V, older CMOS
//	             used 3.3V or 5V, TTL always uses 5V.
//
//	Temperature: Junction temperature in Kelvin. Room temperature is
//	             ~300K (27C).
type CircuitParams struct {
	Vdd         float64
	Temperature float64
}

// DefaultCircuitParams returns typical circuit parameters (3.3V, 300K).
func DefaultCircuitParams() CircuitParams {
	result, _ := StartNew[CircuitParams]("transistors.DefaultCircuitParams", CircuitParams{},
		func(op *Operation[CircuitParams], rf *ResultFactory[CircuitParams]) *OperationResult[CircuitParams] {
			return rf.Generate(true, false, CircuitParams{
				Vdd:         3.3,
				Temperature: 300.0,
			})
		}).GetResult()
	return result
}

// ===========================================================================
// RESULT TYPES
// ===========================================================================
// These structs hold the results of transistor and circuit analysis.

// GateOutput is the result of evaluating a logic gate with voltage-level detail.
//
// Unlike the logic_gates package which only returns 0 or 1, this gives
// you the full electrical picture: what voltage does the output actually
// sit at? How much power is being consumed? How long did the signal
// take to propagate?
type GateOutput struct {
	LogicValue        int
	Voltage           float64
	CurrentDraw       float64
	PowerDissipation  float64
	PropagationDelay  float64
	TransistorCount   int
}

// AmplifierAnalysis holds results of analyzing a transistor as an amplifier.
//
// When a transistor operates in its linear/active region (not as a
// digital switch), it can amplify signals.
//
//	VoltageGain:     How much the output voltage changes per unit input change.
//	Transconductance: gm — ratio of output current change to input voltage change.
//	InputImpedance:  How much the amplifier "loads" the signal source.
//	OutputImpedance: How "stiff" the output is.
//	Bandwidth:       Frequency at which gain drops to 70.7% (-3dB).
//	OperatingPoint:  DC bias conditions (map of parameter name to value).
type AmplifierAnalysis struct {
	VoltageGain      float64
	Transconductance float64
	InputImpedance   float64
	OutputImpedance  float64
	Bandwidth        float64
	OperatingPoint   map[string]float64
}

// NoiseMargins tells you how much electrical noise a digital signal
// can tolerate before being misinterpreted.
//
//	VOL: Output LOW voltage
//	VOH: Output HIGH voltage
//	VIL: Input LOW threshold — max voltage accepted as 0
//	VIH: Input HIGH threshold — min voltage accepted as 1
//	NML: Noise Margin LOW  = VIL - VOL
//	NMH: Noise Margin HIGH = VOH - VIH
type NoiseMargins struct {
	VOL float64
	VOH float64
	VIL float64
	VIH float64
	NML float64
	NMH float64
}

// PowerAnalysis holds power consumption breakdown for a gate.
//
//	StaticPower:     Power consumed when not switching (leakage).
//	DynamicPower:    Power consumed during switching (C * Vdd^2 * f * alpha).
//	TotalPower:      Static + Dynamic.
//	EnergyPerSwitch: Energy for one 0->1->0 transition (C * Vdd^2).
type PowerAnalysis struct {
	StaticPower     float64
	DynamicPower    float64
	TotalPower      float64
	EnergyPerSwitch float64
}

// TimingAnalysis holds timing characteristics for a gate.
//
//	Tphl:         Propagation delay HIGH to LOW output.
//	Tplh:         Propagation delay LOW to HIGH output.
//	Tpd:          Average propagation delay = (Tphl + Tplh) / 2.
//	RiseTime:     Time for output to go from 10% to 90% of Vdd.
//	FallTime:     Time for output to go from 90% to 10% of Vdd.
//	MaxFrequency: Maximum clock frequency = 1 / (2 * Tpd).
type TimingAnalysis struct {
	Tphl         float64
	Tplh         float64
	Tpd          float64
	RiseTime     float64
	FallTime     float64
	MaxFrequency float64
}
