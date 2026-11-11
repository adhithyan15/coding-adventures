# Changelog — perl-bridge

## Unreleased

- Show the C compiler's own output when the Windows shim compile fails
  (`ExtUtils::CBuilder` `quiet=>0`). The first Windows full build failed
  `perl/conduit` and `perl/irc-server-native` here with only "error building
  xs.dll", which hid the real diagnostic.
- Release callback error strings through the same C shim that allocated them.
  This fixes a Windows allocator mismatch when a Perl handler throws: Conduit
  must return its `on_error` response and keep serving later requests.
- Build and archive the C shim on native Windows using the selected Strawberry
  Perl's compiler configuration, instead of skipping the shim entirely.
- Propagate the exact CORE import library to Rust consumers, including MSVC
  cdylibs; validate the interpreter's platform, pointer/IV width and NV size.
- On Windows, prefer the Perl on PATH over mise unless PERL_BIN is explicit,
  keeping compilation aligned with the BUILD recipe's loading interpreter.
