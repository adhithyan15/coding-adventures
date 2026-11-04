## Unreleased — Windows native Perl bridge

- Build and run through the shared native-Perl helper, including when launched
  from Git Bash with MSYS Perl first on PATH.
- Delegate Windows shim compilation and the exact CORE import library to
  `perl-bridge`, replacing interpreter-library directory guessing. The release
  DLL is verified by real loopback IRC clients through Strawberry Perl.
