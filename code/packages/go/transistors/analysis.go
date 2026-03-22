package transistors

// Electrical Analysis — noise margins, power, timing, and technology comparison.
//
// Digital logic designers don't just care about truth tables — they care about:
//
// 1. NOISE MARGINS: Can the circuit tolerate voltage fluctuations?
//    A chip has billions of wires running millimeters apart, each creating
//    electromagnetic interference on its neighbors.
//
// 2. POWER: How much energy does the chip consume? Power = the #1 constraint
//    in modern chip design.
//
// 3. TIMING: How fast can the circuit switch? The propagation delay through
//    a gate determines the maximum clock frequency.
//
// 4. SCALING: How do these properties change as we shrink transistors?
//    Moore's Law predicts transistor count doubles every ~2 years.

import (
	"fmt"
	"math"
)

// gateInfo is a helper interface for extracting circuit parameters from
// different gate types. We use a type switch rather than a common interface
// because the gate types have different internal structures.
type gateInfo struct {
	isCMOS     bool
	vdd        float64
	nmos       *NMOS
	pmos       *PMOS
	staticPow  float64
}

// extractGateInfo pulls circuit parameters from a gate using type assertion.
func extractGateInfo(gate interface{}) (gateInfo, error) {
	switch g := gate.(type) {
	case *CMOSInverter:
		return gateInfo{
			isCMOS: true, vdd: g.Circuit.Vdd,
			nmos: g.Nmos, pmos: g.Pmos, staticPow: 0.0,
		}, nil
	case *CMOSNand:
		return gateInfo{
			isCMOS: true, vdd: g.Circuit.Vdd,
			nmos: g.Nmos1, pmos: g.Pmos1, staticPow: 0.0,
		}, nil
	case *CMOSNor:
		return gateInfo{
			isCMOS: true, vdd: g.Circuit.Vdd,
			nmos: g.Nmos1, pmos: g.Pmos1, staticPow: 0.0,
		}, nil
	case *TTLNand:
		return gateInfo{
			isCMOS: false, vdd: g.Vcc,
			staticPow: g.StaticPower(),
		}, nil
	default:
		return gateInfo{}, fmt.Errorf("unsupported gate type: %T", gate)
	}
}

// ComputeNoiseMargins analyzes noise margins for a gate.
//
// Noise margins tell you how much electrical noise a digital signal
// can tolerate before being misinterpreted by the next gate.
//
// For CMOS:
//
//	VOL ~ 0V, VOH ~ Vdd -> large noise margins
//	NML ~ NMH ~ 0.4 * Vdd (symmetric)
//
// For TTL:
//
//	VOL ~ 0.2V, VOH ~ 3.5V -> smaller margins
//	VIL = 0.8V, VIH = 2.0V (defined by spec)
//
// Supported gate types: *CMOSInverter, *TTLNand.
func ComputeNoiseMargins(gate interface{}) (NoiseMargins, error) {
	var vol, voh, vil, vih float64

	switch g := gate.(type) {
	case *CMOSInverter:
		vdd := g.Circuit.Vdd
		// CMOS has nearly ideal rail-to-rail output
		vol = 0.0
		voh = vdd
		// Input thresholds at ~40% and ~60% of Vdd (symmetric CMOS)
		vil = 0.4 * vdd
		vih = 0.6 * vdd
	case *TTLNand:
		// TTL specifications (standard 74xx series)
		vol = 0.2  // Vce_sat of output transistor
		voh = g.Vcc - 0.7 // Vcc minus one diode drop
		vil = 0.8  // Standard TTL input LOW threshold
		vih = 2.0  // Standard TTL input HIGH threshold
	default:
		return NoiseMargins{}, fmt.Errorf("unsupported gate type: %T", gate)
	}

	nml := vil - vol
	nmh := voh - vih

	return NoiseMargins{
		VOL: vol, VOH: voh,
		VIL: vil, VIH: vih,
		NML: nml, NMH: nmh,
	}, nil
}

// AnalyzePower computes power consumption for a gate at a given frequency.
//
// === Power in CMOS ===
//
//	P_total = P_static + P_dynamic
//	P_static ~ negligible (nanowatts)
//	P_dynamic = C_load * Vdd^2 * f * activityFactor
//
// === Power in TTL ===
//
//	P_static ~ milliwatts (DOMINATES!)
//	P_dynamic = similar formula but static power is so large it barely matters.
//
// Supported gate types: *CMOSInverter, *CMOSNand, *CMOSNor, *TTLNand.
func AnalyzePower(gate interface{}, frequency, cLoad, activityFactor float64) (PowerAnalysis, error) {
	info, err := extractGateInfo(gate)
	if err != nil {
		return PowerAnalysis{}, err
	}

	staticPower := info.staticPow
	vdd := info.vdd

	// Dynamic power: P = C * V^2 * f * alpha
	dynamic := cLoad * vdd * vdd * frequency * activityFactor
	total := staticPower + dynamic

	// Energy per switching event: E = C * V^2
	energyPerSwitch := cLoad * vdd * vdd

	return PowerAnalysis{
		StaticPower:     staticPower,
		DynamicPower:    dynamic,
		TotalPower:      total,
		EnergyPerSwitch: energyPerSwitch,
	}, nil
}

// AnalyzeTiming computes timing characteristics for a gate.
//
// For CMOS:
//
//	t_pd ~ (C_load * Vdd) / (2 * I_sat)
//	I_sat = 0.5 * k * (Vdd - Vth)^2
//
// For TTL:
//
//	t_pd ~ 5-15 ns (fixed by transistor switching speed)
//
// Supported gate types: *CMOSInverter, *CMOSNand, *CMOSNor, *TTLNand.
func AnalyzeTiming(gate interface{}, cLoad float64) (TimingAnalysis, error) {
	var tphl, tplh, riseTime, fallTime float64

	switch g := gate.(type) {
	case *TTLNand:
		// TTL has relatively fixed timing characteristics
		tphl = 7e-9    // HIGH to LOW: ~7 ns
		tplh = 11e-9   // LOW to HIGH: ~11 ns (slower pull-up)
		riseTime = 15e-9
		fallTime = 10e-9

	case *CMOSInverter:
		tphl, tplh, riseTime, fallTime = cmosTiming(
			g.Circuit.Vdd, g.Nmos, g.Pmos, cLoad)

	case *CMOSNand:
		tphl, tplh, riseTime, fallTime = cmosTiming(
			g.Circuit.Vdd, g.Nmos1, g.Pmos1, cLoad)

	case *CMOSNor:
		tphl, tplh, riseTime, fallTime = cmosTiming(
			g.Circuit.Vdd, g.Nmos1, g.Pmos1, cLoad)

	default:
		return TimingAnalysis{}, fmt.Errorf("unsupported gate type: %T", gate)
	}

	tpd := (tphl + tplh) / 2.0

	maxFrequency := math.Inf(1)
	if tpd > 0 {
		maxFrequency = 1.0 / (2.0 * tpd)
	}

	return TimingAnalysis{
		Tphl:         tphl,
		Tplh:         tplh,
		Tpd:          tpd,
		RiseTime:     riseTime,
		FallTime:     fallTime,
		MaxFrequency: maxFrequency,
	}, nil
}

// cmosTiming calculates CMOS timing parameters from transistor characteristics.
func cmosTiming(vdd float64, nmos *NMOS, pmos *PMOS, cLoad float64) (tphl, tplh, riseTime, fallTime float64) {
	k := nmos.Params.K
	vth := nmos.Params.Vth

	// Saturation current for NMOS pull-down
	idsSatN := 1e-12
	if vdd > vth {
		idsSatN = 0.5 * k * (vdd-vth) * (vdd - vth)
	}

	// Saturation current for PMOS pull-up
	idsSatP := 1e-12
	if vdd > pmos.Params.Vth {
		idsSatP = 0.5 * pmos.Params.K * (vdd - pmos.Params.Vth) * (vdd - pmos.Params.Vth)
	}

	// Propagation delays
	tphl = cLoad * vdd / (2.0 * idsSatN) // Pull-down (NMOS)
	tplh = cLoad * vdd / (2.0 * idsSatP) // Pull-up (PMOS)

	// Rise and fall times (2.2 RC time constants)
	rOnN := vdd / (2.0 * idsSatN)
	rOnP := vdd / (2.0 * idsSatP)
	riseTime = 2.2 * rOnP * cLoad
	fallTime = 2.2 * rOnN * cLoad

	return
}

// CompareCMOSvsTTL compares CMOS and TTL NAND gates across all metrics.
//
// This demonstrates WHY CMOS replaced TTL:
//   - CMOS has ~1000x less static power
//   - CMOS has better noise margins (relative to Vdd)
//   - CMOS can operate at lower voltages
func CompareCMOSvsTTL(frequency, cLoad float64) map[string]map[string]float64 {
	cmosNand := NewCMOSNand(nil, nil, nil)
	ttlNand := NewTTLNand(5.0, nil)

	cmosPower, _ := AnalyzePower(cmosNand, frequency, cLoad, 0.5)
	ttlPower, _ := AnalyzePower(ttlNand, frequency, cLoad, 0.5)

	cmosTiming, _ := AnalyzeTiming(cmosNand, cLoad)
	ttlTiming, _ := AnalyzeTiming(ttlNand, cLoad)

	cmosNM, _ := ComputeNoiseMargins(NewCMOSInverter(nil, nil, nil))
	ttlNM, _ := ComputeNoiseMargins(ttlNand)

	return map[string]map[string]float64{
		"cmos": {
			"transistor_count":      4,
			"supply_voltage":        cmosNand.Circuit.Vdd,
			"static_power_w":        cmosPower.StaticPower,
			"dynamic_power_w":       cmosPower.DynamicPower,
			"total_power_w":         cmosPower.TotalPower,
			"propagation_delay_s":   cmosTiming.Tpd,
			"max_frequency_hz":      cmosTiming.MaxFrequency,
			"noise_margin_low_v":    cmosNM.NML,
			"noise_margin_high_v":   cmosNM.NMH,
		},
		"ttl": {
			"transistor_count":      3,
			"supply_voltage":        ttlNand.Vcc,
			"static_power_w":        ttlPower.StaticPower,
			"dynamic_power_w":       ttlPower.DynamicPower,
			"total_power_w":         ttlPower.TotalPower,
			"propagation_delay_s":   ttlTiming.Tpd,
			"max_frequency_hz":      ttlTiming.MaxFrequency,
			"noise_margin_low_v":    ttlNM.NML,
			"noise_margin_high_v":   ttlNM.NMH,
		},
	}
}

// DemonstrateCMOSScaling shows how CMOS performance changes with technology scaling.
//
// As transistors shrink (Moore's Law):
//   - Gate length decreases -> faster switching
//   - Supply voltage decreases -> less power per switch
//   - Gate capacitance decreases -> less energy per transition
//   - BUT leakage current INCREASES -> more static power (the "leakage wall")
//
// If nodes is nil, defaults to [180nm, 90nm, 45nm, 22nm, 7nm, 3nm].
func DemonstrateCMOSScaling(nodes []float64) []map[string]float64 {
	if nodes == nil {
		nodes = []float64{180e-9, 90e-9, 45e-9, 22e-9, 7e-9, 3e-9}
	}

	results := make([]map[string]float64, 0, len(nodes))

	for _, node := range nodes {
		// Empirical scaling relationships (simplified)
		scale := node / 180e-9

		vdd := 3.3 * math.Sqrt(scale)
		if vdd < 0.7 {
			vdd = 0.7
		}

		vth := 0.4 * math.Pow(scale, 0.3)
		if vth < 0.15 {
			vth = 0.15
		}

		cGate := 1e-15 * scale
		k := 0.001 / math.Sqrt(scale)

		// Create transistor and circuit with scaled parameters
		params := MOSFETParams{Vth: vth, K: k, L: node, CGate: cGate,
			W: 1e-6, CDrain: 0.5e-15}
		circuit := CircuitParams{Vdd: vdd, Temperature: 300.0}
		inv := NewCMOSInverter(&circuit, &params, &params)

		loadCap := cGate * 10
		timing, _ := AnalyzeTiming(inv, loadCap)
		power, _ := AnalyzePower(inv, 1e9, loadCap, 0.5)

		// Leakage current increases exponentially as Vth decreases
		leakage := 1e-12 * math.Exp((0.4-vth)/0.052)

		results = append(results, map[string]float64{
			"node_nm":              node * 1e9,
			"vdd_v":               vdd,
			"vth_v":               vth,
			"c_gate_f":            cGate,
			"propagation_delay_s": timing.Tpd,
			"dynamic_power_w":     power.DynamicPower,
			"leakage_current_a":   leakage,
			"max_frequency_hz":    timing.MaxFrequency,
		})
	}

	return results
}
