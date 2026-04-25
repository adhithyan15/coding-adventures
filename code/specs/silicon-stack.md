# Silicon Stack — Master Index

> **Status: Outline (Phase-1 contract).** This document is a structural skeleton committed during Phase 1 to keep all leaf specs aligned. Each section header is final; section content is filled in as the corresponding leaf specs land. Sections marked `[STUB]` will be expanded in Phase 7 once all leaves exist; sections marked `[FROZEN]` are complete and committed-to as part of the contract.

## 0. Purpose `[FROZEN]`

This spec is the top-level index for the digital hardware design flow in this repository. It exists to answer three questions for any reader who arrives without context:

1. **What is in this stack?** A complete, working pipeline from human-readable HDL source code (Verilog, VHDL, or a Ruby DSL) all the way down to files that can be sent to a semiconductor fab to produce real silicon, plus a parallel branch that produces FPGA bitstreams for off-the-shelf programmable hardware.
2. **Where does each piece sit?** A layered diagram and dependency graph that places every individual spec in the stack relative to its neighbors.
3. **How do I navigate it for *my* question?** Reading orders organized by intent: top-down ("follow the data"), bottom-up ("from electrons to gates"), and end-to-end ("I want a 4-bit adder running on real hardware").

The stack is intentionally general-purpose. The 4-bit adder appears throughout as a smoke-test example, but every component scales — from a single-LUT design to a multi-million-cell SoC — without architectural change.

## 1. Scope and goals `[FROZEN]`

### In scope
- Verilog (IEEE 1364-2005), VHDL (IEEE 1076-2008), and a Ruby HDL DSL as front-end input languages.
- Simulation of any synthesizable or testbench HDL with full IEEE-compliant semantics.
- Synthesis from HDL to a generic gate netlist.
- Tech mapping from generic gates to a Sky130-compatible standard cell library.
- Place & route to the existing `fpga` package's CLB grid, AND emission of structural Verilog for the open-tool flow (`yosys`/`nextpnr`/`icepack`) targeting iCE40 FPGAs.
- ASIC backend: floorplan, placement, routing, GDSII export.
- Verification: testbenches, coverage, SPICE simulation, DRC, LVS.
- Tape-out bundle preparation for the Efabless chipIgnite Sky130 shuttle.

### Out of scope (documented as future)
- SystemVerilog (IEEE 1800).
- Mixed-signal / analog-mixed-signal (Verilog-AMS, VHDL-AMS).
- 3-D ICs and chiplets.
- Quantum / photonic / non-CMOS technologies.
- Beyond-Sky130 PDKs (although the architecture should accommodate them with re-parameterization).

### What "done" means
- The 4-bit adder can be written in any of the three front-ends, simulated, synthesized, mapped to Sky130 cells, placed and routed, exported to GDSII, signed off (DRC/LVS clean), and bundled for tape-out.
- The same flow works on a 32-bit ALU.
- The same flow works on a small RISC-V core.
- An iCE40 FPGA bitstream is produced and successfully programs a real iCE40-HX1K-EVN dev board.

## 2. The stack at a glance `[STUB — will hold the canonical layered diagram once all specs land]`

Layered ASCII diagram from HDL source at top to GDSII / FPGA bitstream at bottom. Each box labeled with the corresponding spec filename. The two-track structure (FPGA path on left, ASIC path on right) made explicit. Cross-cutting concerns (testbench, coverage, SPICE) shown as side-channels.

(Phase-7 task to populate.)

## 3. The 4-bit adder traced through every layer `[STUB]`

A single concrete table threading the 4-bit adder through every spec. Each row is a layer; each cell is the adder's representation at that layer (a code snippet, a JSON fragment, an ASCII diagram, or a screenshot reference). When complete, this is the most useful single page in the entire stack — a reader who wants to know "what does my circuit look like at layer X?" finds it here.

(Phase-7 task: populate one row per spec as that spec's worked example crystallizes.)

## 4. Specs in this stack `[FROZEN — table of contents]`

### Frontend & IR (Band A)
- `f05-full-ieee-extensions.md` — Extends F05 grammars to full IEEE 1076-2008 / 1364-2005.
- `hdl-ir.md` — **Keystone.** The unified Hardware IR (HIR) that all front-ends produce and all back-ends consume.
- `hdl-elaboration.md` — How AST + DSL traces become elaborated HIR.
- `ruby-hdl-dsl.md` — Chisel-style Ruby DSL as the third HDL front-end.

### Simulation (Band B)
- `hardware-vm.md` — Event-driven simulation kernel with delta cycles. The "Hardware VM."
- `vcd-writer.md` — Value Change Dump format for waveform output.
- `testbench-framework.md` — Testbench DSL + assertions + stimulus.
- `coverage.md` — Code and functional coverage measurement.

### Synthesis (Band C)
- `gate-netlist-format.md` — HNL (Hardware NetList): the canonical netlist data structure and JSON format.
- `synthesis.md` — HIR → HNL with generic gates; RTL inference + FSM extraction + arithmetic mapping.
- `tech-mapping.md` — Generic HNL → standard-cell HNL.

### FPGA (Band D)
- `fpga-place-route-bridge.md` — HNL → existing `fpga` package's JSON config.
- `fpga-bitstream.md` — Real iCE40 bitstream format.
- `real-fpga-export.md` — HNL/HIR → structural Verilog/EDIF for `yosys`/`nextpnr`.

### ASIC backend (Band E)
- `sky130-pdk.md` — Sky130 process design kit integration.
- `standard-cell-library.md` — Liberty-style characterized cells.
- `lef-def.md` — LEF (cell library) and DEF (design exchange) formats.
- `asic-floorplan.md` — Die area, IO ring, power grid.
- `asic-placement.md` — Place every standard cell on a legal site.
- `asic-routing.md` — Route every net with metal traces, respecting DRC.
- `gdsii-writer.md` — Binary GDSII output for fab consumption.
- `drc-lvs.md` — Design Rule Check + Layout-vs-Schematic verification.
- `tape-out.md` — Bundle preparation for Efabless chipIgnite shuttle.

### Analog & physics (Band F)
- `device-physics.md` — Drift-diffusion and MOSFET threshold derivation from first principles.
- `mosfet-models.md` — Level-1 / EKV / BSIM3 MOSFET I-V models.
- `spice-engine.md` — Modified Nodal Analysis with transient/DC/AC solvers.
- `fab-process-simulation.md` — Oxidation, lithography, etching, doping, planarization.

### Index
- `silicon-stack.md` — This document.

## 5. Spec dependency graph `[FROZEN — DAG]`

```
                                   device-physics.md
                                          │
                                          ▼
                                   mosfet-models.md
                                          │
                                          ▼
                                    spice-engine.md  ◄── (used by) standard-cell-library
                                          │                              ▲
                                          ▼                              │
                                  fab-process-simulation.md              │
                                                                         │
                                                                         │
   F05 (existing) ─── f05-full-ieee-extensions.md                        │
                              │                                          │
                              ▼                                          │
                          hdl-ir.md (keystone)                           │
                  ┌───────────┼───────────────┐                          │
                  ▼           ▼               ▼                          │
       hdl-elaboration  ruby-hdl-dsl  gate-netlist-format ───┐           │
                  │           │              │              │           │
                  └───────────┴──────────────┘              │           │
                                  │                         │           │
                                  ▼                         ▼           │
                           hardware-vm.md            synthesis.md       │
                                  │                         │           │
              ┌───────────────────┼─────┐                   ▼           │
              ▼                   ▼     ▼            tech-mapping.md ◄──┘
       vcd-writer  testbench-framework coverage             │
                                                            │
                  ┌─────────────────────────────────────────┴────────┐
                  ▼                                                  ▼
          (FPGA path)                                       (ASIC path)
        fpga-place-route-bridge ──► fpga-bitstream             sky130-pdk
        real-fpga-export                                          │
                                                          standard-cell-library
                                                                  │
                                                              lef-def
                                                                  │
                                                          asic-floorplan
                                                                  │
                                                          asic-placement
                                                                  │
                                                          asic-routing
                                                                  │
                                                          gdsii-writer
                                                                  │
                                                              drc-lvs
                                                                  │
                                                              tape-out
```

The DAG is acyclic. No spec depends on a spec downstream of it.

## 6. Reading orders `[STUB]`

Three suggested paths through the stack:

### "Follow the data" (top-down)
For a reader who thinks like a compiler engineer: starts at HDL source, traces every transformation down to silicon. Spec sequence: `f05-full-ieee-extensions` → `hdl-ir` → `hdl-elaboration` → `ruby-hdl-dsl` → `hardware-vm` → `vcd-writer` → `testbench-framework` → `coverage` → `gate-netlist-format` → `synthesis` → `tech-mapping` → (split) → FPGA path / ASIC path → `tape-out` → `silicon-stack` (this doc, re-read).

### "From electrons to gates" (bottom-up)
For a reader who wants to understand silicon physically before getting to logic. Spec sequence: `device-physics` → `mosfet-models` → `spice-engine` → `fab-process-simulation` → `sky130-pdk` → `standard-cell-library` → `tech-mapping` → `gate-netlist-format` → `synthesis` → `hdl-ir` → ... → top.

### "I just want the adder to blink" (minimum viable end-to-end)
For a reader who wants to see something work fast. Spec sequence: `hdl-ir` (skim) → `real-fpga-export` (full read) → run `yosys` → `nextpnr` → `icepack` → flash to a real iCE40 board. Skip simulation, skip ASIC, skip everything that doesn't lead to bits-on-FPGA. Comes back to the rest later.

(Phase-7 task: populate each path with concrete first-3-questions per spec.)

## 7. Glossary `[STUB — accumulating]`

Maintain a single glossary of every acronym used anywhere in the stack. Phase-7 task: gather from all leaf specs and consolidate. Start of seed list:

| Term | Definition |
|---|---|
| **AST** | Abstract Syntax Tree. Output of the parser. |
| **HIR** | Hardware IR. Internal representation; defined in `hdl-ir.md`. |
| **HNL** | Hardware NetList. Canonical netlist format; defined in `gate-netlist-format.md`. |
| **CLB** | Configurable Logic Block. The repeating tile in an FPGA fabric. |
| **LUT** | Look-Up Table. The atom of FPGA programmable logic. |
| **PDK** | Process Design Kit. The technology files for a fab process. |
| **PVT** | Process, Voltage, Temperature corner. |
| **DRC** | Design Rule Check. Geometric correctness of layout. |
| **LVS** | Layout vs Schematic. Verifies layout matches netlist. |
| **GDSII** | Calma Stream Format. Binary mask layout file. |
| **LEF** | Library Exchange Format. Cell library + tech rules text format. |
| **DEF** | Design Exchange Format. Layout + placement + routing text format. |
| **MNA** | Modified Nodal Analysis. The algorithm at the heart of every SPICE engine. |
| **BSIM** | Berkeley Short-channel IGFET Model. Industry-standard MOSFET I-V model. |
| **VCD** | Value Change Dump. Waveform format. |
| **EDIF** | Electronic Design Interchange Format. LISP-like netlist format. |
| **BLIF** | Berkeley Logic Interchange Format. Flat truth-table-based netlist. |
| **SDF** | Standard Delay Format. Back-annotation file for timing. |
| **PSL** | Property Specification Language. Assertion language. |
| **PEX** | Parasitic Extraction. Adds RC parasitics to post-layout netlist. |

## 8. Standards index `[FROZEN]`

| Standard | Specs that reference it |
|---|---|
| **IEEE 1076-2008** (VHDL) | f05-full-ieee-extensions.md, hdl-ir.md, hdl-elaboration.md |
| **IEEE 1364-2005** (Verilog) | f05-full-ieee-extensions.md, hdl-ir.md, hdl-elaboration.md |
| **IEEE 1800** (SystemVerilog) | Out of scope; future spec systemverilog-extensions.md |
| **IEEE 1850** (PSL) | f05-full-ieee-extensions.md (parse-skip); future spec |
| **IEEE 1497** (SDF) | hardware-vm.md, asic-routing.md (back-annotation) |
| **IEEE 1481** (Delay calculation) | standard-cell-library.md (Liberty timing) |
| **IEEE 1164** (std_logic / std_ulogic types) | hdl-ir.md, hardware-vm.md |
| **IEEE 1076.6** (VHDL synthesis subset) | synthesis.md (referenced for synthesis rules) |
| **EDIF 2.0.0** | gate-netlist-format.md, real-fpga-export.md |
| **BLIF (Sentovich, 1992)** | gate-netlist-format.md |
| **GDSII (Calma Stream Format)** | gdsii-writer.md |
| **LEF/DEF** (Si2 / Cadence) | lef-def.md |
| **Liberty (`.lib`)** (Synopsys) | standard-cell-library.md |
| **SPICE3 netlist** | spice-engine.md |
| **VCD** (defined in IEEE 1364 §18) | vcd-writer.md |
| **Sky130 PDK** (Google/Skywater) | sky130-pdk.md |
| **SPEF** (parasitics) | Future; referenced in asic-routing.md |

## 9. Cross-package side-channels `[FROZEN]`

Three existing packages in the repo are shared across multiple specs and act as common substrate:

- **`logic-gates`** package — provides AND/OR/NAND/etc. behavioral cells. Used by `synthesis.md` (target of generic-gate output) and `hardware-vm.md` (cell-level simulation when no HDL is involved).
- **`transistors`** package — provides NMOS/PMOS behavioral models. Used by `standard-cell-library.md` (cell schematics built from transistor primitives) and `spice-engine.md` (extended with physical models from `mosfet-models.md`).
- **`fpga`** package — existing FPGA fabric simulator with CLB/LUT/switch-matrix model. Used by `fpga-place-route-bridge.md` (target of HNL → JSON conversion).

Two existing specs are referenced by the new specs:
- `F05-verilog-vhdl.md` — base HDL grammar specs.
- `F01-fpga.md` — existing FPGA simulator architecture.

Two existing data structures are reused:
- `directed-graph` package (per `F02-graph-foundations.md`) — used by `hardware-vm.md` (event scheduling), `synthesis.md` (cell graph), `asic-routing.md` (routing graph), `gate-netlist-format.md` (cycle detection).
- The grammar-driven lexer/parser infrastructure (`02-lexer.md`, `03-parser.md`, `F04-lexer-pattern-groups.md`) — used by every parser in the stack.

## 10. Generality and N-scaling `[FROZEN]`

Although the canonical worked example threading every spec is the 4-bit adder, the architecture must scale to arbitrary digital circuits. Each spec includes:

- A **second worked example** larger than the adder (typical: 32-bit ALU, 4-state FSM, register file, the existing `arm1-gatelevel` / `intel4004-gatelevel` reference cores).
- A **scaling note** describing how time complexity, space complexity, and any algorithmic limits behave as N (cells, nets, bits) grows.
- Where applicable, a **gracefully degraded mode** for designs too large for in-memory representation (streaming, on-disk).

The 4-bit adder is the smoke test, not the architectural ceiling.

## 11. Workflow conventions `[FROZEN]`

Per `CLAUDE.md`:
- **Specs first.** No implementation begins until the corresponding spec is committed to main.
- **Feature branches.** Each spec or coherent batch lands on its own feature branch.
- **Detailed commit messages.** Commits should explain *why*, not just *what*.
- **Per-package quality bars.** Every implementation package must have a `BUILD`, `README.md`, `CHANGELOG.md`, full type annotations (Python `mypy --strict`), and 95%+ test coverage for libraries (80%+ for programs).
- **Spec-implementation drift.** When implementation diverges from spec, the spec is updated and the divergence noted in the commit message. Specs are the source of truth.
- **PR review.** Every PR runs `/security-review` and `/babysit-pr`.

## 12. Verification (how we know the stack works) `[STUB]`

End-to-end verification gates:

1. **Smoke test:** 4-bit adder runs through every layer and produces a tape-out bundle.
2. **Mid-scale test:** 32-bit ALU runs through every layer and produces a tape-out bundle.
3. **Real-hardware test:** an iCE40 bitstream programs a real Lattice iCE40-HX1K-EVN dev board and the adder operates correctly with hardware test vectors.
4. **Cross-check:** our synthesis + P&R produces results within X% of `yosys` + `nextpnr` for a benchmark suite.
5. **Industry interchange:** designs round-trip through EDIF, BLIF, Verilog, LEF/DEF, GDSII without semantic loss.
6. **Standards conformance:** IEEE 1076-2008 / 1364-2005 reference suites parse and elaborate correctly.

(Phase-7 task: cite specific tests in each leaf spec.)

## 13. Open Questions and Future Work `[FROZEN]`

The following architectural decisions remain open and may be revisited:

1. **AIG (And-Inverter Graph) optimization layer.** Modern synthesis tools (yosys via ABC) use AIG as an intermediate layer for technology-independent optimization. We have deferred AIG to a future spec. If synthesis quality is unacceptable on real designs, an AIG layer is the natural next addition.

2. **Mixed-signal coupling.** `hardware-vm.md` and `spice-engine.md` are independent. A future `mixed-signal-bridge.md` spec would couple them for AMS simulation.

3. **TCAD-grade fab simulation.** `fab-process-simulation.md` uses 1-D analytical models. A future spec could add 2-D or 3-D mesh-based PDE solvers.

4. **3-D / chiplet architectures.** All current specs assume a single 2-D die. Multi-die / chiplet design is future work.

5. **Beyond Sky130.** A `pdk-abstraction.md` spec would let standard-cell-library and ASIC backend support multiple PDKs simultaneously (Sky130, GF180MCU, ASAP7).

6. **Power signoff.** Static and dynamic power estimation, IR-drop analysis, electromigration are all out of scope for v1; `power-signoff.md` is future work.

7. **Polyglot porting strategy.** All specs are language-agnostic, but the implementation reference is Python. After Python lands, the existing repo polyglot-port pattern applies (Ruby, Go, Rust, TypeScript, etc.).

## 14. Document conventions `[FROZEN]`

Every leaf spec in this stack follows the inherited F01/F05 style:
- Overview + analogy.
- Layer-position diagram.
- Comparison tables.
- ASCII art for structure.
- EBNF grammars where applicable.
- Python API sketches with `mypy --strict` annotations.
- 4-bit adder worked example **AND** at least one larger worked example.
- Edge-case table.
- Test strategy.
- IEEE/industry conformance matrix.
- Knuth-style literate prose (derivations, analogies, intuition).
- Open questions and future work.

The template is enforced by reviewer checklist; deviations require justification.

---

*This document is the contract that holds the silicon stack together. As leaf specs land, the `[STUB]` sections fill in. When the stack is complete, every reader can navigate from any question to the spec that answers it within two hops.*
