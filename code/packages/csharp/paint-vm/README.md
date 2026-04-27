# paint-vm

Pure C# dispatch-table VM for executing `paint-instructions` scenes.

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
- Structural `DeepEqual` support for scene diffing

## Usage

```csharp
using CodingAdventures.PaintVm;
using static CodingAdventures.PaintInstructions.PaintInstructions;

var vm = new PaintVM<List<string>>((log, background, _, _) => log.Add($"clear:{background}"));
vm.Register("rect", (instruction, log, _) => log.Add(instruction.Kind));

var scene = PaintScene(100, 40, "#fff", [PaintRect(0, 0, 10, 10)]);
var log = new List<string>();

vm.Execute(scene, log);
```

## Development

```bash
bash BUILD
```
