### Added — `mosaic-compile --backend xaml` CLI wiring

- `mosaic-compile --interface X.mil --layout X.mll --style X.msl
  --backend xaml [-o BASE]` now compiles a three-file Mosaic
  pipeline triple to a WinUI 3 component triple.
- The `--backend` validation list grew `xaml`.
- `run_pipeline` branches on backend: `react` emits one `.tsx`
  file (unchanged from before); `xaml` emits the triple
  (`{base}.xaml`, `{base}.xaml.cs`, `{base}.Event.cs`) plus
  zero-or-more RowVm `.cs` files. `BASE` is treated as a file-name
  prefix; the default is the component name. A trailing `.xaml` in
  `BASE` is stripped so `Grid.xaml` produces three sensibly-named
  files instead of `Grid.xaml.xaml.cs` etc.
- Three new prints (`Written: ...`) per invocation in xaml mode.
- `mosaic-compile`'s `Cargo.toml` now depends on `mosaic-emit-xaml`.

