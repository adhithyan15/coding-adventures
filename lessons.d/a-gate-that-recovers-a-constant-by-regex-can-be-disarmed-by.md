# A gate that recovers a constant by regex can be disarmed by a comment

Replacing a hardcoded CI ceiling with one **derived** from the corpus was the right move and
almost shipped with the same class of hole it was removing.

The bundler config declared the band width; the checker recovered it with
`/export const LESSON_BAND_CHAPTERS = (\d+);/.exec(configSource)`. `exec` returns the
**first** match anywhere in the file — and that file carries eighty lines of prose that
discuss the constant by name, because a number nobody can explain is a number the next
person bumps. So a line as innocent as

    // historical note: this was `export const LESSON_BAND_CHAPTERS = 1;` before

hands the checker a band width of 1 while the bundler goes on using 5. Smaller bands mean
*more* bands, so the derived budget inflated from 281 to **1,158**, and a grouping
regression all the way back to the byte-linear shape the change existed to kill would have
passed unremarked. Documenting the constant well was what armed the attack.

**The fix is not a better regex** — `^`-anchoring or "last match wins" both lose to a
slightly different comment. Put the value in a module and `import` it from both sides. An
import cannot be shadowed by a comment, and the two consumers stop being two
implementations that merely look alike.

**Generalisable check:** any time a checker recovers a value by *parsing the source of the
thing it checks*, ask what happens when that source also *talks about* the value. Config
files, migration scripts and lockfile linters all do this. If both sides can import, they
must.

**Corollary — for a CI gate, ask which direction an error moves it.** Every guard on that
corpus walk pointed the same way once the question was framed: a symlinked `lessons/`
directory, an unbounded digit run reaching `Infinity`, a track name Rollup's sanitiser
mangles — each *invents* a band, each *raises* the budget, each makes the gate pass when it
should fail. None of them threatened the build. A gate's own permissiveness is the only bug
class it cannot catch for you.
