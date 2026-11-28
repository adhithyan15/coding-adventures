# NuGet package readme metadata needs a package-root path

### The symptom

Setting `PackageReadmeFile` alone does not ensure a README is packed. A
`None Update` only modifies an item already created by SDK defaults; some F#
projects have no default README item, so that update silently packs nothing.
An invalid package path can also make `dotnet pack` reject the metadata as an
invalid URI.

### The fix

Use `<PackageReadmeFile>README.md</PackageReadmeFile>` and declare the pack item
with `<None Include="README.md" Pack="true" PackagePath="/" />` when the
project has no default `None` item. Use `Update` only when an item is already
present. Verify the resulting `.nupkg` lists `README.md` at its root.
