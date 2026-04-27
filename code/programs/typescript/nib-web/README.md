# nib-web

Interactive browser playground for the Nib language.

It compiles Nib source code to Intel 4004 assembly, binary, and Intel HEX
using the real TypeScript toolchain from this repository, then runs the
generated program inside the TypeScript Intel 4004 simulator.

The UI is styled with Lattice and compiled through the TypeScript
`vite-plugin-lattice` pipeline during Vite dev/build.
