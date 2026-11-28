# NuGet package readme metadata needs a package-root path

### The symptom

Setting `PackageReadmeFile` alone does not ensure a README is packed. Adding a
`None Update` with an invalid package path can also make `dotnet pack` reject
the metadata as an invalid URI.

### The fix

Use `<PackageReadmeFile>README.md</PackageReadmeFile>` and update the SDK's
existing item with `<None Update="README.md" Pack="true" PackagePath="/" />`.
Verify the resulting `.nupkg` lists `README.md` at its root.
