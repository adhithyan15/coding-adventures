- Add a shared Open in New Window chrome transaction that resolves the
  committed history URL and emits a `_blank` GET browsing-context request with
  `noopener`, without mutating current history or consulting the address draft.
