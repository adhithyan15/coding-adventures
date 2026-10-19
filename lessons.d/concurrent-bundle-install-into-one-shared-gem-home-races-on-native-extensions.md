---
category: Ruby
---

# Concurrent bundle install into one shared gem home races on native extensions

The build tool runs several packages at once, and every Ruby BUILD starts with
`bundle install`. On a CI runner they all install into the runner Ruby's one
gem home (`/opt/hostedtoolcache/Ruby/<version>/x64/lib/ruby/gems`). RubyGems
takes no lock across processes. When two packages both needed a gem that
nobody had installed yet (json-3.1.0), both compiled its native extension in
the same `ext/` directory. One failed with
`Gem::Ext::BuildError ... rb_sysopen - mkmftmp1.log (Errno::ENOENT)`,
because the other process had deleted mkmf's temp log out from under it.

The race had been latent all along. It never appeared in 71 earlier main shard
logs. It showed up as soon as locality-aware sharding packed Ruby packages
together, so that two of them started at the same moment on a shard whose gem
home was still cold.

The fix is the executor's existing remedy for shared package-manager state
(Hex, Cabal, LuaRocks): a `global:ruby-gems` resource lock for every BUILD
that runs `bundle install` or `gem install`.

When you see a native-extension build failing with a missing temp or log file
inside the gem directory, suspect a concurrent install before you suspect the
toolchain.
