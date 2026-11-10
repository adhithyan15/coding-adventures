---
category: CI & GitHub Actions
---

# Ubuntu 26.04 split ld.gold out of binutils, so GHC (-fuse-ld=gold) cannot link until binutils-gold is installed

`ubuntu-latest` now resolves to Ubuntu 26.04 (runner image `ubuntu-26.04`). On
22.04 and 24.04 `ld.gold` ships inside `binutils` and `binutils-gold` is a
virtual package; on 26.04 it is a separate real package that is not installed by
default. GHC 9.4.8 links with `-fuse-ld=gold`, so anything that actually links
failed in every `main` full build:

```
command was: /usr/bin/gcc ... -fuse-ld=gold ...
error: collect2: fatal error: cannot find 'ld'
```

It showed up only for `haskell/conduit` (cabal building `network` through
hsc2hs, plus an FFI library). The other Haskell packages here are tested with
`runghc`, which interprets and never invokes the linker, so a missing linker
looked like a problem with one package instead of with the image. The failure had
been red for days before anyone attributed it to the runner.

What to do differently:

- When one package in a language fails with a toolchain-shaped error ("cannot find
  ld", missing header, missing binary) and its siblings build, check what is
  different about *that* package (here: it links) and what the runner image
  changed, before debugging the package.
- A `-latest` runner label moves under you. The runner log's `Image:` line
  (`Image: ubuntu-26.04`) and the annotation about the migration date say so.
- apt steps must go through `code/scripts/ci/apt-install.sh` and carry their own
  `timeout-minutes` (`test_apt_install.py` enforces both).
