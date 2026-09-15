# Replacing a shell tool with a library silently drops the flags it was given

Fixing the above, I swapped `zip -qry` for Python's `zipfile`. I reasoned about
the tool's *availability* and not about its *behaviour*, and the `-y` in that
command is load-bearing: it means **store symbolic links as links**.
`ZipFile.write` does the opposite — it opens the path, so a symlink is archived
as its target's bytes under the link's own innocuous name. Demonstrated on a
fixture:

```
ZipFile.write   sub/innocent.txt -> b'SECRET-TOKEN\n'     # the target's content
zip -qry        sub/innocent.txt -> b'../../secret.txt'   # the link
```

These archives are published as public release assets. Anything able to place
one symlink into the build tree — a committed file under a Gradle-copied
resources directory, any plugin running during `createDistributable` — could
have had the contents of any runner-readable file baked into a download. On a
GitHub runner `.git/config` holds the checkout token as an `http.extraheader`.
The member name looks entirely ordinary, and it needs no outbound network.

The same mistake broke the payload in the other direction, which is what makes
it worth remembering: `rglob` does **not** descend into a symlinked directory,
and a symlinked directory is not `is_file()`, so following links also *drops*
every file beneath one while reporting success. A macOS `.app` from jpackage
bundles a runtime full of symlinked directories, so the fix that leaked secrets
would also have shipped a gutted, unsignable bundle.

Two things generalise:

- **When you replace a command with a library, enumerate its flags and ask what
  each one was doing.** `-y` was one character and the entire difference
  between storing a link and publishing its target.
- **The security bug and the correctness bug were the same bug.** Following the
  link leaked a file *and* lost a directory tree. When a security review lands
  a finding, check whether the safe behaviour is also the correct one — here,
  storing links was required by both, so there was no trade-off to weigh.
