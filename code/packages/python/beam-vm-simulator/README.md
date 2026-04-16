# beam-vm-simulator

An initial Python BEAM VM simulator built as the final stage of a reusable
pipeline:

`beam-opcode-metadata` -> `beam-bytes-decoder` -> `beam-bytecode-disassembler` -> `beam-vm-simulator`

This first slice intentionally implements a small executable subset so we can
verify the package boundaries while keeping the runtime reusable.
