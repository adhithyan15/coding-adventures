# Mosaic.Flux

Strict-Flux runtime for the Mosaic UI WinUI / XAML emitter.

This is the C# counterpart to `@coding-adventures/mosaic-flux-react`,
`mosaic-flux-swiftui`, `mosaic-flux-compose`, and `mosaic-flux-flutter`.
Every Mosaic backend ships its own runtime so application code never
depends on a third-party Flux library — TCA, Bloc, Fluxor, etc. are
deliberately avoided. The shape of the API is identical across
backends so cross-platform Mosaic projects only learn it once.

## What's in the box

| Type | Role |
| --- | --- |
| `IMosaicAction<TState>` | command-pattern action with an `Apply(state) → state` method |
| `MosaicStore<TState>` | dispatcher + state holder + `INotifyPropertyChanged` for XAML binding |
| `Middleware<TState>` | logger / dev-tools hook between every dispatch |
| `Selector` | memoised projection helpers (1–3 input slices) |
| `DevTools` | local channel for inspecting actions and state diffs |

## Why strict Flux

Mosaic UI takes the Redux contract literally:

1. The view layer is **read-only**. Nothing in a `.msl` file mutates
   state directly.
2. Every change goes through `Store.Dispatch`. Even a single keystroke
   in a VisiCalc cell rounds back through the store before the view
   re-renders.
3. Actions are **classes** (records) with an explicit `Apply(state)`
   method. If a backend needs special handling for an action — for
   example, debouncing a network call from XAML — you edit the action
   class, not a hidden reducer table.

That last bullet is why this exists rather than reusing Fluxor or
ReactiveUI: Mosaic wants the generated action surface to be exactly
the thing the user edits.

## Usage

```csharp
using Mosaic.Flux;

public record AppState(int Count = 0);

public sealed record Increment : IMosaicAction<AppState>
{
    public AppState Apply(AppState state) => state with { Count = state.Count + 1 };
}

var store = new MosaicStore<AppState>(new AppState());
store.Subscribe(s => s.Count, count => Console.WriteLine($"count = {count}"));
store.Dispatch(new Increment());  // → "count = 1"
```

In a WinUI page, bind to `store.State` and `PropertyChanged` will fire
the XAML data-binding pipeline whenever an action changes the slice
your control reads.

## Status

`v0.1.0` — core dispatcher, subscription, selector, middleware, and
dev-tools surface. The WinUI-specific glue (XAML markup extension,
`MosaicBinding`, page lifecycle hooks) lands in `v0.2.0`.
