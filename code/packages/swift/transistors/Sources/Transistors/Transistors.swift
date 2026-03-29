// Transistors.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// Transistors — Module Entry Point
// ============================================================================
//
// This module simulates semiconductor transistors and the logic gates built
// from them. It is the foundation of the computing stack — every logic gate,
// every flip-flop, every CPU register ultimately reduces to the physics
// implemented here.
//
// # Module Structure
//
//   Types.swift      — Parameter structs, region enums, output records
//   MOSFET.swift     — NMOS and PMOS transistor physics
//   BJT.swift        — NPN and PNP bipolar transistor physics
//   CMOSGates.swift  — CMOS inverter, NAND, NOR, AND, OR, XOR gates
//   TTLGates.swift   — TTL NAND and RTL inverter
//   Amplifier.swift  — Single-stage amplifier analysis
//   Analysis.swift   — Noise margins, power, timing, CMOS vs TTL comparison
//
// # Usage
//
//   import Transistors
//
//   // Evaluate a CMOS AND gate digitally
//   let and = CMOSAnd()
//   let result = and.evaluateDigital(1, 1)  // → 1
//
//   // Full physics evaluation with analog voltages
//   let nand = CMOSNand()
//   let out = nand.evaluate(va: 1.8, vb: 1.8)
//   print(out.logicValue)          // 0
//   print(out.powerDissipation)    // in watts
//   print(out.propagationDelay)    // in seconds
//
//   // Amplifier analysis
//   let nmos = NMOS()
//   let amp = analyzeCommonSource(transistor: nmos, vgs: 0.8, vdd: 1.8,
//                                  rDrain: 10_000, cLoad: 10e-15)
//   print(amp.voltageGain)         // negative (inverting)
//   print(amp.bandwidth)           // in Hz
//
// ============================================================================

// All public types, structs, functions, and enums are defined in the
// companion source files and are automatically part of this module.
// Swift does not require explicit re-exports within a single target.

/// The version of this package.
public let version = "0.1.0"
