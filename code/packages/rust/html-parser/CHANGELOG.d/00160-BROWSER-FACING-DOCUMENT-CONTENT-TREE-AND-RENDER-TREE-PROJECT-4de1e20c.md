- Browser-facing document, content-tree, and render-tree projections now carry
  resolved URL metadata for links, resources, images, and form actions using
  the document `base` href when available, while preserving raw authored
  attributes for downstream policy decisions.
