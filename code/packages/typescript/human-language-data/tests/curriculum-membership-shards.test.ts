import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  attachCurriculumLessonMemberships,
  type AuthoredLanguageCurriculum,
} from "../src/curriculum-membership.js";
import {
  readCurriculumMembershipOwners,
} from "../src/curriculum-membership-shards.js";
import {
  defaultCurriculumRoot,
  loadAuthoredLanguageCurricula,
  loadLanguageCurricula,
} from "../src/loader.js";

const roots: string[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

function fixture(): string {
  const root = mkdtempSync(join(tmpdir(), "hl-curriculum-membership-"));
  roots.push(root);
  mkdirSync(join(root, "toy", "lessons"), { recursive: true });
  mkdirSync(join(root, "toy", "curriculum-membership.d"), { recursive: true });
  writeFileSync(join(root, "toy", "lessons", "TOY-L1.md"), "---\nid: TOY-L1\n---\n");
  writeFileSync(
    join(root, "toy", "curriculum-membership.d", "_meta.json"),
    `${JSON.stringify({ version: 1, language: "toy" }, null, 2)}\n`,
  );
  return root;
}

function authored(): AuthoredLanguageCurriculum {
  return {
    version: 1,
    language: "toy",
    path: [
      {
        id: "TOY-PATH-1",
        spine_node: "SPINE-1",
        before: [],
        inline: ["TOY-EXT-1"],
        after: [],
      },
    ],
    spine: {
      "SPINE-1": { segments: ["TOY-PATH-1"], omits: [], relocates: {} },
    },
    extensions: [
      {
        id: "TOY-EXT-1",
        stage: "A1",
        kind: "required",
        category: "grammar",
        canDo: "I can test the direct owner.",
        prerequisites: [],
      },
    ],
  };
}

describe("direct curriculum lesson owners", () => {
  it("reconstructs the exact pre-migration public graph", () => {
    const curricula = loadLanguageCurricula(defaultCurriculumRoot());
    const digest = createHash("sha256")
      .update(JSON.stringify(curricula))
      .digest("hex");
    // The digest and count below are of the LIVE curriculum graph, so they move
    // whenever any track gains a lesson -- not only when the membership
    // migration changes shape. 7204 -> 7246 was the first Spanish A2 vocabulary
    // tranche (chapters 431-436) and 7246 -> 7267 was the second (437-439) and
    // 7267 -> 7295 was the third (440-443) and
    // 7295 -> 7322 was the fourth (444-447) and
    // 7322 -> 7351 was the fifth (448-451) and
    // 7351 -> 7376 was the sixth (452-455) and
    // 7376 -> 7398 was the seventh (456-459) and
    // 7398 -> 7423 was the eighth (460-464) and
    // 7423 -> 7429 was the ninth (465, on its own) and
    // 7429 -> 7435 was the tenth (466) and
    // 7435 -> 7441 was the eleventh (467) and
    // 7441 -> 7447 was the twelfth (468) and
    // 7447 -> 7453 was the thirteenth (469) and
    // 7453 -> 7459 was the fourteenth (470) and
    // 7459 -> 7467 was the fifteenth (471), which is EIGHT rather than six because that chapter teaches six
    // headwords instead of four, and
    // 7467 -> 7474 was the sixteenth (472), SEVEN for five headwords, and
    // 7474 -> 7479 was the seventeenth (473), FIVE for three headwords, and
    // 7479 -> 7485 is the eighteenth (474) and the LAST of the A2 vocabulary programme -- see the mock-audit
    // test for why it stops there; the guard still does its job, which is to make any OTHER change to the
    // public graph fail loudly rather than pass quietly.
    //
    // Attribution was checked by RECONSTRUCTION, not assumed. A clean worktree
    // at the previous commit was loaded by this same function and reproduced the
    // previous digest and count byte for byte, so nothing outside the new path
    // segments moved. For 444-447 the control was origin/main, which reproduced
    // 1787ed7f... and 7295 exactly; ES-PATH-444-CASA, ES-PATH-445-PUESTO,
    // ES-PATH-446-MOVER and ES-PATH-447-JUICIO hold exactly 27 lessons between
    // them, and the corpus-wide count moved by exactly 27.
    //
    // Filtering the loaded object is NOT a substitute for that control: dropping
    // segments from an in-memory graph leaves the remaining records ordered and
    // shaped as the larger load produced them, so it reproduces neither digest.
    // Only re-loading a checkout that never had the files answers the question.
    // For 465 the control was origin/main at 35573f7d0a in a clean worktree,
    // which reproduced 06f2a8b6... and 7423 exactly; ES-PATH-465-APUNTE holds
    // exactly 6 lessons and the corpus-wide count moved by exactly 6. For 466
    // the control was origin/main at e34a62aa9f, which reproduced 4fdc1ab5...
    // and 7429 exactly; ES-PATH-466-INFORME holds exactly 6 lessons and the
    // count again moved by exactly 6. For 467 the control reproduced
    // 954755fd... and 7435 exactly, and ES-PATH-467-ALIMENTO holds 6. For 468
    // the control reproduced 5ef6e708... and 7441 exactly, and
    // ES-PATH-468-ABRIGO holds 6. For 469 the control was origin/main at
    // 4e5939c6db, which reproduced e1344996... and 7447 exactly;
    // ES-PATH-469-OFICINA holds exactly 6 lessons and the count moved by 6.
    // For 470 the control was origin/main at f3a7bed89d, which reproduced
    // 7248b982... and 7453 exactly; ES-PATH-470-PARAGUAS holds exactly 6
    // lessons and the count moved by 6. For 471 the control was origin/main at
    // 2eac85f55a, which reproduced ac4c49e1... and 7459 exactly;
    // ES-PATH-471-CURSO holds exactly 8 lessons and the count moved by 8.
    // For 472 the control was origin/main at 524d393136, which reproduced
    // 1218403f... and 7467 exactly; ES-PATH-472-ENCUESTA holds exactly 7
    // lessons and the count moved by 7. For 473 the control was origin/main at
    // f83309736e, which reproduced dc9be5f8... and 7474 exactly;
    // ES-PATH-473-CONSEJO holds exactly 5 lessons and the count moved by 5.
    // For 474 the control was origin/main at 2245f54d77, which reproduced
    // 6d6ea3c0... and 7479 exactly; ES-PATH-474-PAGO holds exactly 6 lessons
    // and the count moved by 6.
    //
    // HL-C417 is the FIRST change to move this digest WITHOUT moving the count.
    // No lesson was added: 44 Spanish path segments (431-474) were repointed from
    // the A1 `SPINE-READ-SIGNS-AND-NOTICES` to the new A2
    // `SPINE-READ-PRACTICAL-TEXTS`, and all 23 tracks gained that node's
    // realization ledger. The count staying at 7485 across a digest move is the
    // evidence that the migration RELOCATED membership rather than creating it.
    //
    // Attribution was again by reconstruction, and this time by a full structural
    // diff as well. A clean worktree at d647c889e6 reproduced 7b1879e4... and 7485
    // byte for byte; dumping the loaded graph from both trees and diffing them
    // gives 294 changed lines and NOTHING that is not the migration: 44
    // `spine_node` values flipped, 23 `SPINE-READ-PRACTICAL-TEXTS` ledgers added
    // (22 of them with empty `segments`, because only Spanish realizes the rung
    // today), and the 44 `ES-PATH-4xx` ids moving from one node's derived
    // `segments` list to the other's.
    //
    // 7485 -> 7486 is HL-C424: ONE Telugu lesson, `TE-C08-andi`, splitting the
    // respectful `-andi` ending out of `TE-C08-dayachesi`, which introduced four
    // atoms against a budget of three. It is the first entry here that is not a
    // Spanish vocabulary tranche, and the first for a track other than Spanish.
    //
    // Attribution by reconstruction AND structural diff, as for HL-C417. A clean
    // worktree at 29907337c4 reproduced 5a445a65... and 7485 byte for byte.
    // Dumping the loaded graph from both trees and diffing gives SIXTEEN changed
    // lines and nothing that is not this change: `TE-C08-andi` joins
    // `TE-PATH-013`'s lessons, that segment's `inline` gains
    // `TE-EXT-013-POLITENESS`, and the extension node appears with its single
    // lesson. `TE-C09-kshaminchandi` moving from pathOrder 1 to 2 does not
    // appear, because order is positional in the derived list and both lessons
    // are in it.
    //
    // 7486 -> 7495 is HL-C426: NINE Telugu review lessons, the second-pass arc
    // that closes the track's reinforcement blocker. Fifty atoms at or below
    // pre-A1 had been revisited exactly once -- their own chapter recap and
    // nothing after it -- and the level gate wants two. None of the nine
    // introduces an atom or a headword, which is the whole design: a `review`
    // lesson is outside `CONTENT_TYPES`, so it discharges reinforcement debt
    // without moving the vocabulary shortfall or the chapter atom budget.
    //
    // Attribution by reconstruction AND structural diff, as for HL-C417 and
    // HL-C424. A clean worktree at 0fc9d29a5a reproduced 40c1fd07... and 7486
    // byte for byte. Dumping the loaded graph from both trees and diffing gives
    // NINETY changed lines and nothing that is not this change: the nine lessons
    // joining their segments' derived `lessons` lists, four new
    // `TE-EXT-0xx-CONSOLIDATION` extension nodes, and those four ids appearing
    // in their segments' `inline` lists. No existing lesson moved node.
    //
    // 7495 -> 7505 is HL-C428: TEN `review` lessons across SIX tracks -- marwadi,
    // persian, portuguese, italian, urdu and latin -- applying HL-C426's finding
    // to the rest of the corpus. Those six carried 36 atoms between them that the
    // pre-A1 gate counts as revisited fewer than twice, and all six now carry
    // none. As with HL-C426 none of the ten introduces an atom or a headword.
    //
    // Attribution by reconstruction AND structural diff. A clean worktree at
    // 5112463d2c reproduced 0a385842... and 7495 byte for byte. Dumping the loaded
    // graph from both trees and diffing gives 122 changed lines and nothing that
    // is not this change: the ten lessons joining their segments' derived
    // `lessons` lists, six new `-CONSOLIDATION` extension nodes, and those six ids
    // appearing in their segments' `inline` or `after` lists. No existing lesson
    // moved node.
    //
    // 7505 -> 7511 is HL-C430: SIX `review` lessons continuing HL-C428's sweep
    // onto chinese and bengali, the next two tracks without reinforcement-window
    // position pins. Chinese needed three because six of its sixteen thin atoms
    // had no later revisit at all and wanted two passes each. As before, none of
    // the six introduces an atom or a headword.
    //
    // Attribution by reconstruction AND structural diff. A clean worktree at
    // d7e6d60073 reproduced 7d8ceaa9... and 7505 byte for byte. Dumping the
    // loaded graph from both trees and diffing gives 65 changed lines and
    // nothing that is not this change: the six lessons joining their segments'
    // derived `lessons` lists, three new `-CONSOLIDATION` extension nodes, and
    // those three ids appearing in their segments' `inline` lists.
    //
    // 7511 -> 7517 is HL-C431: SIX `review` lessons taking french and russian to
    // zero pre-A1 reinforcement debt. Three each, and in both tracks the third
    // exists because an atom had NO later revisit at all: french's three accent
    // marks and russian's `ли`. No new atoms and no new headwords.
    //
    // Attribution by reconstruction AND structural diff. A clean worktree at
    // 444726ca3b reproduced e0968596... and 7511 byte for byte. Dumping the
    // loaded graph from both trees and diffing gives 63 changed lines and
    // nothing that is not this change: the six lessons joining their segments'
    // derived `lessons` lists, three new `-CONSOLIDATION` extension nodes, and
    // those ids appearing in their segments' `after` lists.
    //
    // 7517 -> 7520 is HL-C432: THREE `review` lessons taking gujarati and
    // marathi to zero pre-A1 reinforcement debt, which finishes the small-debt
    // half of HL-C428's sweep. Gujarati needed only one because all thirteen of
    // its thin atoms sit below sequence 1820 and one lesson at 1825 reaches
    // them all. None of the three introduces an atom or a headword.
    //
    // Attribution by reconstruction AND structural diff. A clean worktree at
    // b34f182c78 reproduced 647640ba... and 7517 byte for byte. Dumping the
    // loaded graph from both trees and diffing gives 40 changed lines and
    // nothing that is not this change: the three lessons joining their
    // segments' derived `lessons` lists, two new `-CONSOLIDATION` extension
    // nodes, and those ids in their segments' `inline` lists.
    //
    // 7520 -> 7526 is HL-C433: SIX `review` lessons taking sanskrit to zero
    // pre-A1 reinforcement debt. Six rather than three because sanskrit is the
    // first track in this programme where one revisit per atom was not enough:
    // 17 of its 28 thin atoms had ZERO revisits and the criterion asks for two,
    // so 28 atoms cost 45 retrieval slots. None introduces an atom or a
    // headword.
    //
    // Attribution by reconstruction AND structural diff. A clean worktree at
    // ef987b60d6 reproduced 87df31a5... and 7520 byte for byte. Dumping the
    // loaded graph from both trees and diffing gives 36 changed lines and
    // nothing that is not this change: the six lessons joining their segments'
    // and extensions' derived `lessons` lists, twice each, with the previous
    // last element gaining a trailing comma. It is the FIRST entry here with no
    // new extension node at all -- every one of the six reuses the extension
    // its path segment already carried -- which is why 36 lines buys six
    // lessons where HL-C431 needed 63 for the same count.
    //
    // 7526 -> 7533 is HL-C434: SEVEN `practice` lessons taking german to zero
    // pre-A1 reinforcement debt. Seven because 19 of its 37 thin atoms had ZERO
    // revisits and the criterion asks for two, so 37 atoms cost 56 retrieval
    // slots. They are `practice` and not `review` because german has no `review`
    // lesson at all -- 35 of its lessons are `practice` -- and both types are
    // outside `CONTENT_TYPES`, so neither adds a headword.
    //
    // Attribution by reconstruction AND structural diff. A clean worktree at
    // 5a8e144dd3 reproduced aa7d8235... and 7526 byte for byte. Dumping the
    // loaded graph from both trees and diffing gives 54 changed lines and
    // nothing that is not this change: the seven lessons joining their segments'
    // and extensions' derived `lessons` lists, ONE new extension node
    // (`GE-EXT-024-CONSOLIDATION`, the only target segment that carried no
    // extension at all), and that id joining `GE-PATH-024`'s `after` list.
    //
    // 7533 -> 7539 is HL-C436: SIX `review` lessons taking kannada to zero
    // pre-A1 reinforcement debt. Kannada is the cheapest track of the programme
    // per atom -- 41 thin atoms but only THREE with zero revisits, so 44
    // retrieval slots against sanskrit's 45 for 28 atoms. What made it work was
    // not the count: 23 of the 41 are script-recognition atoms on ONE segment,
    // so three of the six lessons are a script recall and the chapter spread
    // that looked expensive collapsed.
    //
    // Attribution by reconstruction AND structural diff. A clean worktree at
    // 61c1d2a914 reproduced e4e3c864... and 7533 byte for byte. Dumping the
    // loaded graph from both trees and diffing gives 50 changed lines and
    // nothing that is not this change: the six lessons joining their segments'
    // and extensions' derived `lessons` lists, and one new chapter-81 segment
    // plus its extension (`KA-PATH-81-LEFTOVERS` / `KA-EXT-81-LEFTOVERS`),
    // which follow the chapter 77-80 script-recall pattern this track already
    // uses rather than inventing a shape.
    //
    // 7539 -> 7545 is HL-C437: SIX `review` lessons taking punjabi to zero pre-A1
    // reinforcement debt. Punjabi is the opposite shape to kannada: its forty
    // thin atoms sit on SEVENTEEN path segments, with no concentration any one
    // earlier chapter could reach, so the tranche is a new chapter 48 appended at
    // the end in three strands -- the form-field ladder, the Gurmukhi pieces, and
    // the courtesy and parting words -- rather than lessons placed among the
    // material they retrieve.
    //
    // Attribution by reconstruction AND structural diff. A clean worktree at
    // 08f6a81a44 reproduced 8fe09909... and 7539 byte for byte. Dumping the
    // loaded graph from both trees and diffing gives 84 changed lines and nothing
    // that is not this change: the six lessons joining their segments' and
    // extensions' derived `lessons` lists, THREE new segments and THREE new
    // extensions for chapter 48, and those segment ids joining their spine nodes'
    // derived lists. Three segments rather than one because a lesson's
    // `spine_node` must equal its segment's, and the three strands serve three
    // different nodes.
    //
    // 7545 -> 7550 is HL-C438: FIVE `review` lessons taking malayalam to zero
    // pre-A1 reinforcement debt. Cheapest track of the programme: 44 thin atoms
    // but only ONE with zero revisits, so 45 retrieval slots, and 18 of the 44
    // sit on `ML-PATH-100` -- kannada's concentration rather than punjabi's
    // spread.
    //
    // Attribution by reconstruction AND structural diff. A clean worktree at
    // 96691b9dfc reproduced b22345f3... and 7545 byte for byte. Dumping the
    // loaded graph from both trees and diffing gives THIRTY changed lines, the
    // smallest in this pin's history, and nothing that is not this change: five
    // lessons joining their segments' and extensions' derived `lessons` lists,
    // twice each, with the previous last element gaining a trailing comma.
    //
    // It is also the FIRST entry here with no new graph node of any kind --
    // no segment, no extension. Every one of the five was appended to an
    // existing segment whose `spine_node` already matched its content, so
    // malayalam needed no new chapter and none of the four-part chapter
    // registration HL-C436 and HL-C437 required.
    //
    // 7550 -> 7555 is HL-C439: FIVE `review` lessons taking hindi to zero pre-A1
    // reinforcement debt. 44 thin atoms and 54 retrieval slots, because TEN of
    // the 44 had never been revisited at all and each of those costs TWO lessons
    // rather than one -- `practisedAtoms` is a set per lesson, so an atom cannot
    // earn two revisits from one page however many times that page names it.
    //
    // The five split by where the debt sat, not by chapter: 22 script atoms went
    // into two recall pages in chapter 85 (`HI-PATH-78-READING`), the twelve
    // atoms of the opening chapters 1-4 into chapter 89, ten scattered leftovers
    // -- a body word, a person word in three registers, two things in a house,
    // two form fields and the two named meals -- into chapter 94, and the ten
    // zero-revisit atoms got their SECOND pass on the book's last page, chapter
    // 105. 12 + 10 + 10 + 22 slots from the first four, and the last one closes
    // the remaining ten. 54 exactly.
    //
    // Attribution by reconstruction AND structural diff. A clean worktree at
    // 32d9f83927 reproduced 98e3031a... and 7550 byte for byte. Dumping the
    // loaded graph from both trees and diffing gives TWENTY-SIX changed lines and
    // nothing that is not this change: five lessons joining their segments' and
    // extensions' derived `lessons` lists, twice each, with four previous last
    // elements gaining a trailing comma on each side.
    //
    // Like HL-C438 and unlike HL-C436 and HL-C437, no new graph node of any kind
    // -- no segment, no extension, no chapter. Every one of the five was appended
    // to an existing segment whose `spine_node` already matched its content.
    //
    // 7555 -> 7564 is HL-C440: NINE `review` lessons taking tamil to zero pre-A1
    // reinforcement debt, and the LAST of the large ones. 73 thin atoms, 15 of
    // them never revisited, so (15 x 2) + 58 = 88 retrieval slots -- the biggest
    // of the programme, but concentrated rather than spread: 42 of the 73 sat on
    // just TWO segments.
    //
    // Nine rather than eight because one planned 16-atom page was split; slots
    // are a floor, not a cap, and the duration ceiling is the real constraint.
    // All nine went onto EXISTING segments in chapters 84-86 -- the far end of a
    // track whose debt is concentrated in chapters 1-39 -- so no new chapter,
    // segment or extension, as for HL-C438 and HL-C439.
    //
    // Attribution by reconstruction AND structural diff. A clean worktree at
    // d9b659429b reproduced 998a8193... and 7555 byte for byte. Dumping the
    // loaded graph from both trees and diffing gives THIRTY changed lines and
    // nothing that is not this change: nine lessons joining their segments' and
    // extensions' derived `lessons` lists, twice each, with three previous last
    // elements gaining a trailing comma on each side.
    //
    // The gate that shaped this tranche was NOT the digest but
    // `forwardReferences`: tamil sits at 8 against a ceiling of 8, and
    // `lessonsEarly > 1` at 4 against 4, so a retrieval page printing any word
    // the course teaches later would have failed the build. It held at 8 and 4.
    //
    // 7564 -> 7579 is HL-C441: FIFTEEN `review` lessons taking arabic to zero
    // pre-A1 reinforcement debt, which takes the CORPUS to zero -- arabic was the
    // last of twenty-three tracks still carrying any. 78 thin atoms, 68 of them
    // never revisited, so (68 x 2) + 10 = 146 retrieval slots: the largest number
    // in the programme, and it came from the SHAPE of the debt rather than its
    // size. Arabic's pre-A1 has almost no review layer, so 87% of its thin atoms
    // had no later revisit at all, against tamil's 21% and malayalam's 2%. An atom
    // at zero revisits costs two lessons; one at one costs one. That ratio, not the
    // atom count, is what set this tranche at fifteen lessons.
    //
    // The debt was also CONCENTRATED -- three segments carried 52 of the 78 -- so
    // as with HL-C438, HL-C439 and HL-C440, every one of the fifteen went onto an
    // EXISTING segment whose `spine_node` already matched. No new chapter, segment
    // or extension. Six sit mid-book (chapters 10-24) to give the opening chapters
    // a first pass at a reachable distance, and nine sit late (33-45) for the
    // second, which is why the closing page of the book is a retrieval page.
    //
    // Attribution by reconstruction AND structural diff. A clean worktree at
    // c5b37e048e reproduced 297f88ff... and 7564 byte for byte. Dumping the loaded
    // graph from both trees and diffing gives exactly THIRTY removed lines and
    // SIXTY added ones, and nothing that is not this change: each of the fifteen
    // lessons joins its segment's and its extension's derived `lessons` list, which
    // is thirty new entries, and the thirty previous last elements each gain a
    // trailing comma.
    expect(digest).toBe("7a0bc9d7f03bce2c84f6d9f11685008253737dc2b39eec2288f88b21e9bf2331");
    expect(curricula.flatMap((curriculum) => curriculum.path).flatMap((path) => path.lessons))
      .toHaveLength(7579);
  });

  it("keeps the canonical curriculum shards free of derived lesson arrays", () => {
    for (const curriculum of loadAuthoredLanguageCurricula(defaultCurriculumRoot())) {
      expect(curriculum.path.every((path) => !Object.hasOwn(path, "lessons"))).toBe(true);
      expect(curriculum.extensions.every((extension) => !Object.hasOwn(extension, "lessons"))).toBe(true);
    }
  });

  it("requires one canonical owner for every lesson filename", () => {
    const root = fixture();
    expect(() => readCurriculumMembershipOwners(root, "toy")).toThrow(
      /missing: TOY-L1\.json/,
    );
    writeFileSync(
      join(root, "toy", "curriculum-membership.d", "TOY-L1.json"),
      `${JSON.stringify({
        id: "TOY-L1",
        pathSegment: "TOY-PATH-1",
        pathOrder: 0,
        extensions: [{ id: "TOY-EXT-1", order: 0 }],
      }, null, 2)}\n`,
    );
    expect(readCurriculumMembershipOwners(root, "toy").owners).toHaveLength(1);
  });

  it("rejects aggregate lesson-array resurrection", () => {
    const input = authored();
    (input.path[0] as unknown as { lessons: string[] }).lessons = ["TOY-L1"];
    expect(() => attachCurriculumLessonMemberships(input, [])).toThrow(
      /must not store derived 'lessons'/,
    );
  });

  it("rejects gaps in per-path order", () => {
    expect(() =>
      attachCurriculumLessonMemberships(authored(), [
        { id: "TOY-L1", pathSegment: "TOY-PATH-1", pathOrder: 1, extensions: [] },
      ]),
    ).toThrow(/expected 0/);
  });

  it("rejects extension membership outside the lesson's path", () => {
    const input = authored();
    input.path[0]!.inline = [];
    expect(() =>
      attachCurriculumLessonMemberships(input, [
        {
          id: "TOY-L1",
          pathSegment: "TOY-PATH-1",
          pathOrder: 0,
          extensions: [{ id: "TOY-EXT-1", order: 0 }],
        },
      ]),
    ).toThrow(/is not attached to path segment/);
  });
});
