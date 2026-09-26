- **Why `{` and `}`:** this closes a pre-existing hole. A loop item containing
  `{{{key}}}` was expanded again by the outer mustache pass, through the
  unescaped triple-mustache path, so any host string could inject markup.
