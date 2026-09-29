### Fixed - Generated WinUI SDK selection

Complete project emission now includes a `global.json` that selects the .NET 9
SDK family targeted by the generated project. The generated build script also
builds from the project directory, so machines with .NET 10 installed globally
do not accidentally run the Windows App SDK 1.7 XAML compiler under an
unsupported newer SDK toolchain.

