// SIMPLE-level local renaming (CLOC12.160).
//
// The SIMPLE pipeline now ends with `rename`, which shortens the
// parameters of leaf functions (functions with no nested function) to
// short names. The function NAME `distance` is top-level and may be
// referenced externally, so it is kept; only its parameters
// `horizontal` and `vertical` are renamed (to `a` and `b`).
//
// `distance(3, 4)` calls it, so treeshake keeps the declaration; an
// uncalled function would be removed before rename ever saw it.
//
// Under WHITESPACE_ONLY nothing is renamed.
function distance(horizontal, vertical) {
  return horizontal * horizontal + vertical * vertical;
}
distance(3, 4);
