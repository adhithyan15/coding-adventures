- Generalize the shared subresource scheduler with typed stylesheet/image
  requests and completions. External author sheets now retain parser-defined
  order, block later rules until settled, honor screen/all media, survive
  fetch/parse failures, cancel on navigation, and incrementally restyle the
  retained page through the shared computed cascade.
