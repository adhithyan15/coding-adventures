# PowerShell validation chains must stop before commit on failure

A semicolon-separated validation-and-commit command ran from the repository root,
so `npm run build` targeted a nonexistent root `package.json`, the coverage command
could not see the package-local dev dependency, and later commands still committed
the change even though validation and lesson checks had failed. Run package-native
commands from the package directory and join validation to staging/commit with `&&`
so a failed gate prevents publication. Always inspect the generated lesson shard and
remove its placeholder before invoking `lessons.py validate`.
