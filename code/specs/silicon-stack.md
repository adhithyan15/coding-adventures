# Silicon Stack — Master Index

> **Status: Complete.** All 28 leaf specs have landed. Every section is filled in. The structural contract laid down in Phase 1 has held: no leaf spec required revising the index DAG.

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

## 2. The stack at a glance

```
                       Verilog (.v)        VHDL (.vhd)        Ruby DSL (.rb)
                       ───────────         ─────────────      ──────────────
                            │                    │                  │
                            ▼                    ▼                  ▼
                      verilog-parser      vhdl-parser     ruby-hdl-dsl.md
                       AST per F05         AST per F05    elaboration trace
                       + f05-full-ieee-extensions.md
                            │                    │                  │
                            └────────────┬───────┴──────────────────┘
                                         ▼
                              hdl-elaboration.md
                                         │
                                         ▼
                              ╔═════════════════╗
                              ║  hdl-ir.md      ║   ◀── KEYSTONE
                              ║  (HIR — unified)║
                              ╚════════╤════════╝
                                       │
            ┌──────────────┬───────────┼─────────────────┐
            ▼              ▼           ▼                 ▼
  hardware-vm.md     synthesis.md   coverage.md   real-fpga-export.md
       │                  │              ▲              (Verilog/EDIF
   vcd-writer.md          ▼              │               for yosys/nextpnr)
   testbench-                            │                 │
    framework.md  gate-netlist-format    │                 ▼
                  (HNL: generic gates)   │              external toolchain
                          │              │              (yosys → nextpnr →
                          ▼              │               icepack → iceprog)
                    tech-mapping.md  ◄───┘                 │
                          │                                │
                          ▼                                ▼
                  HNL: stdcell                       real iCE40 board
                          │
              ┌───────────┴───────────┐
              ▼                       ▼
      (FPGA path)              (ASIC path — Sky130)
              │                       │
   fpga-place-route-bridge.md   sky130-pdk.md
              │                       │
              ▼              standard-cell-library.md
   F01 fpga JSON config              │
              │                       ▼
              ▼                  lef-def.md
   fpga-bitstream.md                  │
   (iCE40 .bin)                       ▼
              │                  asic-floorplan.md
              ▼                       │
   real iCE40 board                   ▼
                                 asic-placement.md
                                       │
                                       ▼
                                 asic-routing.md
                                       │
                                       ▼
                                 gdsii-writer.md
                                       │
                                       ▼
                                  drc-lvs.md
                                       │
                                       ▼
                                  tape-out.md
                                       │
                                       ▼
                          Efabless chipIgnite shuttle
                                       │
                                       ▼
                                 Real silicon (~6 mo later)


  Analog substrate (used by sky130-pdk + standard-cell-library):
       device-physics.md → mosfet-models.md → spice-engine.md
                                                     ▲
                                       fab-process-simulation.md
```

**FPGA path** (left of split) — fast end-to-end win: HIR → synth → tech-map → fpga JSON → iCE40 bin → real hardware. Or the parallel `real-fpga-export.md` shortcut: HIR → Verilog → yosys/nextpnr toolchain.

**ASIC path** (right of split) — full silicon flow ending at GDSII + tape-out bundle.

**Cross-cutting** — `testbench-framework`, `coverage`, `vcd-writer` instrument any simulation; `spice-engine` characterizes cells for the ASIC path; `device-physics` and `mosfet-models` ground the SPICE results in physics; `fab-process-simulation` validates Sky130 parameters from first principles.

## 3. The 4-bit adder traced through every layer

The adder is the smoke-test example. Same circuit, different representations as it descends the stack:

| Layer | Spec | Representation |
|---|---|---|
| Source — Verilog | F05 + ext | `module adder4(input [3:0] a, b, input cin, output [3:0] sum, output cout); assign {cout, sum} = a + b + cin; endmodule` |
| Source — VHDL | F05 + ext | `entity adder4 is port (a,b: in std_logic_vector(3 downto 0); cin: in std_logic; sum: out std_logic_vector(3 downto 0); cout: out std_logic); ...` |
| Source — Ruby DSL | ruby-hdl-dsl | `class Adder4 < Module; io = Bundle.new(a:Input(UInt(4)), ...); io.sum := io.a +& io.b + io.cin; end` |
| AST | parsers (F05) | `ModuleDecl{name="adder4", ports=[...], body=[ContAssign(Concat(cout,sum), Add(Add(a,b),cin))]}` |
| HIR | hdl-ir | `Module{ports=..., cont_assigns=[ContAssign(Concat((PortRef("cout"), PortRef("sum"))), BinaryOp("+", BinaryOp("+", PortRef("a"), PortRef("b")), PortRef("cin")))]}` |
| Simulated | hardware-vm | a=0001, b=0010, cin=0 → sum=00011, cout=0 (one transition emitted to VCD per delta) |
| VCD trace | vcd-writer | `#0` `b0001 !` `b0010 "` `0 #` `#5000` `b0011 $` `0 %` (~3 KB total file) |
| Tested | testbench-framework | 256-vector exhaustive test passes; ALU testbench with reference model passes |
| Synthesized | synthesis | 4 × FullAdder, each = 2 XOR + 2 AND + 1 OR = 20 generic gates |
| Generic HNL | gate-netlist-format | `level=generic`; 20 instances; 5 internal nets; ~5 KB JSON |
| Tech-mapped (Sky130) | tech-mapping + sky130-pdk | 16 stdcells: 4 × `xor2_1`, 8 × `and2_1` (after AOI21 fold), 4 × `or2_1` (or 4 × `aoi22_1`); ~70 µm² estimated area |
| FPGA-mapped | fpga-place-route-bridge | 8 × LUT4 + 1 × LUT3 packed into 4 CLBs on a 2×2 fabric grid |
| FPGA JSON | F01-fpga + place-route-bridge | F01 schema: 4 CLBs configured, ~10 routes, ~6 KB |
| iCE40 bitstream | fpga-bitstream | ~135 KB `.bin` (mostly zeros — most tile config unused) |
| Real iCE40 board | (programmer step) | `iceprog adder4.bin` → flashed; LEDs respond to switches per truth table |
| Verilog re-emission | real-fpga-export | Identical structural Verilog re-emitted from HNL; passes `yosys -p "synth_ice40"` cleanly |
| Floorplan DEF | asic-floorplan | 27 µm × 28 µm die; 3 rows × 16 sites; 14 IO pins on boundary; VDD/VSS rings on met4/met5 |
| Placed DEF | asic-placement | 16 cells laid out left-to-right by bit position; HPWL ~25 µm |
| Routed DEF | asic-routing | ~30 metal segments on met1+met2; carry chain on met2; ~4 vias |
| GDSII | gdsii-writer | ~5 KB `.gds`; layers: nwell, pwell, diff, poly, li1, met1, met2, vias |
| DRC | drc-lvs | 0 violations |
| LVS | drc-lvs | layout extracts to ~480 transistors; isomorphic to schematic netlist ✓ |
| Tape-out bundle | tape-out | `adder4_chipignite.tar.gz`: GDS, LEF, DEF, Verilog, signoff reports, manifest.yaml |
| Real silicon | Efabless chipIgnite | ~6 months after submission; packaged QFN32 chip arrives in mail |
| SPICE-level verify | spice-engine + mosfet-models | Per-cell SPICE: NAND2_X1 propagation delay = 87 ps @ TT corner |
| Device physics | device-physics | NAND2's NMOS V_t derived from physics: 0.42 V (matches BSIM3v3 default) |
| Process | fab-process-simulation | NMOS cross-section from bare wafer: gate oxide 5 nm, V_t-adjust implant peak ~4e17/cm³, salicided source/drain |

The adder threads from `assign sum = a + b + cin` to packaged silicon in 25 distinct representations, each documented in its own spec.

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

## 6. Reading orders

Three suggested paths through the stack, with first-3-questions per stop.

### "Follow the data" (top-down) — for compiler-engineer minded readers

| Stop | Spec | First 3 questions to answer |
|---|---|---|
| 1 | f05-full-ieee-extensions | What's a synthesizable subset? Which IEEE constructs are added? Why "additive only"? |
| 2 | hdl-ir | What is HIR? Why one IR vs three? How is provenance preserved? |
| 3 | hdl-elaboration | What is the three-pass design? How does generate-for unroll? How is mixed-language handled? |
| 4 | ruby-hdl-dsl | What is tracing-style elaboration? How does `:=` work? What's a Bundle? |
| 5 | hardware-vm | What is a delta cycle? Why CPS-style processes? How does blocking vs non-blocking differ? |
| 6 | vcd-writer | What's the VCD format? How does identifier compaction work? |
| 7 | testbench-framework | Three test surfaces — when to use each? |
| 8 | coverage | Code vs functional coverage — when does each catch bugs? |
| 9 | gate-netlist-format | What's HNL? How does it differ from HIR? |
| 10 | synthesis | What does process classification do? How is FSM extraction done? |
| 11 | tech-mapping | Bubble-pushing? AOI/OAI folding? Drive-strength selection? |
| 12 | (split) FPGA: fpga-place-route-bridge / fpga-bitstream / real-fpga-export | LUT packing? PathFinder? iCE40 .bin format? |
| 13 | (split) ASIC: sky130-pdk → standard-cell-library → lef-def → asic-floorplan → asic-placement → asic-routing → gdsii-writer → drc-lvs → tape-out | Sky130 layer stack? Liberty timing? Floorplan utilization? SA placement? Lee routing? GDSII record format? DRC rules? Tape-out bundle? |

### "From electrons to gates" (bottom-up) — for physics-first readers

| Stop | Spec | First 3 questions |
|---|---|---|
| 1 | device-physics | Where do BSIM constants come from? What's the depletion approximation? How is V_t derived? |
| 2 | mosfet-models | Level-1 vs EKV vs BSIM3 — when to use each? What's the model interface? |
| 3 | spice-engine | What is MNA? Why Newton-Raphson? How does transient integration work? |
| 4 | fab-process-simulation | Deal-Grove? Implant Gaussian? Diffusion broadening? |
| 5 | sky130-pdk | What's in a PDK? Teaching vs full subset? PVT corners? |
| 6 | standard-cell-library | What's Liberty? Cell characterization methodology? Drive strengths? |
| 7 | tech-mapping | (cross-reference) |
| 8 | gate-netlist-format | (cross-reference) |
| 9 | synthesis → hdl-ir → hdl-elaboration → parsers | Climb the stack to source code |

### "I just want the adder to blink on real hardware" (minimum viable end-to-end)

| Stop | Spec | What you do |
|---|---|---|
| 1 | hdl-ir | Skim — just enough to understand what HIR is |
| 2 | real-fpga-export | Read carefully — this is the path |
| 3 | (action) | Write Verilog adder; emit via real-fpga-export driver |
| 4 | (action) | `yosys → nextpnr → icepack → iceprog` runs the toolchain |
| 5 | (action) | Plug iCE40-HX1K-EVN dev board into USB; LEDs blink per inputs |

Skip everything else for now. Come back later for synthesis, simulation, the ASIC track.

## 7. Glossary

| Term | Definition |
|---|---|
| **AOI / OAI** | And-Or-Invert / Or-And-Invert. Compound CMOS cells that fold multi-stage logic into one cell. |
| **AST** | Abstract Syntax Tree. Output of the parser. |
| **BSIM** | Berkeley Short-channel IGFET Model. Industry-standard MOSFET I-V model (BSIM3v3 for Sky130). |
| **CLB** | Configurable Logic Block. The repeating tile in an FPGA fabric. Contains LUTs + flip-flops. |
| **CPS** | Continuation-passing style. Used in `hardware-vm.md` to model wait/@ as suspension points. |
| **CRAM** | Configuration RAM. The on-FPGA SRAM that holds the bitstream. |
| **CTS** | Clock-Tree Synthesis. Builds a balanced buffer tree for the clock signal. |
| **DEF** | Design Exchange Format. Layout + placement + routing text format. |
| **DIBL** | Drain-induced barrier lowering. A short-channel effect captured by BSIM3v3. |
| **DRC** | Design Rule Check. Geometric correctness of layout. |
| **EDIF** | Electronic Design Interchange Format. LISP-like netlist format (used by `nextpnr`). |
| **EKV** | A MOSFET model (Enz-Krummenacher-Vittoz) — smooth all-region. |
| **FF** | Flip-flop. Sequential storage element. |
| **FPGA** | Field-Programmable Gate Array. Reconfigurable hardware. |
| **GDSII** | Calma Stream Format. Binary mask layout file (the format fabs accept). |
| **HDL** | Hardware Description Language (Verilog, VHDL, Ruby DSL in this stack). |
| **HIR** | Hardware IR. Internal representation; defined in `hdl-ir.md`. |
| **HNL** | Hardware NetList. Canonical netlist format; defined in `gate-netlist-format.md`. |
| **HPWL** | Half-perimeter wirelength. Cost function for placement. |
| **IR** | Intermediate Representation. |
| **iCE40** | A family of small Lattice FPGAs supported by Project IceStorm (open-source toolchain). |
| **LEF** | Library Exchange Format. Cell library + tech rules text format. |
| **LTE** | Local Truncation Error. Used to size SPICE timesteps. |
| **LUT** | Look-Up Table. The atom of FPGA programmable logic. |
| **LVS** | Layout vs Schematic. Verifies layout matches netlist. |
| **MNA** | Modified Nodal Analysis. The algorithm at the heart of every SPICE engine. |
| **NBA** | Non-Blocking Assignment. Verilog `<=`. Deferred update; visible at next delta. |
| **PDK** | Process Design Kit. The technology files for a fab process. |
| **PEX** | Parasitic Extraction. Adds RC parasitics to post-layout netlist. |
| **PSL** | Property Specification Language. Assertion language (IEEE 1850). |
| **PVT** | Process, Voltage, Temperature corner. |
| **RTL** | Register-Transfer Level. The abstraction synthesis works on. |
| **SA** | Simulated Annealing. Used for placement. |
| **SDF** | Standard Delay Format. Back-annotation file for timing. |
| **Sky130** | The open-source 130nm PDK from SkyWater. Our target process. |
| **SPICE** | Simulation Program with Integrated Circuit Emphasis. The analog simulator. |
| **SREF** | Structure Reference. GDSII record for instantiating a cell by reference. |
| **STA** | Static Timing Analysis. |
| **TBUF** | Tristate Buffer. A buffer with output-enable. |
| **VCD** | Value Change Dump. Waveform format (IEEE 1364 §18). |

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

## 12. Verification (how we know the stack works)

End-to-end verification gates, each with the specific tests in the responsible leaf spec.

| Gate | Test | Defined in |
|---|---|---|
| **Smoke test** | 4-bit adder runs through every layer; produces tape-out bundle. | Worked Example 1 in every spec |
| **Mid-scale test** | 32-bit ALU runs through every layer; produces tape-out bundle. | Worked Example 2 in synthesis, hardware-vm, asic-placement, asic-routing, etc. |
| **Reference design** | ARM1 (`arm1-gatelevel`) runs through; gate count comparable to existing reference. | Test Strategy in synthesis, tech-mapping |
| **Real-hardware FPGA** | iCE40 bitstream programs a real iCE40-HX1K-EVN dev board; adder responds to switch inputs per truth table. | real-fpga-export Test Strategy + Worked Example |
| **Cross-validation vs yosys/nextpnr** | Our synth+PnR results within X% of yosys/nextpnr on benchmark suite. | synthesis Test Strategy; fpga-place-route-bridge Test Strategy |
| **Cross-validation vs OpenROAD** | Our placement HPWL within 10% of OpenROAD's RePlAce; routing within 30% of TritonRoute. | asic-placement, asic-routing |
| **Industry interchange** | Round-trip HNL → BLIF → HNL: structurally identical. Round-trip HNL → EDIF → HNL: same. GDSII round-trips through KLayout. | gate-netlist-format Test Strategy; gdsii-writer |
| **IEEE 1076-2008 / 1364-2005 conformance** | Parse + elaborate IEEE reference suite vectors. | f05-full-ieee-extensions; hdl-ir; hdl-elaboration |
| **Sky130 V_t derivation** | Compute V_t from physics; matches BSIM3v3 default within 10%. | device-physics + fab-process-simulation Worked Examples |
| **Cell characterization vs Sky130 reference** | Re-characterize teaching subset; match published Liberty within 10% (combinational), 15% (sequential). | standard-cell-library Worked Example 3 |
| **DRC clean** | 4-bit adder GDS DRC-clean against Sky130 teaching rule deck. | drc-lvs Worked Example 1 |
| **LVS match** | 4-bit adder layout extracts to ~480 transistors isomorphic to schematic netlist. | drc-lvs Worked Example 2 |
| **Tape-out bundle valid** | `validate_for_chipignite(bundle)` passes. | tape-out Worked Example |
| **Real silicon (optional)** | Submit to Efabless chipIgnite shuttle; receive packaged QFN chip ~6 mo later; bring up; verify behavior on bench. | tape-out future-work |
| **Determinism** | Every spec's algorithm produces identical results given identical inputs + seed. | Property tests in every spec |

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
