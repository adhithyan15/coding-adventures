### Changed

- Made each curriculum path shard's `spine_node` the single owner of its spine
  membership. The 920 duplicate reverse `segments` arrays are gone; loaders
  derive the unchanged public graph in path order and reject their resurrection.
