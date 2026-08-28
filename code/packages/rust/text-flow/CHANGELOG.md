# Changelog

## 0.2.0

- Replaced handwritten grapheme, bidi, and line-break classifiers with
  generated Unicode conformance data while preserving the analyzer API.
- Added bidi isolate/embedding resolution, the Unicode 17 full line-break
  state machine, and dictionary segmentation for Thai, Lao, Khmer, and
  Myanmar.
- Added a public conformance profile for diagnostics and focused acceptance
  for controls, CJK punctuation, and complex-script boundaries.

## 0.1.0

- Added host-neutral grapheme, bidi-run, and line-break analysis with UTF-8
  source ranges and selection-boundary snapping.
