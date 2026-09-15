### Shared .NET build-tool discovery

- Aligned the shared C# engine with the complete language-neutral discovery
  registry: only the exact bucket below `packages` or `programs` selects a
  language, domain-language buckets remain classified, BUILD roots outside
  those containers stay out of the package graph, and generated
  `dist-newstyle` and exact case-sensitive `_build` trees are pruned.
- Added independent C# and F# fixture evidence, including a native F# discovery
  facade, while preserving `_Build` and `_build-example` source directories.

