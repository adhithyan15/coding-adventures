# assembler (C)

An ARM assembly parser and binary encoder, in **pure ISO C17**. A faithful port
of the Rust [`assembler`](../../rust/assembler) crate.

## What it does

Parses a subset of ARM assembly source text into structured instructions, then
encodes each into its 32-bit ARM machine-code word.

Supported mnemonics: `MOV(S)`, `ADD(S)`, `SUB(S)`, `AND(S)`, `ORR(S)`,
`EOR(S)`, `RSB(S)`, `CMP`, `LDR`, `STR`, `NOP`, and labels (`name:`).
Registers accept `R0`–`R15`, `SP`, `LR`, `PC` (case-insensitive); immediates are
`#42` or `#0xFF`; comments start with `;` or `//`.

## API

- `asm_init` / `asm_free` — an `Assembler` holding the label table.
- `asm_parse(asmr, source, &out, &out_len, err)` — parse to a malloc'd
  `ArmInstruction` array (free with `asm_instructions_free`); labels are
  recorded in `asmr`.
- `asm_label_lookup(asmr, name, &addr)` — resolve a label's address.
- `asm_encode(instrs, n, &words, &out_len)` — encode to a malloc'd `uint32_t`
  array (labels emit nothing; free with `free`).
- The `AsmError` out-parameter carries the code and a message reproducing the
  Rust `Display` text (e.g. `"Unknown mnemonic: BLAH"`).

## Design notes

- **Status codes, not `Result`.** Rust's `AssemblerError(String)` becomes an
  `AsmStatus` code plus an optional `AsmError` out-parameter; results
  (`Vec<ArmInstruction>` / `Vec<u32>`) become malloc'd arrays the caller frees.
- **Ownership.** Only `ASM_INSTR_LABEL` owns a heap string;
  `asm_instructions_free` releases them. Growable arrays and the label table are
  overflow-guarded. The parser copies each line into a scratch buffer for
  in-place tokenising, so the source is never mutated.

## Usage

```c
#include "assembler.h"

Assembler a; asm_init(&a);
ArmInstruction *ins = NULL; size_t n = 0; AsmError err;
if (asm_parse(&a, "MOV R0, #42\nADD R2, R0, R1", &ins, &n, &err) == ASM_OK) {
    uint32_t *words = NULL; size_t w = 0;
    asm_encode(ins, n, &words, &w);   /* words[0] == 0xE3A0002A */
    free(words);
    asm_instructions_free(ins, n);
}
asm_free(&a);
```

## Building

```sh
sh BUILD           # POSIX: GCC and/or Clang via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
