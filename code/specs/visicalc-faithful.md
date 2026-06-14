# VisiCalc Faithful Reconstruction

## Overview

VisiCalc was the first spreadsheet program, released in October 1979
on the Apple II. Dan Bricklin and Bob Frankston wrote it in 6502
assembly to fit in 32 KB of RAM. The 27 KB binary that shipped on the
original distribution diskette is the cultural artifact that turned
the Apple II from a hobbyist machine into a serious business tool and
that defined the design vocabulary every spreadsheet since has
inherited.

This spec lays out the **faithful reconstruction track**: running the
real 1979 VisiCalc binary on a from-scratch Apple II machine
emulator, sitting on top of the existing Python `mos6502-simulator`
in this repo. The goal is not to be the fastest Apple II emulator
(it will be slow) and not to support every Apple II program (it
won't) — the goal is to make the artifact run again, in the
literate-programming style this repo prefers, with the surrounding
hardware specified and implemented well enough that VisiCalc itself,
as the canary application, runs end-to-end.

A separate spec, `visicalc-modern.md`, covers the *modern*
reconstruction in Rust on the Mosaic UI substrate. The two tracks
share specs and a vision but live in different language ecosystems
and on different platforms. They are siblings, not layers.

---

## Why the Faithful Track Belongs in Python

The existing 6502 simulator at `code/packages/python/mos6502-simulator/`
is mature: 151 official NMOS opcodes, 13 addressing modes, BCD
decimal-mode arithmetic, the indirect-JMP page-wrap bug,
cycle-accurate behavioral simulation. Building the Apple II machine
layer in Python keeps it next to the simulator it depends on, lets
us reuse the same `SIM00 Simulator[MOS6502State]` protocol for
testing, and avoids cross-language plumbing for what is essentially
hardware emulation.

The "real implementation" of the spreadsheet (formula engine, statistics
library, modern UI) moves to Rust per the architecture in
`statistics-core.md` and `spreadsheet-core.md`. The faithful track
does **not** share code with the modern track. It does not need a
formula engine — VisiCalc *is* its own formula engine, baked into the
6502 binary. It does not need a statistics core — VisiCalc's 25
functions are inside the binary too. The faithful track exists to run
the binary, not to interoperate with the rest of the Rust stack.

---

## Where It Fits

```
   Apple II VisiCalc disk image (.dsk file, 140 KB)
                       │
                       │  VisiCalc binary loaded into memory at $1000
                       ▼
   ┌──────────────────────────────────────────────────────────┐
   │             Apple II Machine Layer (Python)              │
   │                                                          │
   │   ┌────────────────────────────────────────────────────┐ │
   │   │  apple-ii-machine                                  │ │
   │   │   memory map  ($0000–$FFFF)                        │ │
   │   │   soft switches ($C000–$C0FF)                      │ │
   │   │   text page ($0400–$07FF) → 40×24 character grid   │ │
   │   │   lores page ($0400–$07FF, alternate use)          │ │
   │   │   hires pages ($2000–$3FFF, $4000–$5FFF)           │ │
   │   │   keyboard strobe ($C000), $C010 clear             │ │
   │   │   speaker click ($C030)                            │ │
   │   │   paddle buttons ($C061–$C063)                     │ │
   │   │   language card switching ($C080–$C08F)            │ │
   │   └─────────────────────┬──────────────────────────────┘ │
   │                         │                                │
   │   ┌─────────────────────▼──────────────────────────────┐ │
   │   │  apple-ii-disk                                     │ │
   │   │   Disk II controller hardware ($C0E0–$C0EF)        │ │
   │   │   .dsk and .nib image formats                      │ │
   │   │   GCR encoding / decoding                          │ │
   │   │   sector skewing                                   │ │
   │   │   DOS 3.3 RWTS routines (or Apple ProDOS for v2)  │ │
   │   └─────────────────────┬──────────────────────────────┘ │
   │                         │                                │
   │   ┌─────────────────────▼──────────────────────────────┐ │
   │   │  apple-ii-display                                  │ │
   │   │   text-page → ASCII art (terminal frontend)        │ │
   │   │   text-page → glyph buffer (GUI frontend)          │ │
   │   │   refresh on $0400-$07FF write                     │ │
   │   └─────────────────────┬──────────────────────────────┘ │
   │                         │                                │
   │                         ▼                                │
   │   ┌────────────────────────────────────────────────────┐ │
   │   │  mos6502-simulator (existing — Python)             │ │
   │   │  Memory-mapped I/O hooks invoke the layers above   │ │
   │   └────────────────────────────────────────────────────┘ │
   └──────────────────────────────────────────────────────────┘
```

Two follow-up specs flesh out the layers:

- **`apple-ii-machine.md`** (follow-up; not in this PR) — the memory
  map, soft switches, video pages, keyboard, speaker, paddles,
  language card. The hardware that VisiCalc directly touches and a
  few extras for completeness. Targets: enough fidelity that the
  VisiCalc binary boots, accepts keystrokes, and writes the screen
  buffer.
- **`apple-ii-disk.md`** (follow-up) — Disk II controller, GCR
  encoding, .dsk/.nib formats, RWTS routines from DOS 3.3. Targets:
  enough fidelity that the VisiCalc disk boots, that VisiCalc can
  load and save sheets to a writable disk image.

This spec is a top-level overview that sequences the work, defines
the success criteria, and pins down the boundaries. The detailed
hardware specs follow.

---

## §1 Success Criteria

The faithful track is *done* when:

1. The original 1979 VisiCalc binary (the 27 KB image hosted on
   bricklin.com, MD5 verified) boots on the emulator from a `.dsk`
   image we can construct from the binary plus a DOS 3.3 boot loader.
2. The UI grid renders correctly in both terminal mode (40×24 text
   characters, no color) and an optional GUI window with a CRT-like
   font.
3. Keyboard input is accepted: cursor movement (`I/J/K/M`), command
   leader (`/`), function entry (`@SUM`, `@AVERAGE`, etc.), all
   produce the same screen state as the original.
4. Spreadsheets can be saved to and loaded from disk images. Round-tripping
   a saved sheet through emulator → disk image → emulator returns the
   same display.
5. The full VisiCalc reference card's example session runs to
   completion without divergence.
6. We can play the 1979 demo sheet bundled on the original disk.

Success is *not* measured by:

- General Apple II compatibility (other programs may or may not work)
- Performance matching real hardware (we don't need real-time)
- Disk II copy protection workarounds (we use the ungated VisiCalc
  binary; protected versions are out of scope)
- Color, hires graphics, mouse, mockingboard sound (VisiCalc uses
  none of these)

---

## §2 What VisiCalc Touches

A 6502 trace of VisiCalc's first second of execution shows what the
machine layer actually has to provide. The list is short:

| Hardware                  | Why VisiCalc needs it                              |
|---------------------------|----------------------------------------------------|
| 64 KB RAM (with language card for the upper 16 KB) | Code + sheet data |
| Text page at `$0400-$07FF` | The 40×24 character display |
| Keyboard at `$C000-$C001`  | Reading keystrokes |
| Speaker at `$C030`         | Cell-overflow beep |
| Disk II controller         | Loading the binary at boot, save/load sheets |
| ROM monitor (`$F800-$FFFF`) | Boot path; VisiCalc itself does not use monitor calls beyond startup |
| DOS 3.3 RWTS               | File operations (GET FILE, SAVE FILE) |

Things VisiCalc does **not** touch:

- Hires pages
- Lores pages (uses text only)
- Paddles, joysticks, mouse
- Cassette interface
- Auxiliary slots beyond Disk II

This is the entire reason the faithful track is feasible: the
hardware surface is small. We do not need a complete Apple II
emulator. We need the slice VisiCalc uses.

---

## §3 Boot Sequence

The Apple II boot ROM at `$FF00` runs at power-on, looks for a Disk
II controller in slot 6, jumps to its boot ROM. Disk II's boot ROM
loads track 0 sector 0 from the disk into `$0800`, jumps to it. Track
0 sector 0 is the DOS 3.3 boot loader, which loads DOS into upper
memory and chain-loads the binary specified in `HELLO` (the boot
program).

For VisiCalc:

1. Boot disk track 0: DOS 3.3 boot loader
2. DOS 3.3 loads, finds `HELLO` (the boot program), executes it
3. `HELLO` is a small BASIC program that does `BRUN VISICALC`
4. `BRUN` loads VisiCalc into `$1000-$76FF` and jumps to `$1000`
5. VisiCalc takes over

The emulator's job is to provide enough of (a) the boot ROM, (b) the
Disk II controller, (c) DOS 3.3, and (d) the file system to let
this sequence run. We do not need to handle the case where the user
types `]CATALOG` from the BASIC prompt or runs other programs; the
boot disk is purpose-built to launch VisiCalc.

A simplification we adopt: instead of constructing a faithful DOS 3.3
disk image and running its full boot loader, we provide a *mock boot*
that loads the VisiCalc binary directly into memory at `$1000` and
jumps there. This shortcuts the multi-stage boot for development but
sacrifices fidelity. A `--full-boot` flag enables the real boot path
once `apple-ii-disk.md` is implemented, for users who want the
authentic load-from-disk experience.

---

## §4 Display Backends

The text page maps directly to a 40×24 character grid. Each cell is
a byte at `$0400 + offset`, where the offset is computed by the Apple
II's notorious non-linear screen layout (rows are interleaved in
groups of 8 to support hires-mode hardware addressing). The
character byte's high bit indicates inverse video; low 7 bits are
the (modified) ASCII code.

Two backends:

### Terminal backend

Renders to stdout using ANSI escape codes for cursor positioning and
inverse video. Each frame is a `40×24` block written to a fixed
position. Refresh on every change to `$0400-$07FF`. The terminal
must support at least 40×24; we recommend 80×24 with 40 columns of
padding.

### GUI backend

Renders into a window using a CRT-style font (the Apple II's
character ROM dumped to a TrueType-equivalent). Optional CRT
shader for the period feel. The GUI backend is in a separate Python
package depending on `pygame` or similar, kept optional so the
terminal backend works on any Python install.

Both backends watch the same memory range and re-render on dirty.
Refresh rate caps at 60 Hz to avoid thrashing.

### No graphics

Hires-page rendering is out of scope. VisiCalc never draws to hires.

---

## §5 Keyboard

The Apple II keyboard appears at `$C000` (read for the current key
+ strobe bit) and `$C010` (write to clear the strobe). The mapping
is a near-ASCII subset with modifications:

- Letters arrive as uppercase only (no shift state)
- Control characters from `Ctrl`-modified keys
- Arrow keys do not exist on the original Apple II — VisiCalc uses
  `I` (up), `J` (left), `K` (right), `M` (down) for navigation
- The `Esc` key sends `$1B` (standard ASCII)
- The `Return` key sends `$0D`

The emulator polls the Python frontend's input source (terminal stdin
in non-canonical mode, or the GUI window's events) and queues
keystrokes. The 6502 sees one byte at `$C000` with the high bit set
when a key is pending; reading `$C010` clears it.

Key-repeat behavior: the original Apple II had a hardware repeat
after a held delay. VisiCalc relies on this for cursor movement.
The emulator's input driver implements key-repeat with the same
~250 ms delay, ~10 Hz repeat rate.

---

## §6 Speaker

The speaker at `$C030` toggles every read. VisiCalc beeps on cell
overflow and on errors. The emulator's audio backend is optional; on
machines without audio output, beeps are dropped silently.

---

## §7 Disk and File System

VisiCalc saves sheets as a custom binary format on the boot disk.
The format is documented in the VisiCalc reference materials and is
small (header + cell array). The emulator's disk layer presents a
writable `.dsk` image to VisiCalc's RWTS calls.

For development, the recommended workflow:

1. Start with a read-only boot disk containing the VisiCalc binary
2. Mount a separate writable data disk in slot 6 drive 2 (or use a
   single-drive disk swap)
3. SAVE FILES to the data disk
4. Inspect the saved sheets via a host-side tool that reads the
   VisiCalc file format

The host-side tool lives at `code/programs/python/visicalc-file-reader/`
and is a thin wrapper around the documented format. It is useful for
verification and round-trip testing but is not part of the emulator
itself.

---

## §8 Testing

Three test layers:

1. **Per-component tests** (in each Python package's own tests/):
   memory map decoding, soft-switch behavior, keyboard queue, screen
   layout decoding, GCR encoding round-trip.
2. **Integration tests** (in `code/programs/python/visicalc-faithful-tests/`):
   boot the binary, drive a scripted keystroke sequence, assert the
   final screen state. The keystroke sequences come from a corpus of
   VisiCalc tutorial examples.
3. **Differential testing** against AppleWin (or another mature Apple
   II emulator): same keystroke sequence, compare final screen
   state. Useful for catching subtle hardware-emulation bugs we
   missed.

Testing notes:

- The corpus includes the demo sheet from the original 1979
  distribution, recreated by hand from the documentation.
- Cycle-accurate timing is asserted only at the unit-test level for
  the simulator itself; the integration tests do not depend on
  cycle counts.

---

## §9 Implementation Phases

Each phase is its own follow-up implementation PR:

1. **Phase 1 — `apple-ii-machine.md` spec + skeleton** (Python crate
   `apple-ii-machine`). Spec the memory map, soft switches, write
   the Python module that wires `mos6502-simulator`'s memory-access
   hooks to soft-switch handlers. Boot a tiny test program (not
   VisiCalc yet).
2. **Phase 2 — Display backend.** Terminal first, GUI later. Watch
   `$0400-$07FF`, render the screen.
3. **Phase 3 — Keyboard.** Terminal stdin in non-canonical mode +
   GUI events.
4. **Phase 4 — `apple-ii-disk.md` spec + Disk II + DOS 3.3 RWTS.**
   Full disk image support. Boot a real DOS 3.3 disk.
5. **Phase 5 — VisiCalc boot.** Construct the boot disk, run the
   binary, watch it draw the splash screen.
6. **Phase 6 — End-to-end demo.** Full keystroke-driven demo sheet.

Phases 1-3 are useful even without Phases 4-6: they give us a working
Apple II text-mode emulator that can run BASIC and small assembly
programs from memory dumps. Phases 4-6 add the disk subsystem to
support the original VisiCalc binary specifically.

---

## §10 Out of Scope

- Color (VisiCalc is monochrome)
- Hires / lores graphics
- Mouse, paddles, joysticks
- Auxiliary cards beyond Disk II
- Other 1979-era spreadsheets (SuperCalc, Multiplan early version,
  context MBA) — those would each need their own boot ROM and
  language environment
- The Lotus 1-2-3 / Symphony / Excel reconstruction lineage — those
  ran on PCs, not Apple IIs, and have their own emulation
  requirements (8086, MS-DOS, EGA/VGA)
- Period-correct printer output (Imagewriter, Silentype)
- Networking (none in 1979)

---

## References

- Sather, *Understanding the Apple II* (1983) — definitive hardware
  reference
- Bricklin & Frankston interview on VisiCalc internals (Computer
  History Museum oral history)
- DOS 3.3 source listing (Apple Computer, 1980)
- Bricklin's site, https://www.bricklin.com/history/intro.htm — hosts
  the original VisiCalc binary and historical materials
- VisiCalc Reference Card (1979)
- The existing `mos6502-simulator` and its spec
  `code/specs/07j-mos6502-simulator.md`
