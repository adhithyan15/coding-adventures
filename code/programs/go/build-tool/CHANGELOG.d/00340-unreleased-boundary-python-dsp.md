## Unreleased - source-input boundary sync (python dsp packages)

- The graph-diff conformance test pins the repository source-input boundary digest; it moved from
  `3f8aff75...` to `43a5ed7f...` because `dsp-complex`, `dsp-dct`, `dsp-fft`, and `dsp-filters` were added
  to the Python uv workspace manifest (#19437). Test pin only; no tool behaviour change.
