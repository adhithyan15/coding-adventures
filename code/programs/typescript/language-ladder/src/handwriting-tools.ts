// One lazy browser entry for the handwriting-only model, renderer, and font
// parser. Keeping this as an ordinary dynamic entry lets the bundler leave
// script inventory JSON shared with the eager shell in its existing
// `script-data` chunks instead of pulling that shared data into a forced manual
// group.
export {
  ductusFilmstrip,
  ductusFor,
  isSafeName,
} from "@coding-adventures/script-ductus/src/ductusview.ts";

export {
  boundsOf,
  parseFont,
} from "@coding-adventures/script-ductus/src/truetype.ts";
