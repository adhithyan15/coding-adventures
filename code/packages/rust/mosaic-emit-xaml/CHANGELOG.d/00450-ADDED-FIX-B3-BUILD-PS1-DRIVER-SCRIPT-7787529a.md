### Added — Fix B3: `build.ps1` driver script

Cleans bin/obj with `-Clean`, builds with `dotnet build -c Debug
-p:Platform=x64 --nologo` (Platform=x64 required because
WindowsAppSDK refuses AnyCPU), and with `-Run` launches the .exe.

