# paint-vm

Pure F# dispatch-table VM for executing `paint-instructions` scenes.

`paint-vm` is the bridge between the declarative paint IR and a concrete
backend. A renderer registers handlers for instruction kinds, and the VM
provides ordered execution, scene patching, and optional pixel export.

## Dependencies

- paint-instructions

## What It Provides

- Per-kind handler registration with duplicate-kind protection
- Ordered scene execution with a backend-provided clear function
- Patch callbacks for delete, insert, and update detection
- Optional export hook for backends that can return a `PixelContainer`
- Structural equality helpers for scene diffing

## Usage

```fsharp
open System.Collections.Generic
open CodingAdventures.PaintInstructions
open CodingAdventures.PaintVm

let vm = PaintVM<List<string>>(fun log background _ _ -> log.Add($"clear:{background}"))
vm.Register("rect", fun instruction log _ -> log.Add(instruction.Kind))

let scene = PaintInstructions.paintScene 100 40 "#fff" [ PaintInstructions.paintRect 0 0 10 10 ]
let log = List<string>()

vm.Execute(scene, log)
```

## Development

```bash
bash BUILD
```
