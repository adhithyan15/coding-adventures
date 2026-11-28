---
category: BUILD files & dependency management
---

# OCaml Dune aliases must reach the CLI as literal arguments

because a bare `@fmt` in a PowerShell command can be interpreted as splatting.
The command may exit successfully while Dune builds only its default target,
leaving formatter drift for CI to catch. Quote the alias as `'@fmt'` in
PowerShell, or invoke the command through Bash as CI does. Confirm the command
output includes Dune's recursive `@fmt` target, and test an isolated package
copy when CI builds from a copied package.
