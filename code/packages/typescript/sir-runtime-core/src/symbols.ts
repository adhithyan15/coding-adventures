/**
 * Interned symbols — the SIR `Sym` value type.
 *
 * A Lisp/Ruby *symbol* (`:foo`) is an interned, immutable name with
 * **identity** semantics: two symbols with the same text are the *same*
 * object. JavaScript's nearest native thing, a string, has value
 * semantics and no distinct identity, so symbols are a genuine SIR quirk
 * that lives here in the runtime library rather than being faked inline.
 *
 *     intern("a") === intern("a")   // true  (same text -> same object)
 *     intern("a") === intern("b")   // false
 */

/** An interned symbol. Construct via {@link intern}, not `new Sym`. */
export class Sym {
  constructor(public readonly name: string) {}

  toString(): string {
    return this.name;
  }
}

const table = new Map<string, Sym>();

/** Return the canonical {@link Sym} for `name`, creating it on first sight. */
export function intern(name: string): Sym {
  let existing = table.get(name);
  if (existing === undefined) {
    existing = new Sym(name);
    table.set(name, existing);
  }
  return existing;
}
