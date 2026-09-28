- The conformance coverage audit now reads tree-construction cases from WPT and
  tokenizer cases from html5lib-tests, pins every missing WPT source signature,
  and accepts only explicit missing-case debt counts so the now-zero gap cannot
  silently regress as upstream evolves.
