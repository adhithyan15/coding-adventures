package transistors

// Analog Amplifier Analysis — transistors as signal amplifiers.
//
// === Beyond Digital: Transistors as Amplifiers ===
//
// A transistor used as a digital switch operates in only two states: ON/OFF.
// But transistors are fundamentally ANALOG devices. When biased in the right
// operating region (saturation for MOSFET, active for BJT), they can amplify
// small signals into larger ones.
//
// === Common-Source Amplifier (MOSFET) ===
//
//	    Vdd
//	     |
//	    [Rd]  <- voltage drop = Ids x Rd
//	     |
//	-----| Drain (output)
//	Gate -||
//	-----| Source
//	     |
//	    GND
//
//	Voltage gain: Av = -gm x Rd (inverting amplifier)
//
// === Common-Emitter Amplifier (BJT) ===
//
//	    Vcc
//	     |
//	    [Rc]
//	     |
//	-----| Collector (output)
//	Base -|-->
//	-----| Emitter
//	     |
//	    GND
//
//	Voltage gain: Av = -gm x Rc = -(Ic/Vt) x Rc

import "math"

// AnalyzeCommonSourceAmp analyzes an NMOS common-source amplifier.
//
// The input signal is applied to the gate, and the output is taken from
// the drain. A drain resistor (rDrain) converts drain current variation
// into a voltage swing.
//
// For the amplifier to work, the MOSFET must be biased in SATURATION:
// Vgs > Vth AND Vds >= Vgs - Vth.
func AnalyzeCommonSourceAmp(t *NMOS, vgs, vdd, rDrain, cLoad float64) AmplifierAnalysis {
	result, _ := StartNew[AmplifierAnalysis]("transistors.AnalyzeCommonSourceAmp", AmplifierAnalysis{},
		func(op *Operation[AmplifierAnalysis], rf *ResultFactory[AmplifierAnalysis]) *OperationResult[AmplifierAnalysis] {
			op.AddProperty("vgs", vgs)
			op.AddProperty("vdd", vdd)
			// Calculate DC operating point
			ids := t.DrainCurrent(vgs, vdd)
			vds := vdd - ids*rDrain

			// Recalculate with correct Vds
			vdsActual := vds
			if vdsActual < 0 {
				vdsActual = 0
			}
			ids = t.DrainCurrent(vgs, vdsActual)
			vds = vdd - ids*rDrain

			// Transconductance
			gm := t.Transconductance(vgs, vdsActual)

			// Voltage gain: Av = -gm x Rd (negative = inverting)
			voltageGain := -gm * rDrain

			// Input impedance: essentially infinite for MOSFET (gate is insulated)
			inputImpedance := 1e12

			// Output impedance: approximately Rd
			outputImpedance := rDrain

			// Bandwidth: f_3dB = 1 / (2 * pi * Rd * C_load)
			bandwidth := 1.0 / (2.0 * math.Pi * rDrain * cLoad)

			operatingPoint := map[string]float64{
				"vgs": vgs,
				"vds": vds,
				"ids": ids,
				"gm":  gm,
			}

			return rf.Generate(true, false, AmplifierAnalysis{
				VoltageGain:      voltageGain,
				Transconductance: gm,
				InputImpedance:   inputImpedance,
				OutputImpedance:  outputImpedance,
				Bandwidth:        bandwidth,
				OperatingPoint:   operatingPoint,
			})
		}).GetResult()
	return result
}

// AnalyzeCommonEmitterAmp analyzes an NPN common-emitter amplifier.
//
// BJT amplifiers typically have higher voltage gain than MOSFET amplifiers
// at the same current, because BJT transconductance (gm = Ic/Vt) is
// higher than MOSFET transconductance for the same bias current.
//
// However, BJT amplifiers have lower input impedance because base current
// flows continuously.
func AnalyzeCommonEmitterAmp(t *NPN, vbe, vcc, rCollector, cLoad float64) AmplifierAnalysis {
	result, _ := StartNew[AmplifierAnalysis]("transistors.AnalyzeCommonEmitterAmp", AmplifierAnalysis{},
		func(op *Operation[AmplifierAnalysis], rf *ResultFactory[AmplifierAnalysis]) *OperationResult[AmplifierAnalysis] {
			op.AddProperty("vbe", vbe)
			op.AddProperty("vcc", vcc)
			// Calculate DC operating point
			vce := vcc
			ic := t.CollectorCurrent(vbe, vce)
			vce = vcc - ic*rCollector
			if vce < 0 {
				vce = 0
			}

			// Recalculate with correct Vce
			ic = t.CollectorCurrent(vbe, vce)

			// Transconductance
			gm := t.Transconductance(vbe, vce)

			// Voltage gain: Av = -gm x Rc
			voltageGain := -gm * rCollector

			// Input impedance: r_pi = beta * Vt / Ic
			beta := t.Params.Beta
			vt := 0.026
			var rPi float64
			if ic > 0 {
				rPi = beta * vt / ic
			} else {
				rPi = 1e12
			}

			inputImpedance := rPi
			outputImpedance := rCollector

			// Bandwidth: f_3dB = 1 / (2 * pi * Rc * C_load)
			bandwidth := 1.0 / (2.0 * math.Pi * rCollector * cLoad)

			operatingPoint := map[string]float64{
				"vbe": vbe,
				"vce": vce,
				"ic":  ic,
				"ib":  t.BaseCurrent(vbe, vce),
				"gm":  gm,
			}

			return rf.Generate(true, false, AmplifierAnalysis{
				VoltageGain:      voltageGain,
				Transconductance: gm,
				InputImpedance:   inputImpedance,
				OutputImpedance:  outputImpedance,
				Bandwidth:        bandwidth,
				OperatingPoint:   operatingPoint,
			})
		}).GetResult()
	return result
}
