import { describe, expect, it } from "vitest";
import { realCorpusTargets as targets } from "./real-corpus-fixture.js";

describe("Gujarati real-corpus figure targets", () => {
  it("draws every Gujarati sign lesson, and every Gujarati word or list whose signs are cited", () => {
    // Ten lessons teach a sign by itself, and thirteen word lessons (eleven
    // words, and હા twice) carry one: 23 in all. The six lists that carry one
    // (GU-R04 … GU-R19) and હા's dictation are writing from sound, so they
    // print no strip (no-strip-above-a-dictation). All are drawn
    // consonant first, sign after: KanoAI's barakhadi templates write every
    // sign after its consonant, even િ. GU-W21-ai-matra and the seven place
    // words of chapters 20-23 (મંદિર … ગામ) have no Writing block: their strip
    // lands in their guided- or delayed-copy practice (figure-targets.ts).
    const gujarati = targets.filter((target) => target.lessonId.startsWith("GU-"));
    const withSign = gujarati.filter((target) =>
      (target.letters ?? [target.glyph]).some((piece) => /^\p{M}+$/u.test(piece)),
    );
    expect(
      Object.fromEntries(
        withSign.map((target) => [target.lessonId, (target.letters ?? [target.glyph]).join(" ")]),
      ),
    ).toEqual({
      "GU-C20-mandir": "મ ં દ િ ર",
      "GU-C21-haath": "હ ા થ",
      "GU-C21-paisa": "પ ૈ સ ા",
      "GU-C22-shaalaa": "શ ા ળ ા",
      "GU-C22-shahar": "શ હ ે ર",
      "GU-C23-dukaan": "દ ુ ક ા ન",
      "GU-C23-gaam": "ગ ા મ",
      "GU-C32-ane-write": "અ ન ે",
      "GU-C33-ke-write": "ક ે",
      "GU-C34-kemke-write": "ક ે મ ક ે",
      "GU-C36-te-write": "ત ે",
      "GU-W01-aa-matra": "ા",
      "GU-W01-e-matra": "ે",
      "GU-W01-haa-delayed-copy": "હ ા",
      "GU-W01-haa-guided-copy": "હ ા",
      "GU-W03-anusvara": "ં",
      "GU-W03-ii-matra": "ી",
      "GU-W03-o-matra": "ો",
      "GU-W03-u-matra": "ુ",
      "GU-W04-i-matra": "િ",
      "GU-W07-uu-matra": "ૂ",
      "GU-W21-ai-matra": "ૈ",
      "GU-W45-ai-sign": "ૈ",
    });
    // The virama ્ and the vocalic-r sign ૃ have no Gujarati source, so their
    // own lessons and every word that carries one stay undrawn. જો is refused
    // because the bundled font joins the ā bar of ો to જ.
    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of [
      "GU-W01-virama",
      "GU-W02-vocalic-r",
      "GU-W01-namaste-read",
      "GU-C37-kyaan-write",
      "GU-C35-jo-write",
      "GU-R23-route-three-r1",
      "GU-R23-shaalaa-rasto-r2",
    ]) {
      expect(lessonIds.has(id), id).toBe(false);
    }
  });

});
