# Exclude nested test sources from C# library projects

The generated C# DER ASN.1 library project used SDK default globbing, so its
nested xUnit sources were compiled into the production assembly and failed
before the test project could build. Every C# package that keeps tests under
the package root must explicitly remove `tests/**/*.cs` from the library
compile inputs before the first test-first compile.
