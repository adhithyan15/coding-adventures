---
category: Native extensions & FFI
---

# Windows Perl extensions need the bridge C shim and the interpreter CORE import library

PR #18844 made the native Perl Windows recipes executable and exposed unresolved
`perl_bridge_*` symbols in both Conduit and the IRC extension. The shared bridge
returned early on Windows, so none of its C shim was compiled. Consumer scripts
also searched Perl's lib directory instead of Config's CORE import library.

Build the shim with the same native interpreter that loads the extension, and
propagate its exact import library from the shared bridge. Check ABI widths
before compiling and run real Perl tests after staging both DLLs. Merely making
cmd syntax valid or successfully linking Rust does not prove XS works.

The real Windows Conduit test then crashed on its dying-handler case. `XSUB.h`
remaps malloc/free to PerlMem_malloc/PerlMem_free, but Rust released the returned
error string through its CRT free. Move deallocation into the same C shim.
The existing exception-to-500 test followed by another request checks both
error delivery and server survival.

Windows Perl emulates fork; fork followed by exec can leave a server child alive
after terminating the pseudo-process. Launch the server with native asynchronous
system and clean up its actual PID. Run dependent Windows DLL staging checks
sequentially: copying a staged DLL while another suite has loaded it fails.
Git Bash also puts its MSYS Perl ahead of native Strawberry in inherited PATH.
Select native Perl once for Cargo, staging and prove; choosing it only in the
Rust build script leaves the test loader on the wrong interpreter.
