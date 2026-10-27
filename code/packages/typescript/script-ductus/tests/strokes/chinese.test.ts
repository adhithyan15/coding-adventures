import { beforeAll, describe, expect, it } from "vitest";
import { SCRIPTS, verifiedLetterFont } from "../../src/scriptdata";
import {
  DUCTUS,
  ductusFor,
  ductusKey,
  joinGaps,
  penLifts,
  penPath,
  penPathD,
  penTip,
  type LetterDuctus,
  type Point,
  type Stroke,
} from "../../src/strokes";
import { registerStrokeHonestyTests } from "../support/stroke-honesty";

const CHINESE_REN = DUCTUS[ductusKey("chinese", "人")];
const CHINESE_PERSON_RADICAL = DUCTUS[ductusKey("chinese", "亻")];
const CHINESE_MOUTH = DUCTUS[ductusKey("chinese", "口")];
const CHINESE_WOMAN = DUCTUS[ductusKey("chinese", "女")];
const CHINESE_CHILD = DUCTUS[ductusKey("chinese", "子")];
const CHINESE_SUN = DUCTUS[ductusKey("chinese", "日")];
const CHINESE_SPEECH_RADICAL = DUCTUS[ductusKey("chinese", "讠")];
const CHINESE_WATER_RADICAL = DUCTUS[ductusKey("chinese", "氵")];
const CHINESE_ROOF_RADICAL = DUCTUS[ductusKey("chinese", "宀")];
const CHINESE_YOU = DUCTUS[ductusKey("chinese", "你")];
const CHINESE_GOOD = DUCTUS[ductusKey("chinese", "好")];
const CHINESE_I = DUCTUS[ductusKey("chinese", "我")];
const CHINESE_BE = DUCTUS[ductusKey("chinese", "是")];
const CHINESE_NOT = DUCTUS[ductusKey("chinese", "不")];
const CHINESE_NAME = DUCTUS[ductusKey("chinese", "名")];
const CHINESE_CHARACTER = DUCTUS[ductusKey("chinese", "字")];
const CHINESE_THANK = DUCTUS[ductusKey("chinese", "谢")];
const CHINESE_PLEASE = DUCTUS[ductusKey("chinese", "请")];
const CHINESE_AGAIN = DUCTUS[ductusKey("chinese", "再")];
const CHINESE_SEE = DUCTUS[ductusKey("chinese", "见")];
const CHINESE_WHAT = DUCTUS[ductusKey("chinese", "什")];
const CHINESE_PARTICLE_ME = DUCTUS[ductusKey("chinese", "么")];
const CHINESE_EARLY = DUCTUS[ductusKey("chinese", "早")];
const CHINESE_UP = DUCTUS[ductusKey("chinese", "上")];
const CHINESE_HAN = DUCTUS[ductusKey("chinese", "汉")];
const CHINESE_LANGUAGE = DUCTUS[ductusKey("chinese", "语")];
const CHINESE_WRITING = DUCTUS[ductusKey("chinese", "文")];
const CHINESE_COUNTRY = DUCTUS[ductusKey("chinese", "国")];
const CHINESE_LOOK = DUCTUS[ductusKey("chinese", "看")];
const CHINESE_BOOK = DUCTUS[ductusKey("chinese", "书")];
const CHINESE_QUESTION = DUCTUS[ductusKey("chinese", "吗")];
const CHINESE_SIX = DUCTUS[ductusKey("chinese", "六")];
const CHINESE_SEVEN = DUCTUS[ductusKey("chinese", "七")];
const CHINESE_EIGHT = DUCTUS[ductusKey("chinese", "八")];
const CHINESE_NINE = DUCTUS[ductusKey("chinese", "九")];
const CHINESE_TEN = DUCTUS[ductusKey("chinese", "十")];
const CHINESE_HUNDRED = DUCTUS[ductusKey("chinese", "百")];
const NEW_PARTICLE_AND_JOINING_DUCTUS = [
  [DUCTUS[ductusKey("chinese", "的")], "%E7%9A%84", [4, 2, 4, 2, 2, 3, 5, 3]],
  [DUCTUS[ductusKey("chinese", "有")], "%E6%9C%89", [4, 6, 7, 12, 2, 2]],
  [DUCTUS[ductusKey("chinese", "个")], "%E4%B8%AA", [4, 4, 2]],
  [DUCTUS[ductusKey("chinese", "了")], "%E4%BA%86", [5, 5]],
  [DUCTUS[ductusKey("chinese", "呢")], "%E5%91%A2", [2, 4, 5, 4, 2, 8, 3, 6]],
  [DUCTUS[ductusKey("chinese", "对")], "%E5%AF%B9", [7, 6, 2, 6, 2]],
  [DUCTUS[ductusKey("chinese", "岁")], "%E5%B2%81", [2, 4, 3, 3, 10, 11]],
  [DUCTUS[ductusKey("chinese", "和")], "%E5%92%8C", [4, 2, 6, 5, 2, 2, 5, 5]],
  [
    DUCTUS[ductusKey("chinese", "喜")],
    "%E5%96%9C",
    [3, 5, 2, 5, 3, 3, 4, 4, 4, 5, 5, 2],
  ],
  [DUCTUS[ductusKey("chinese", "欢")], "%E6%AC%A2", [11, 5, 3, 4, 5, 4]],
] as const;

// The fifty-four characters stage 2 teaches to write, in the order its
// writing lessons write them: tranche a (chapters 31-55), then tranche b
// (chapters 58-82).
const STAGE_TWO =
  "他们妈男叫来这那吃饭喝水买多少去车天今星期年明点午晚手说话想" +
  "机白长快电打起门问气雨马山做火开下进客乐会课里前";

// The thirteen characters the A1 core (chapters 85-97) teaches to write, in
// the order its writing lessons write them.
const A1_CORE = "在哪没两给能要也太都为因时";

// The twenty-four characters A1 vocabulary teaches to write, in the order
// its writing lessons write them: tranche c (chapters 99-119), then
// tranche d (chapters 126-140).
const A1_VOCABULARY = "洗菜用本站试网外边出心包" + "以些音吵病春千扫重节每半";

// The ten characters the A2 core (chapters 147-156) teaches to write, in the
// order its writing lessons write them.
const A2_CORE = "后月正过到得比还从元";

// The twenty-four characters A2 vocabulary teaches to write, in the order
// its writing lessons write them: tranche e (chapters 158-187), then
// tranche f (chapters 188-217).
const A2_VOCABULARY = "房米衣茶事觉杯冷候面常间" + "钱号店回路票找关东离公地";

const OWNER_SCRIPTS = new Set(["chinese"]);
const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter((letter) =>
  OWNER_SCRIPTS.has(letter.script),
);

describe("handwriting ductus", () => {
  registerStrokeHonestyTests(letters, {});

  beforeAll(() => {
    expect(verifiedLetterFont("人", CHINESE_REN.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("亻", CHINESE_PERSON_RADICAL.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("口", CHINESE_MOUTH.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("女", CHINESE_WOMAN.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("子", CHINESE_CHILD.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("日", CHINESE_SUN.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("讠", CHINESE_SPEECH_RADICAL.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("氵", CHINESE_WATER_RADICAL.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("宀", CHINESE_ROOF_RADICAL.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("你", CHINESE_YOU.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("好", CHINESE_GOOD.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("我", CHINESE_I.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("是", CHINESE_BE.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("不", CHINESE_NOT.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("名", CHINESE_NAME.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("字", CHINESE_CHARACTER.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("谢", CHINESE_THANK.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("请", CHINESE_PLEASE.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("再", CHINESE_AGAIN.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("见", CHINESE_SEE.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("什", CHINESE_WHAT.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("么", CHINESE_PARTICLE_ME.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("早", CHINESE_EARLY.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("上", CHINESE_UP.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("汉", CHINESE_HAN.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("语", CHINESE_LANGUAGE.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("文", CHINESE_WRITING.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("国", CHINESE_COUNTRY.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("看", CHINESE_LOOK.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("书", CHINESE_BOOK.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(verifiedLetterFont("吗", CHINESE_QUESTION.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    for (const numeral of [
      CHINESE_SIX,
      CHINESE_SEVEN,
      CHINESE_EIGHT,
      CHINESE_NINE,
      CHINESE_TEN,
      CHINESE_HUNDRED,
    ]) {
      expect(verifiedLetterFont(numeral.glyph, numeral.source.url)).toBe(
        "_fonts/NotoSansSC-Subset.ttf",
      );
    }
    for (const [ductus] of NEW_PARTICLE_AND_JOINING_DUCTUS) {
      expect(verifiedLetterFont(ductus.glyph, ductus.source.url)).toBe(
        "_fonts/NotoSansSC-Subset.ttf",
      );
    }
  });

  it("marks Chinese complete with every current-corpus row source-verified", () => {
    const chinese = SCRIPTS.find((script) => script.script === "chinese")!;
    expect(chinese.complete).toBe(true);
    // 50 -> 60: Chapters 22–27 add the particles, joining words and five
    // high-value free points whose writing rows the course now verifies.
    // `complete` means the inventory covers the current lesson corpus, so the
    // source-verified inventory and authored ductus must grow together.
    // 60 -> 61: 尔, the sound half of 你, which ZH-W01-er teaches alone.
    // 61 -> 62: 很, which ZH-W30-hen teaches as the character in front of
    // every adjective (我很好), written in chapter 30.
    // 62 -> 92: the thirty characters stage 2's tranche a writes (chapters
    // 31-55), each right after the word that anchors it: 他 们 妈 男 叫 来
    // 这 那 吃 饭 喝 水 买 多 少 去 车 天 今 星 期 年 明 点 午 晚 手 说 话 想.
    // 92 -> 116: the twenty-four characters stage 2's tranche b writes
    // (chapters 58-82), each right after the word that anchors it: 机 白 长
    // 快 电 打 起 门 问 气 雨 马 山 做 火 开 下 进 客 乐 会 课 里 前.
    // 116 -> 129: the thirteen characters the A1 core writes
    // (chapters 85-97), one a chapter, each right after the word that
    // anchors it: 在 哪 没 两 给 能 要 也 太 都 为 因 时.
    // 129 -> 141: the twelve characters A1 vocabulary tranche c writes
    // (chapters 99-119), at most one a chapter, each right after the word
    // that anchors it: 洗 菜 用 本 站 试 网 外 边 出 心 包.
    // 141 -> 153: the twelve characters A1 vocabulary tranche d writes
    // (chapters 126-140), at most one a chapter, each right after the
    // word that anchors it: 以 些 音 吵 病 春 千 扫 重 节 每 半.
    // 153 -> 163: the ten characters the A2 core writes
    // (chapters 147-156), one a chapter, each right after the word that
    // anchors it: 后 月 正 过 到 得 比 还 从 元.
    // 163 -> 175: the twelve characters A2 vocabulary tranche e writes
    // (chapters 158-187), each right after the word that anchors
    // it: 房 米 衣 茶 事 觉 杯 冷 候 面 常 间.
    // 175 -> 187: the twelve characters A2 vocabulary tranche f writes
    // (chapters 188-217), each right after the word that anchors
    // it: 钱 号 店 回 路 票 找 关 东 离 公 地.
    // 187 -> 199: the twelve characters A2 vocabulary tranche g writes
    // (chapters 218-247), each right after the word that anchors
    // it: 风 云 雪 花 草 头 疼 身 体 医 西 现.
    expect(chinese.letters).toHaveLength(199);
    expect(new Set(chinese.letters.map((letter) => letter.glyph)).size).toBe(
      199,
    );
    expect(
      chinese.letters.every((letter) => letter.strokeOrderSource !== undefined),
    ).toBe(true);
  });

  it("Chinese 尔 draws the five strokes that close 你, in the same order and directions", () => {
    // 尔 alone and 尔 inside 你 cite the same Hanzi Writer medians, so the
    // standalone character must keep 你's last five runs: the same captions,
    // the same two hooks, and every run heading the same way.
    const er = DUCTUS[ductusKey("chinese", "尔")];
    const ni = DUCTUS[ductusKey("chinese", "你")];
    expect(penLifts(er)).toBe(4);
    const labels = (strokes: typeof er.strokes) =>
      strokes.map((stroke) => stroke.segments.map((segment) => segment.label));
    expect(labels(er.strokes).slice(1)).toEqual(labels(ni.strokes.slice(3)));
    expect(labels(er.strokes)[0]).toEqual(["draw the upper left-falling stroke"]);
    const heading = (stroke: (typeof er.strokes)[number]) => {
      const path = penPath(stroke);
      const [a, b] = [path[0], path[path.length - 1]];
      return [Math.sign(b.x - a.x), Math.sign(b.y - a.y)];
    };
    expect(er.strokes.map(heading)).toEqual(ni.strokes.slice(2).map(heading));
    expect(er.source.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%B0%94.json",
    );
    expect(verifiedLetterFont("尔", er.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
  });

  it("Chinese 很 writes 彳 (亻 with one more left-falling stroke above it) before 艮, keeping all nine source medians", () => {
    // ZH-W30-hen teaches 很 as two pieces, not nine strokes: its left piece is
    // the person piece the reader has written since 你, with one more
    // left-falling stroke above it. So strokes 2-3 must head the way 亻's two
    // strokes do, stroke 1 must fall the same way above them, and all three
    // must lie left of every stroke of 艮.
    const hen = DUCTUS[ductusKey("chinese", "很")];
    expect(hen.source.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%BE%88.json",
    );
    expect(hen.source.citation).toMatch(
      /Hanzi Writer Data 很\.json.*medians 1–9.*snapshot 68d10a4/i,
    );
    expect(verifiedLetterFont("很", hen.source.url)).toBe(
      "_fonts/NotoSansSC-Subset.ttf",
    );
    expect(hen.strokes.map((stroke) => penPath(stroke).length)).toEqual([
      7, 9, 5, 7, 3, 3, 10, 7, 13,
    ]);
    expect(penLifts(hen)).toBe(8);
    const heading = (stroke: (typeof hen.strokes)[number]) => {
      const path = penPath(stroke);
      const [a, b] = [path[0], path[path.length - 1]];
      return [Math.sign(b.x - a.x), Math.sign(b.y - a.y)];
    };
    expect(hen.strokes.slice(1, 3).map(heading)).toEqual(
      CHINESE_PERSON_RADICAL.strokes.map(heading),
    );
    expect(heading(hen.strokes[0])).toEqual(heading(hen.strokes[1]));
    const xs = (strokes: typeof hen.strokes) =>
      strokes.flatMap((stroke) => penPath(stroke).map((point) => point.x));
    expect(Math.max(...xs(hen.strokes.slice(0, 3)))).toBeLessThan(
      Math.min(...xs(hen.strokes.slice(3))),
    );
  });

  it("Chinese 人 draws the left-falling stroke before the lifted right-falling stroke", () => {
    expect(CHINESE_REN.script).toBe("chinese");
    expect(penLifts(CHINESE_REN)).toBe(1);
    expect(CHINESE_REN.strokes).toHaveLength(2);
    expect(CHINESE_REN.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 1],
    );
    const left = CHINESE_REN.strokes[0].segments[0].path;
    const right = CHINESE_REN.strokes[1].segments[0].path;
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(left[0].x).toBeGreaterThan(left.at(-1)!.x);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(right[0].x).toBeLessThan(right.at(-1)!.x);
  });

  it("Chinese 亻 draws the left-falling stroke before the lifted vertical", () => {
    expect(CHINESE_PERSON_RADICAL.script).toBe("chinese");
    expect(penLifts(CHINESE_PERSON_RADICAL)).toBe(1);
    expect(CHINESE_PERSON_RADICAL.strokes).toHaveLength(2);
    expect(
      CHINESE_PERSON_RADICAL.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1]);
    const left = CHINESE_PERSON_RADICAL.strokes[0].segments[0].path;
    const vertical = CHINESE_PERSON_RADICAL.strokes[1].segments[0].path;
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(left[0].x).toBeGreaterThan(left.at(-1)!.x);
    expect(vertical[0].y).toBeGreaterThan(vertical.at(-1)!.y);
    expect(vertical[0].x).toBe(vertical.at(-1)!.x);
  });

  it("Chinese 口 descends the left, joins the top and right, then closes the bottom", () => {
    expect(CHINESE_MOUTH.script).toBe("chinese");
    expect(penLifts(CHINESE_MOUTH)).toBe(2);
    expect(CHINESE_MOUTH.strokes).toHaveLength(3);
    expect(
      CHINESE_MOUTH.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 2, 1]);
    const left = CHINESE_MOUTH.strokes[0].segments[0].path;
    const top = CHINESE_MOUTH.strokes[1].segments[0].path;
    const right = CHINESE_MOUTH.strokes[1].segments[1].path;
    const bottom = CHINESE_MOUTH.strokes[2].segments[0].path;
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(top.at(-1)).toEqual(right[0]);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(bottom[0].x).toBeLessThan(bottom.at(-1)!.x);
  });

  it("Chinese 女 keeps its first bend joined before the falling and horizontal strokes", () => {
    expect(CHINESE_WOMAN.script).toBe("chinese");
    expect(penLifts(CHINESE_WOMAN)).toBe(2);
    expect(CHINESE_WOMAN.strokes).toHaveLength(3);
    expect(
      CHINESE_WOMAN.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1, 1]);
    const descent = CHINESE_WOMAN.strokes[0].segments[0].path;
    const sweep = CHINESE_WOMAN.strokes[0].segments[1].path;
    const falling = CHINESE_WOMAN.strokes[1].segments[0].path;
    const horizontal = CHINESE_WOMAN.strokes[2].segments[0].path;
    expect(descent[0].y).toBeGreaterThan(descent.at(-1)!.y);
    expect(descent[0].x).toBeGreaterThan(descent.at(-1)!.x);
    expect(descent.at(-1)).toEqual(sweep[0]);
    expect(sweep[0].x).toBeLessThan(sweep.at(-1)!.x);
    expect(sweep[0].y).toBeGreaterThan(sweep.at(-1)!.y);
    expect(falling[0].x).toBeGreaterThan(falling.at(-1)!.x);
    expect(falling[0].y).toBeGreaterThan(falling.at(-1)!.y);
    expect(horizontal[0].x).toBeLessThan(horizontal.at(-1)!.x);
  });

  it("Chinese 子 keeps both hooks joined before its final horizontal", () => {
    expect(CHINESE_CHILD.script).toBe("chinese");
    expect(penLifts(CHINESE_CHILD)).toBe(2);
    expect(CHINESE_CHILD.strokes).toHaveLength(3);
    expect(
      CHINESE_CHILD.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 2, 1]);
    const top = CHINESE_CHILD.strokes[0].segments[0].path;
    const topTurn = CHINESE_CHILD.strokes[0].segments[1].path;
    const vertical = CHINESE_CHILD.strokes[1].segments[0].path;
    const baseHook = CHINESE_CHILD.strokes[1].segments[1].path;
    const horizontal = CHINESE_CHILD.strokes[2].segments[0].path;
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(top.at(-1)).toEqual(topTurn[0]);
    expect(topTurn[0].x).toBeGreaterThan(topTurn.at(-1)!.x);
    expect(topTurn[0].y).toBeGreaterThan(topTurn.at(-1)!.y);
    expect(vertical[0].y).toBeGreaterThan(vertical.at(-1)!.y);
    expect(vertical.at(-1)).toEqual(baseHook[0]);
    expect(baseHook[0].x).toBeGreaterThan(baseHook.at(-1)!.x);
    expect(horizontal[0].x).toBeLessThan(horizontal.at(-1)!.x);
  });

  it("Chinese 日 writes the inside bar before separately closing the box", () => {
    expect(CHINESE_SUN.script).toBe("chinese");
    expect(penLifts(CHINESE_SUN)).toBe(3);
    expect(CHINESE_SUN.strokes).toHaveLength(4);
    expect(CHINESE_SUN.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 2, 1, 1],
    );
    const left = CHINESE_SUN.strokes[0].segments[0].path;
    const top = CHINESE_SUN.strokes[1].segments[0].path;
    const right = CHINESE_SUN.strokes[1].segments[1].path;
    const middle = CHINESE_SUN.strokes[2].segments[0].path;
    const bottom = CHINESE_SUN.strokes[3].segments[0].path;
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(top[0].x).toBeLessThan(top.at(-1)!.x);
    expect(top.at(-1)).toEqual(right[0]);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
    expect(middle[0].x).toBeLessThan(middle.at(-1)!.x);
    expect(bottom[0].x).toBeLessThan(bottom.at(-1)!.x);
  });

  it("Chinese 讠 lifts after the dot and keeps both later turns joined", () => {
    expect(CHINESE_SPEECH_RADICAL.script).toBe("chinese");
    expect(penLifts(CHINESE_SPEECH_RADICAL)).toBe(1);
    expect(CHINESE_SPEECH_RADICAL.strokes).toHaveLength(2);
    expect(
      CHINESE_SPEECH_RADICAL.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 3]);
    const dot = CHINESE_SPEECH_RADICAL.strokes[0].segments[0].path;
    const horizontal = CHINESE_SPEECH_RADICAL.strokes[1].segments[0].path;
    const descent = CHINESE_SPEECH_RADICAL.strokes[1].segments[1].path;
    const rise = CHINESE_SPEECH_RADICAL.strokes[1].segments[2].path;
    expect(dot[0].x).toBeLessThan(dot.at(-1)!.x);
    expect(dot[0].y).toBeGreaterThan(dot.at(-1)!.y);
    expect(horizontal[0].x).toBeLessThan(horizontal.at(-1)!.x);
    expect(horizontal.at(-1)).toEqual(descent[0]);
    expect(descent[0].y).toBeGreaterThan(descent.at(-1)!.y);
    expect(descent.at(-1)).toEqual(rise[0]);
    expect(rise[0].x).toBeLessThan(rise.at(-1)!.x);
    expect(rise[0].y).toBeLessThan(rise.at(-1)!.y);
  });

  it("Chinese 氵 draws two falling dots before its joined rising bottom stroke", () => {
    expect(CHINESE_WATER_RADICAL.script).toBe("chinese");
    expect(penLifts(CHINESE_WATER_RADICAL)).toBe(2);
    expect(CHINESE_WATER_RADICAL.strokes).toHaveLength(3);
    expect(
      CHINESE_WATER_RADICAL.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 2]);
    const upper = CHINESE_WATER_RADICAL.strokes[0].segments[0].path;
    const middle = CHINESE_WATER_RADICAL.strokes[1].segments[0].path;
    const bottomTurn = CHINESE_WATER_RADICAL.strokes[2].segments[0].path;
    const bottomRise = CHINESE_WATER_RADICAL.strokes[2].segments[1].path;
    expect(upper[0].x).toBeLessThan(upper.at(-1)!.x);
    expect(upper[0].y).toBeGreaterThan(upper.at(-1)!.y);
    expect(middle[0].x).toBeLessThan(middle.at(-1)!.x);
    expect(middle[0].y).toBeGreaterThan(middle.at(-1)!.y);
    expect(bottomTurn[0].x).toBeGreaterThan(bottomTurn[1].x);
    expect(bottomTurn.at(-1)).toEqual(bottomRise[0]);
    expect(bottomRise[0].x).toBeLessThan(bottomRise.at(-1)!.x);
    expect(bottomRise[0].y).toBeLessThan(bottomRise.at(-1)!.y);
  });

  it("Chinese 宀 draws two separate marks before its joined roof hook", () => {
    expect(CHINESE_ROOF_RADICAL.script).toBe("chinese");
    expect(penLifts(CHINESE_ROOF_RADICAL)).toBe(2);
    expect(CHINESE_ROOF_RADICAL.strokes).toHaveLength(3);
    expect(
      CHINESE_ROOF_RADICAL.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 2]);
    const dot = CHINESE_ROOF_RADICAL.strokes[0].segments[0].path;
    const left = CHINESE_ROOF_RADICAL.strokes[1].segments[0].path;
    const roof = CHINESE_ROOF_RADICAL.strokes[2].segments[0].path;
    const hook = CHINESE_ROOF_RADICAL.strokes[2].segments[1].path;
    expect(dot[0].x).toBeLessThan(dot.at(-1)!.x);
    expect(dot[0].y).toBeGreaterThan(dot.at(-1)!.y);
    expect(left[0].x).toBeGreaterThan(left.at(-1)!.x);
    expect(left[0].y).toBeGreaterThan(left.at(-1)!.y);
    expect(roof[0].x).toBeLessThan(roof.at(-1)!.x);
    expect(roof.at(-1)).toEqual(hook[0]);
    expect(hook[0].x).toBeGreaterThan(hook.at(-1)!.x);
    expect(hook[0].y).toBeGreaterThan(hook.at(-1)!.y);
  });

  it("Chinese 你 writes 亻 before two joined hooks and two separate dots", () => {
    expect(CHINESE_YOU.script).toBe("chinese");
    expect(penLifts(CHINESE_YOU)).toBe(6);
    expect(CHINESE_YOU.strokes).toHaveLength(7);
    expect(CHINESE_YOU.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 1, 1, 2, 2, 1, 1],
    );
    const personFall = CHINESE_YOU.strokes[0].segments[0].path;
    const personVertical = CHINESE_YOU.strokes[1].segments[0].path;
    const upperFall = CHINESE_YOU.strokes[2].segments[0].path;
    const horizontal = CHINESE_YOU.strokes[3].segments[0].path;
    const upperHook = CHINESE_YOU.strokes[3].segments[1].path;
    const vertical = CHINESE_YOU.strokes[4].segments[0].path;
    const baseHook = CHINESE_YOU.strokes[4].segments[1].path;
    const leftDot = CHINESE_YOU.strokes[5].segments[0].path;
    const rightDot = CHINESE_YOU.strokes[6].segments[0].path;
    expect(personFall[0].x).toBeGreaterThan(personFall.at(-1)!.x);
    expect(personVertical[0].y).toBeGreaterThan(personVertical.at(-1)!.y);
    expect(upperFall[0].x).toBeGreaterThan(upperFall.at(-1)!.x);
    expect(horizontal.at(-1)).toEqual(upperHook[0]);
    expect(upperHook[0].x).toBeGreaterThan(upperHook.at(-1)!.x);
    expect(vertical.at(-1)).toEqual(baseHook[0]);
    expect(baseHook[0].x).toBeGreaterThan(baseHook.at(-1)!.x);
    expect(leftDot[0].x).toBeGreaterThan(leftDot.at(-1)!.x);
    expect(rightDot[0].x).toBeLessThan(rightDot.at(-1)!.x);
  });

  it("Chinese 好 writes 女 before 子 with three joined turns", () => {
    expect(CHINESE_GOOD.script).toBe("chinese");
    expect(penLifts(CHINESE_GOOD)).toBe(5);
    expect(CHINESE_GOOD.strokes).toHaveLength(6);
    expect(
      CHINESE_GOOD.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([2, 1, 1, 2, 2, 1]);
    const womanBend = CHINESE_GOOD.strokes[0].segments[0].path;
    const womanSweep = CHINESE_GOOD.strokes[0].segments[1].path;
    const womanFall = CHINESE_GOOD.strokes[1].segments[0].path;
    const womanBar = CHINESE_GOOD.strokes[2].segments[0].path;
    const childBar = CHINESE_GOOD.strokes[3].segments[0].path;
    const childTurn = CHINESE_GOOD.strokes[3].segments[1].path;
    const childVertical = CHINESE_GOOD.strokes[4].segments[0].path;
    const childHook = CHINESE_GOOD.strokes[4].segments[1].path;
    const childMiddle = CHINESE_GOOD.strokes[5].segments[0].path;
    expect(womanBend.at(-1)).toEqual(womanSweep[0]);
    expect(womanSweep[0].x).toBeLessThan(womanSweep.at(-1)!.x);
    expect(womanFall[0].x).toBeGreaterThan(womanFall.at(-1)!.x);
    expect(womanBar[0].x).toBeLessThan(womanBar.at(-1)!.x);
    expect(childBar.at(-1)).toEqual(childTurn[0]);
    expect(childTurn[0].x).toBeGreaterThan(childTurn.at(-1)!.x);
    expect(childVertical.at(-1)).toEqual(childHook[0]);
    expect(childHook[0].x).toBeGreaterThan(childHook.at(-1)!.x);
    expect(childMiddle[0].x).toBeLessThan(childMiddle.at(-1)!.x);
  });

  it("Chinese 我 preserves seven strokes and the joined vertical hook", () => {
    expect(CHINESE_I.script).toBe("chinese");
    expect(penLifts(CHINESE_I)).toBe(6);
    expect(CHINESE_I.strokes).toHaveLength(7);
    expect(CHINESE_I.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 2, 1, 2, 1, 1,
    ]);
    const vertical = CHINESE_I.strokes[2].segments[0].path;
    const hook = CHINESE_I.strokes[2].segments[1].path;
    expect(vertical.at(-1)).toEqual(hook[0]);
    expect(hook[0].x).toBeGreaterThan(hook.at(-1)!.x);
    const slash = CHINESE_I.strokes[4].segments[0].path;
    const slashHook = CHINESE_I.strokes[4].segments[1].path;
    expect(slash.at(-1)).toEqual(slashHook[0]);
    expect(slashHook[0].y).toBeLessThan(slashHook.at(-1)!.y);
  });

  it("Chinese 是 closes 日 before its five-stroke lower body", () => {
    expect(CHINESE_BE.script).toBe("chinese");
    expect(penLifts(CHINESE_BE)).toBe(8);
    expect(CHINESE_BE.strokes).toHaveLength(9);
    expect(CHINESE_BE.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 2, 1, 1, 1, 1, 1, 1, 1,
    ]);
    const top = CHINESE_BE.strokes[1].segments[0].path;
    const right = CHINESE_BE.strokes[1].segments[1].path;
    expect(top.at(-1)).toEqual(right[0]);
    expect(right[0].y).toBeGreaterThan(right.at(-1)!.y);
  });

  it("Chinese 不 keeps its four source strokes as four pen-down runs", () => {
    expect(CHINESE_NOT.script).toBe("chinese");
    expect(penLifts(CHINESE_NOT)).toBe(3);
    expect(CHINESE_NOT.strokes).toHaveLength(4);
    expect(CHINESE_NOT.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 1, 1, 1],
    );
    expect(CHINESE_NOT.strokes[0].segments[0].path.at(-1)!.x).toBeGreaterThan(
      CHINESE_NOT.strokes[0].segments[0].path[0].x,
    );
    expect(CHINESE_NOT.strokes[1].segments[0].path.at(-1)!.x).toBeLessThan(
      CHINESE_NOT.strokes[1].segments[0].path[0].x,
    );
  });

  it("Chinese 名 completes 夕 before 口 and preserves both joined turns", () => {
    expect(CHINESE_NAME.script).toBe("chinese");
    expect(penLifts(CHINESE_NAME)).toBe(5);
    expect(CHINESE_NAME.strokes).toHaveLength(6);
    expect(
      CHINESE_NAME.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 2, 1, 1, 2, 1]);
    expect(CHINESE_NAME.strokes[1].segments[0].path.at(-1)).toEqual(
      CHINESE_NAME.strokes[1].segments[1].path[0],
    );
    expect(CHINESE_NAME.strokes[4].segments[0].path.at(-1)).toEqual(
      CHINESE_NAME.strokes[4].segments[1].path[0],
    );
  });

  it("Chinese 字 completes 宀 before 子 and preserves all three joined turns", () => {
    expect(CHINESE_CHARACTER.script).toBe("chinese");
    expect(penLifts(CHINESE_CHARACTER)).toBe(5);
    expect(CHINESE_CHARACTER.strokes).toHaveLength(6);
    expect(
      CHINESE_CHARACTER.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 2, 2, 2, 1]);
    for (const strokeIndex of [2, 3, 4]) {
      expect(
        CHINESE_CHARACTER.strokes[strokeIndex].segments[0].path.at(-1),
      ).toEqual(CHINESE_CHARACTER.strokes[strokeIndex].segments[1].path[0]);
    }
  });

  it("Chinese 谢 completes 讠, 身, and 寸 in order and preserves all five joined turns", () => {
    expect(CHINESE_THANK.script).toBe("chinese");
    expect(penLifts(CHINESE_THANK)).toBe(11);
    expect(CHINESE_THANK.strokes).toHaveLength(12);
    expect(
      CHINESE_THANK.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 3, 1, 1, 3, 1, 1, 1, 1, 1, 2, 1]);
    for (const strokeIndex of [1, 4]) {
      for (let segmentIndex = 0; segmentIndex < 2; segmentIndex++) {
        expect(
          CHINESE_THANK.strokes[strokeIndex].segments[segmentIndex].path.at(-1),
        ).toEqual(
          CHINESE_THANK.strokes[strokeIndex].segments[segmentIndex + 1].path[0],
        );
      }
    }
    expect(CHINESE_THANK.strokes[10].segments[0].path.at(-1)).toEqual(
      CHINESE_THANK.strokes[10].segments[1].path[0],
    );
  });

  it("Chinese 请 completes 讠 before 青 and preserves all four joined turns", () => {
    expect(CHINESE_PLEASE.script).toBe("chinese");
    expect(penLifts(CHINESE_PLEASE)).toBe(9);
    expect(CHINESE_PLEASE.strokes).toHaveLength(10);
    expect(
      CHINESE_PLEASE.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 3, 1, 1, 1, 1, 1, 3, 1, 1]);
    for (const strokeIndex of [1, 7]) {
      for (let segmentIndex = 0; segmentIndex < 2; segmentIndex++) {
        expect(
          CHINESE_PLEASE.strokes[strokeIndex].segments[segmentIndex].path.at(
            -1,
          ),
        ).toEqual(
          CHINESE_PLEASE.strokes[strokeIndex].segments[segmentIndex + 1]
            .path[0],
        );
      }
    }
  });

  it("Chinese 再 closes last and preserves both turns inside the enclosing stroke", () => {
    expect(CHINESE_AGAIN.script).toBe("chinese");
    expect(penLifts(CHINESE_AGAIN)).toBe(5);
    expect(CHINESE_AGAIN.strokes).toHaveLength(6);
    expect(
      CHINESE_AGAIN.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 3, 1, 1, 1]);
    for (let segmentIndex = 0; segmentIndex < 2; segmentIndex++) {
      expect(
        CHINESE_AGAIN.strokes[2].segments[segmentIndex].path.at(-1),
      ).toEqual(CHINESE_AGAIN.strokes[2].segments[segmentIndex + 1].path[0]);
    }
  });

  it("Chinese 见 completes its open frame before both lower runs", () => {
    expect(CHINESE_SEE.script).toBe("chinese");
    expect(penLifts(CHINESE_SEE)).toBe(3);
    expect(CHINESE_SEE.strokes).toHaveLength(4);
    expect(CHINESE_SEE.strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 2, 1, 3],
    );
    for (const strokeIndex of [1, 3]) {
      const stroke = CHINESE_SEE.strokes[strokeIndex];
      for (
        let segmentIndex = 0;
        segmentIndex + 1 < stroke.segments.length;
        segmentIndex++
      ) {
        expect(stroke.segments[segmentIndex].path.at(-1)).toEqual(
          stroke.segments[segmentIndex + 1].path[0],
        );
      }
    }
  });

  it("Chinese 什 completes 亻 before writing 十 in four separate strokes", () => {
    expect(CHINESE_WHAT.script).toBe("chinese");
    expect(penLifts(CHINESE_WHAT)).toBe(3);
    expect(CHINESE_WHAT.strokes).toHaveLength(4);
    expect(
      CHINESE_WHAT.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 1, 1, 1]);
    const leftStrokes = CHINESE_WHAT.strokes.slice(0, 2).flatMap(penPath);
    const rightStrokes = CHINESE_WHAT.strokes.slice(2).flatMap(penPath);
    expect(Math.max(...leftStrokes.map((point) => point.x))).toBeLessThan(
      Math.min(...rightStrokes.map((point) => point.x)),
    );
  });

  it("Chinese 么 keeps the second fall joined to its rightward base sweep", () => {
    expect(CHINESE_PARTICLE_ME.script).toBe("chinese");
    expect(penLifts(CHINESE_PARTICLE_ME)).toBe(2);
    expect(CHINESE_PARTICLE_ME.strokes).toHaveLength(3);
    expect(
      CHINESE_PARTICLE_ME.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 2, 1]);
    expect(CHINESE_PARTICLE_ME.strokes[1].segments[0].path.at(-1)).toEqual(
      CHINESE_PARTICLE_ME.strokes[1].segments[1].path[0],
    );
  });

  it("Chinese 早 completes 日 before the two strokes of 十", () => {
    expect(CHINESE_EARLY.script).toBe("chinese");
    expect(penLifts(CHINESE_EARLY)).toBe(5);
    expect(CHINESE_EARLY.strokes).toHaveLength(6);
    expect(
      CHINESE_EARLY.strokes.map((stroke) => stroke.segments.length),
    ).toEqual([1, 2, 1, 1, 1, 1]);
    expect(CHINESE_EARLY.strokes[1].segments[0].path.at(-1)).toEqual(
      CHINESE_EARLY.strokes[1].segments[1].path[0],
    );
    expect(
      Math.max(...penPath(CHINESE_EARLY.strokes[4]).map((point) => point.y)),
    ).toBeLessThan(
      Math.min(...penPath(CHINESE_EARLY.strokes[3]).map((point) => point.y)),
    );
  });

  it("Chinese 上 writes the vertical before its short and long horizontals", () => {
    expect(CHINESE_UP.script).toBe("chinese");
    expect(penLifts(CHINESE_UP)).toBe(2);
    expect(CHINESE_UP.strokes).toHaveLength(3);
    expect(CHINESE_UP.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 1,
    ]);
    const vertical = penPath(CHINESE_UP.strokes[0]);
    const shortHorizontal = penPath(CHINESE_UP.strokes[1]);
    const base = penPath(CHINESE_UP.strokes[2]);
    expect(vertical[0].y).toBeGreaterThan(vertical.at(-1)!.y);
    expect(shortHorizontal[0].x).toBeLessThan(shortHorizontal.at(-1)!.x);
    expect(base[0].x).toBeLessThan(base.at(-1)!.x);
    expect(base.at(-1)!.x - base[0].x).toBeGreaterThan(
      shortHorizontal.at(-1)!.x - shortHorizontal[0].x,
    );
  });

  it("Chinese 人 traces its two medians to the pinned PRC-order dataset", () => {
    const src = CHINESE_REN.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%BA%BA.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 人\.json.*ordered stroke paths and medians 1–2.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*two ordered strokes.*Median 1.*upper centre.*descends down-left.*median 2.*central junction.*descends down-right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*one intervening pen lift.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 亻 traces its falling stroke and vertical to the pinned PRC-order dataset", () => {
    const src = CHINESE_PERSON_RADICAL.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%BA%BB.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 亻\.json.*ordered stroke paths and medians 1–2.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*two ordered strokes.*Median 1.*upper right.*bends slightly right.*descends down-left.*median 2.*central junction.*moves slightly down-right.*descends vertically.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*one intervening pen lift.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 口 traces its three-run closing order to the pinned PRC-order dataset", () => {
    const src = CHINESE_MOUTH.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%8F%A3.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 口\.json.*ordered stroke paths and medians 1–3.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*three ordered strokes.*Median 1.*upper left.*descends the left side.*median 2.*upper left.*crosses the top from left to right.*turns without lifting.*descends the right side.*median 3.*lower left.*closes the bottom from left to right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*joined corner.*two intervening pen lifts.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 女 traces its bent first run and two lifted strokes to the pinned PRC-order dataset", () => {
    const src = CHINESE_WOMAN.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%A5%B3.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 女\.json.*ordered stroke paths and medians 1–3.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*three ordered strokes.*Median 1.*upper centre.*short down-right entry.*bends down-left.*lower-left junction.*turns without lifting.*sweeps down-right.*median 2.*upper right.*falls down-left.*median 3.*left edge.*crosses the middle from left to right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*first stroke's internal turn.*two intervening pen lifts.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 子 traces its two joined turns and final horizontal to the pinned PRC-order dataset", () => {
    const src = CHINESE_CHILD.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%AD%90.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 子\.json.*ordered stroke paths and medians 1–3.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*three ordered strokes.*Median 1.*upper left.*crosses the top from left to right.*turns without lifting.*sweeps down-left.*median 2.*upper-left turn.*descends through the centre.*hooks left at the base without lifting.*median 3.*left edge.*crosses the middle from left to right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*both internal turns.*two intervening pen lifts.*base hook is flatter.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 日 traces its joined corner and inside-before-close order to the pinned PRC-order dataset", () => {
    const src = CHINESE_SUN.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%97%A5.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 日\.json.*ordered stroke paths and medians 1–4.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*four ordered strokes.*Median 1.*upper left.*descends the left side.*median 2.*upper left.*crosses the top from left to right.*turns without lifting.*descends the right side.*median 3.*left edge.*crosses the middle from left to right.*median 4.*lower left.*closes the bottom from left to right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*joined top-right corner.*inside-before-close order.*three intervening pen lifts.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 讠 traces its dot and joined turning stroke to the pinned PRC-order dataset", () => {
    const src = CHINESE_SPEECH_RADICAL.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E8%AE%A0.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 讠\.json.*ordered stroke paths and medians 1–2.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*two ordered strokes.*Median 1.*upper left.*dot down-right.*median 2.*left edge.*rises slightly.*short horizontal.*turns without lifting.*descend.*turns again without lifting.*finishes up-right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*both turns inside the second stroke.*one intervening pen lift.*squarer.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 氵 traces its falling dots and rising bottom stroke to the pinned PRC-order dataset", () => {
    const src = CHINESE_WATER_RADICAL.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%B0%B5.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 氵\.json.*ordered stroke paths and medians 1–3.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*three ordered strokes.*Median 1.*upper left.*descends down-right.*median 2.*middle left.*descends down-right.*median 3.*bottom.*moves slightly up-left.*rises.*upper right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*source order.*two intervening pen lifts.*shallower turn.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 宀 traces its dot, left drop, and joined roof hook to the pinned PRC-order dataset", () => {
    const src = CHINESE_ROOF_RADICAL.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%AE%80.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 宀\.json.*ordered stroke paths and medians 1–3.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*three ordered strokes.*Median 1.*upper left.*top dot down-right.*median 2.*left side.*descends down-left.*median 3.*left roof edge.*horizontally to the right.*hooks down-left without lifting.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*joined horizontal hook.*two intervening pen lifts.*squarer.*more vertical.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 你 traces its component order and two joined hooks to the pinned PRC-order dataset", () => {
    const src = CHINESE_YOU.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%BD%A0.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 你\.json.*ordered stroke paths and medians 1–7.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*seven ordered strokes.*Medians 1–2.*亻 first.*left-falling stroke.*separately started vertical.*Median 3.*upper right.*falls down-left.*median 4.*crosses rightward.*hooks down-left without lifting.*median 5.*centre.*descends.*hooks left without lifting.*medians 6–7.*lower-left dot down-left.*lower-right dot down-right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*both joined hooks.*component order.*six intervening pen lifts.*squarer.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 好 traces 女-before-子 and three joined turns to the pinned PRC-order dataset", () => {
    const src = CHINESE_GOOD.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%A5%BD.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 好\.json.*ordered stroke paths and medians 1–6.*snapshot 68d10a4.*updated from Make Me a Hanzi.*22 June 2019/i,
    );
    expect(src.variation).toMatch(
      /PRC-order dataset.*six ordered strokes.*Medians 1–3.*女 first.*bent stroke.*down-left.*sweeps right without lifting.*separately started stroke falls down-left.*horizontal crosses left-to-right.*Medians 4–6.*子.*horizontal turns down-left without lifting.*vertical descends and hooks left without lifting.*middle horizontal crosses left-to-right.*proper stroke order.*medians.*stroke-order animation.*People's Republic of China stroke order.*Noto Sans SC.*女-before-子 component order.*all three joined turns.*five intervening pen lifts.*more angular.*flatter.*Arphic-derived source graphics/i,
    );
  });

  it("Chinese 我 traces seven strokes to the pinned PRC-order dataset", () => {
    const src = CHINESE_I.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%88%91.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 我\.json.*medians 1–7.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /seven ordered strokes.*upper-left falling stroke.*upper horizontal.*hooked vertical.*lower rising stroke.*long curved slash.*hooks upward.*without lifting.*separate rising slash up-left.*final upper-right dot.*People's Republic of China stroke order.*Noto Sans SC.*both joined hooks.*six lifts/i,
    );
  });

  it("Chinese 是 traces 日-first order to the pinned PRC-order dataset", () => {
    const src = CHINESE_BE.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%98%AF.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 是\.json.*medians 1–9.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /nine ordered strokes.*Medians 1–4.*日 first.*left vertical.*joined top and right sides.*inner horizontal.*closing bottom horizontal.*Medians 5–9.*wide horizontal.*central vertical.*short lower-right horizontal.*lower-left falling stroke.*long finishing stroke down-right.*People's Republic of China stroke order.*Noto Sans SC.*joined top-right corner.*eight intervening lifts/i,
    );
  });

  it("Chinese 不 traces all four separate strokes to the pinned PRC-order dataset", () => {
    const src = CHINESE_NOT.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%B8%8D.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 不\.json.*medians 1–4.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /four ordered strokes.*top horizontal.*left to right.*long falling stroke.*down-left.*central vertical.*right-falling dot.*People's Republic of China stroke order.*Noto Sans SC.*three intervening lifts/i,
    );
  });

  it("Chinese 名 traces 夕-before-口 order to the pinned PRC-order dataset", () => {
    const src = CHINESE_NAME.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%90%8D.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 名\.json.*medians 1–6.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /six ordered strokes.*Medians 1–3.*夕 first.*left-falling stroke.*horizontal.*down-left without lifting.*inner down-right dot.*Medians 4–6.*口.*left vertical.*top horizontal.*right side without lifting.*closing bottom horizontal.*People's Republic of China stroke order.*Noto Sans SC.*both joined turns.*five intervening lifts/i,
    );
  });

  it("Chinese 字 traces 宀-before-子 order to the pinned PRC-order dataset", () => {
    const src = CHINESE_CHARACTER.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%AD%97.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 字\.json.*medians 1–6.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /six ordered strokes.*Medians 1–3.*宀 first.*down-right top dot.*left-side down-left stroke.*horizontal roof.*hooks down-left without lifting.*Medians 4–6.*子.*top horizontal.*turns down-left without lifting.*vertical.*hooks left without lifting.*final middle horizontal.*People's Republic of China stroke order.*Noto Sans SC.*all three joined turns.*five intervening lifts/i,
    );
  });

  it("Chinese 谢 traces 讠-before-身-before-寸 order to the pinned PRC-order dataset", () => {
    const src = CHINESE_THANK.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E8%B0%A2.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 谢\.json.*medians 1–12.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /twelve ordered strokes.*Medians 1–2.*讠.*down-right dot.*short horizontal.*turns down.*finishes up-right without lifting.*Medians 3–9.*身.*upper falling stroke.*left side.*top horizontal.*right side.*hooks left.*two inner horizontals.*wide lower horizontal.*lower falling stroke down-left.*Medians 10–12.*寸.*horizontal.*vertical.*hooks left.*final down-right dot.*People's Republic of China stroke order.*Noto Sans SC.*all five internal turns.*eleven intervening lifts/i,
    );
  });

  it("Chinese 请 traces 讠-before-青 order to the pinned PRC-order dataset", () => {
    const src = CHINESE_PLEASE.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E8%AF%B7.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 请\.json.*medians 1–10.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /ten ordered strokes.*Medians 1–2.*讠.*down-right dot.*short horizontal.*turns down.*finishes up-right without lifting.*Medians 3–10.*青.*two upper horizontals.*central vertical.*wide middle horizontal.*lower left side.*lower top horizontal.*right side.*hooks left.*two inner horizontals.*People's Republic of China stroke order.*Noto Sans SC.*all four internal turns.*nine intervening lifts/i,
    );
  });

  it("Chinese 再 traces its frame-before-close order to the pinned PRC-order dataset", () => {
    const src = CHINESE_AGAIN.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%86%8D.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 再\.json.*medians 1–6.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /six ordered strokes.*top horizontal left-to-right.*left side.*Median 3.*frame's top.*right side.*hooks left.*central vertical.*inner horizontal.*closes with the long bottom horizontal left-to-right.*People's Republic of China stroke order.*Noto Sans SC.*both turns.*close-last rule.*five intervening lifts/i,
    );
  });

  it("Chinese 见 traces its frame-before-legs order to the pinned PRC-order dataset", () => {
    const src = CHINESE_SEE.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E8%A7%81.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 见\.json.*medians 1–4.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /four ordered strokes.*left side.*Median 2.*top horizontal.*right side.*left-falling leg.*Median 4.*second leg.*bends right.*upward hook.*People's Republic of China stroke order.*Noto Sans SC.*frame-before-legs.*three joined turns.*three intervening lifts/i,
    );
  });

  it("Chinese 什 traces its 亻-before-十 order to the pinned PRC-order dataset", () => {
    const src = CHINESE_WHAT.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%BB%80.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 什\.json.*medians 1–4.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /four ordered strokes.*Medians 1–2.*亻 first.*left-falling stroke.*separately started vertical.*Median 3.*十's horizontal left-to-right.*median 4.*descends 十's vertical.*People's Republic of China stroke order.*Noto Sans SC.*亻-before-十.*three intervening lifts/i,
    );
  });

  it("Chinese 么 traces its joined lower sweep to the pinned PRC-order dataset", () => {
    const src = CHINESE_PARTICLE_ME.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%B9%88.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 么\.json.*medians 1–3.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /three ordered strokes.*Median 1.*upper left-falling stroke.*Median 2.*upper right.*falls down-left.*turns without lifting.*sweeps right along the base.*Median 3.*final down-right dot.*People's Republic of China stroke order.*Noto Sans SC.*second stroke's joined turn.*two intervening lifts/i,
    );
  });

  it("Chinese 早 traces its complete 日-before-十 order to the pinned PRC dataset", () => {
    const src = CHINESE_EARLY.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%97%A9.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 早\.json.*medians 1–6.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /six ordered strokes.*Medians 1–4.*complete 日 first.*left side.*top horizontal.*turns down the right side.*middle horizontal.*closing bottom horizontal.*Median 5.*十's horizontal left-to-right.*median 6.*descends its vertical.*People's Republic of China stroke order.*Noto Sans SC.*日-before-十.*joined top-right turn.*five intervening lifts/i,
    );
  });

  it("Chinese 上 traces its vertical-and-horizontals order to the pinned PRC dataset", () => {
    const src = CHINESE_UP.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%B8%8A.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 上\.json.*medians 1–3.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /three ordered strokes.*Median 1.*central vertical.*top toward the base.*Median 2.*starts at the vertical.*short middle horizontal left-to-right.*Median 3.*long base horizontal left-to-right.*People's Republic of China stroke order.*Noto Sans SC.*both intervening lifts.*short-before-long horizontal contrast/i,
    );
  });

  it("Chinese 汉 preserves all five pinned source medians and their lift boundaries", () => {
    const src = CHINESE_HAN.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%B1%89.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 汉\.json.*medians 1–5.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /five ordered strokes.*Medians 1–3.*氵.*median 4.*turns down-left.*median 5.*falls down-right.*People's Republic of China.*source medians directly.*Noto Sans SC/i,
    );
    expect(CHINESE_HAN.strokes).toHaveLength(5);
    expect(CHINESE_HAN.strokes.map((stroke) => penPath(stroke).length)).toEqual(
      [3, 3, 5, 13, 7],
    );
    expect(penLifts(CHINESE_HAN)).toBe(4);
  });

  it("Chinese 语 preserves the 讠-before-五-before-口 source sequence", () => {
    const src = CHINESE_LANGUAGE.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E8%AF%AD.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 语\.json.*medians 1–9.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /nine ordered strokes.*Medians 1–2.*讠.*medians 3–6.*五.*medians 7–9.*口.*bottom-closing.*People's Republic of China.*source medians directly.*Noto Sans SC/i,
    );
    expect(CHINESE_LANGUAGE.strokes).toHaveLength(9);
    expect(
      CHINESE_LANGUAGE.strokes.map((stroke) => penPath(stroke).length),
    ).toEqual([3, 10, 5, 4, 7, 6, 3, 7, 5]);
    expect(penLifts(CHINESE_LANGUAGE)).toBe(8);
  });

  it("Chinese 文 preserves its four separately started source medians", () => {
    const src = CHINESE_WRITING.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E6%96%87.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 文\.json.*medians 1–4.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /four separately started strokes.*top dot.*horizontal.*left-falling.*right-falling.*People's Republic of China.*source medians directly.*Noto Sans SC/i,
    );
    expect(CHINESE_WRITING.strokes).toHaveLength(4);
    expect(
      CHINESE_WRITING.strokes.map((stroke) => penPath(stroke).length),
    ).toEqual([3, 6, 12, 7]);
    expect(penLifts(CHINESE_WRITING)).toBe(3);
  });

  it("Chinese 国 closes the frame only after all five inner 玉 medians", () => {
    const src = CHINESE_COUNTRY.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E5%9B%BD.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 国\.json.*medians 1–8.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /eight ordered strokes.*Medians 1–2.*outer frame.*medians 3–7.*玉.*median 8.*closes.*People's Republic of China.*source medians directly.*Noto Sans SC/i,
    );
    expect(CHINESE_COUNTRY.strokes).toHaveLength(8);
    expect(
      CHINESE_COUNTRY.strokes.map((stroke) => penPath(stroke).length),
    ).toEqual([6, 12, 6, 4, 5, 5, 3, 5]);
    expect(penLifts(CHINESE_COUNTRY)).toBe(7);
  });

  it("Chinese 看 preserves the four upper medians before all five 目 medians", () => {
    const src = CHINESE_LOOK.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E7%9C%8B.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 看\.json.*medians 1–9.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /nine ordered strokes.*Medians 1–4.*upper hand-like.*medians 5–9.*目.*People's Republic of China.*ordered sequence.*fitting.*Noto Sans SC/i,
    );
    expect(CHINESE_LOOK.strokes).toHaveLength(9);
    expect(
      CHINESE_LOOK.strokes.map((stroke) => penPath(stroke).length),
    ).toEqual([7, 5, 5, 9, 6, 9, 5, 5, 5]);
    expect(penLifts(CHINESE_LOOK)).toBe(8);
  });

  it("Chinese 书 preserves both folds before the upright and final dot", () => {
    const src = CHINESE_BOOK.source;
    expect(src.url).toBe(
      "https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/%E4%B9%A6.json",
    );
    expect(src.citation).toMatch(
      /Hanzi Writer Data 书\.json.*medians 1–4.*snapshot 68d10a4/i,
    );
    expect(src.variation).toMatch(
      /four ordered strokes.*two upper folding strokes.*central upright.*final upper-right dot.*People's Republic of China.*ordered sequence.*fitting.*Noto Sans SC/i,
    );
    expect(CHINESE_BOOK.strokes).toHaveLength(4);
    expect(
      CHINESE_BOOK.strokes.map((stroke) => penPath(stroke).length),
    ).toEqual([8, 13, 6, 5]);
    expect(penLifts(CHINESE_BOOK)).toBe(3);
  });

  it("Chinese numeral ductus preserves every pinned source median and lift", () => {
    const numerals = [
      [CHINESE_SIX, "%E5%85%AD", [4, 5, 5, 5]],
      [CHINESE_SEVEN, "%E4%B8%83", [5, 10]],
      [CHINESE_EIGHT, "%E5%85%AB", [6, 6]],
      [CHINESE_NINE, "%E4%B9%9D", [6, 13]],
      [CHINESE_TEN, "%E5%8D%81", [5, 5]],
      [CHINESE_HUNDRED, "%E7%99%BE", [5, 4, 4, 7, 4, 4]],
    ] as const;

    for (const [ductus, encodedGlyph, medianLengths] of numerals) {
      expect(ductus.source.url).toBe(
        `https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/${encodedGlyph}.json`,
      );
      expect(ductus.source.citation).toMatch(
        new RegExp(
          `Hanzi Writer Data ${ductus.glyph}\\.json.*snapshot 68d10a4`,
          "i",
        ),
      );
      expect(ductus.strokes.map((stroke) => penPath(stroke).length)).toEqual(
        medianLengths,
      );
      expect(penLifts(ductus)).toBe(ductus.strokes.length - 1);
    }
  });

  it("Chinese particles and joining words preserve every pinned source median and lift", () => {
    for (const [
      ductus,
      encodedGlyph,
      medianLengths,
    ] of NEW_PARTICLE_AND_JOINING_DUCTUS) {
      expect(ductus.source.url).toBe(
        `https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/${encodedGlyph}.json`,
      );
      expect(ductus.source.citation).toMatch(
        new RegExp(
          `Hanzi Writer Data ${ductus.glyph}\\.json.*medians 1–${ductus.strokes.length}.*snapshot 68d10a4`,
          "i",
        ),
      );
      expect(ductus.strokes.map((stroke) => penPath(stroke).length)).toEqual(
        medianLengths,
      );
      expect(penLifts(ductus)).toBe(ductus.strokes.length - 1);
    }
  });
});

// The checks the stage-2 and A1-core blocks below share: the way a stroke
// heads, the source a character must cite, and how its pieces compare.
const zh = (glyph: string) => DUCTUS[ductusKey("chinese", glyph)];
// The way a stroke heads, its first point to its last, to the nearest of
// the eight compass points: [1, 0] is rightward, [0, -1] straight down,
// [-1, -1] down to the left. (+ 0 turns a rounded -0 into 0.)
const heading = (stroke: Stroke) => {
  const path = penPath(stroke);
  const [a, b] = [path[0], path[path.length - 1]];
  const octant = Math.round(Math.atan2(b.y - a.y, b.x - a.x) / (Math.PI / 4));
  return [
    Math.round(Math.cos((octant * Math.PI) / 4)) + 0,
    Math.round(Math.sin((octant * Math.PI) / 4)) + 0,
  ];
};
const points = (strokes: readonly Stroke[]) =>
  strokes.flatMap((stroke) => penPath(stroke));
const expectPinnedSource = (
  letter: LetterDuctus,
  encodedGlyph: string,
  pathLengths: number[],
) => {
  expect(letter.source.url).toBe(
    `https://raw.githubusercontent.com/chanind/hanzi-writer-data/68d10a4b21150cae5e1ebbd223eed289cf32d90c/data/${encodedGlyph}.json`,
  );
  expect(letter.source.citation).toMatch(
    new RegExp(
      `Hanzi Writer Data ${letter.glyph}\\.json.*medians 1–${letter.strokes.length}.*snapshot 68d10a4`,
      "i",
    ),
  );
  expect(verifiedLetterFont(letter.glyph, letter.source.url)).toBe(
    "_fonts/NotoSansSC-Subset.ttf",
  );
  expect(letter.strokes.map((stroke) => penPath(stroke).length)).toEqual(
    pathLengths,
  );
  expect(letter.strokes.every((stroke) => stroke.segments.length === 1)).toBe(
    true,
  );
  expect(penLifts(letter)).toBe(letter.strokes.length - 1);
};
const expectSameHeadings = (
  letter: LetterDuctus,
  from: number,
  to: number,
  piece: LetterDuctus,
  pieceFrom: number,
) => {
  expect(letter.strokes.slice(from, to).map(heading)).toEqual(
    piece.strokes.slice(pieceFrom, pieceFrom + to - from).map(heading),
  );
};
const expectLeftOf = (left: readonly Stroke[], right: readonly Stroke[]) => {
  expect(Math.max(...points(left).map((point) => point.x))).toBeLessThan(
    Math.min(...points(right).map((point) => point.x)),
  );
};
const expectAbove = (top: readonly Stroke[], bottom: readonly Stroke[]) => {
  expect(Math.min(...points(top).map((point) => point.y))).toBeGreaterThan(
    Math.max(...points(bottom).map((point) => point.y)),
  );
};

// Stage 2 (chapters 31-82) teaches fifty-four more characters to write. Each
// case below pins what its chinese.json row promises about the ductus: the
// pinned Hanzi Writer Data file for stroke ORDER, one pen path per source
// median (point counts pinned, so a refit is a visible change), a lift before
// every stroke but the first, and the way every stroke heads (its first
// point to its last, to the nearest of eight compass points). Where a
// character holds a piece the reader has already written (亻 口 日 讠 人 大
// 小 门 辶 ...), that slice must head stroke for stroke as the piece does
// alone, which is what the lesson's "count pieces, not strokes" relies on;
// where its pieces sit side by side or one over the other, they must not
// overlap.
describe("stage 2 handwriting ductus", () => {
  it("covers every character stage 2 writes, each once", () => {
    const stageTwo = [...STAGE_TWO];
    expect(stageTwo).toHaveLength(54);
    expect(new Set(stageTwo).size).toBe(54);
    expect(stageTwo.every((glyph) => zh(glyph) !== undefined)).toBe(true);
  });

  it("Chinese 他 writes 亻 before 也, keeping all five source medians", () => {
    const letter = zh("他");
    expectPinnedSource(letter, "%E4%BB%96", [4, 4, 6, 3, 9]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 2, zh("亻"), 0);
    expectLeftOf(letter.strokes.slice(0, 2), letter.strokes.slice(2));
  });

  it("Chinese 们 writes 亻 before 门, keeping all five source medians", () => {
    const letter = zh("们");
    expectPinnedSource(letter, "%E4%BB%AC", [5, 4, 3, 3, 10]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [0, -1],
      [1, -1],
      [0, -1],
      [0, -1],
    ]);
    expectSameHeadings(letter, 0, 2, zh("亻"), 0);
    expectSameHeadings(letter, 2, 5, zh("门"), 0);
    expectLeftOf(letter.strokes.slice(0, 2), letter.strokes.slice(2));
  });

  it("Chinese 妈 writes 女 before 马, keeping all six source medians", () => {
    const letter = zh("妈");
    expectPinnedSource(letter, "%E5%A6%88", [7, 6, 2, 7, 12, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [0, -1],
      [1, 0],
      [1, -1],
      [0, -1],
      [1, 0],
    ]);
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 男 closes 田 before 力, keeping all seven source medians", () => {
    const letter = zh("男");
    expectPinnedSource(letter, "%E7%94%B7", [2, 8, 3, 4, 4, 11, 6]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [1, 0],
      [0, -1],
      [1, 0],
      [1, -1],
      [-1, -1],
    ]);
  });

  it("Chinese 叫 closes 口 before its right side, keeping all five source medians", () => {
    const letter = zh("叫");
    expectPinnedSource(letter, "%E5%8F%AB", [2, 3, 2, 4, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [1, 0],
      [1, -1],
      [0, -1],
    ]);
    expectSameHeadings(letter, 0, 3, zh("口"), 0);
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 来 writes both horizontals before the vertical and the spreading pair, keeping all seven source medians", () => {
    const letter = zh("来");
    expectPinnedSource(letter, "%E6%9D%A5", [6, 3, 2, 6, 3, 4, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [1, -1],
      [-1, -1],
      [1, 0],
      [0, -1],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 5, 7, zh("大"), 1);
  });

  it("Chinese 这 writes 文 before the walking piece 辶, keeping all seven source medians", () => {
    const letter = zh("这");
    expectPinnedSource(letter, "%E8%BF%99", [2, 4, 5, 4, 2, 7, 5]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, 0],
      [-1, -1],
      [1, -1],
      [1, -1],
      [0, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 4, 7, zh("进"), 4);
  });

  it("Chinese 那 writes the left piece before 阝, keeping all six source medians", () => {
    const letter = zh("那");
    expectPinnedSource(letter, "%E9%82%A3", [5, 2, 2, 4, 8, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, 0],
      [1, 0],
      [0, -1],
      [0, -1],
      [0, -1],
    ]);
    expectLeftOf(letter.strokes.slice(0, 4), letter.strokes.slice(4));
  });

  it("Chinese 吃 closes 口 before 乞, keeping all six source medians", () => {
    const letter = zh("吃");
    expectPinnedSource(letter, "%E5%90%83", [2, 3, 2, 5, 3, 12]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [1, 0],
      [-1, -1],
      [1, 0],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 3, zh("口"), 0);
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 饭 writes 饣 before 反, whose 又 heads as 对's does, keeping all seven source medians", () => {
    const letter = zh("饭");
    expectPinnedSource(letter, "%E9%A5%AD", [4, 5, 5, 3, 7, 9, 5]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [1, -1],
      [-1, 0],
      [0, -1],
      [0, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 5, 7, zh("对"), 0);
  });

  it("Chinese 喝 closes 口, then 日, before the hooked frame, keeping all twelve source medians", () => {
    const letter = zh("喝");
    expectPinnedSource(letter, "%E5%96%9D", [2, 3, 2, 2, 3, 4, 2, 3, 6, 4, 3, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
      [-1, -1],
      [1, -1],
      [-1, -1],
      [1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 3, 7, zh("日"), 0);
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 水 draws the middle vertical before the left and right sides, keeping all four source medians", () => {
    const letter = zh("水");
    expectPinnedSource(letter, "%E6%B0%B4", [5, 8, 3, 7]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [0, -1],
      [-1, -1],
      [1, -1],
    ]);
  });

  it("Chinese 买 draws the hooked top before 头, keeping all six source medians", () => {
    const letter = zh("买");
    expectPinnedSource(letter, "%E4%B9%B0", [7, 2, 3, 6, 8, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [1, -1],
      [1, -1],
      [1, 0],
      [-1, -1],
      [1, -1],
    ]);
  });

  it("Chinese 多 writes the upper 夕 before the lower one, in the same three strokes, keeping all six source medians", () => {
    const letter = zh("多");
    expectPinnedSource(letter, "%E5%A4%9A", [5, 7, 3, 3, 10, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [-1, -1],
      [1, -1],
      [-1, -1],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 3, 6, zh("多"), 0);
  });

  it("Chinese 少 writes 小 before the long left-falling stroke, keeping all four source medians", () => {
    const letter = zh("少");
    expectPinnedSource(letter, "%E5%B0%91", [4, 3, 3, 5]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [-1, -1],
      [1, -1],
      [-1, -1],
    ]);
  });

  it("Chinese 去 writes 土 before 厶, keeping all five source medians", () => {
    const letter = zh("去");
    expectPinnedSource(letter, "%E5%8E%BB", [4, 4, 6, 6, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [1, 0],
      [1, -1],
      [1, -1],
    ]);
  });

  it("Chinese 车 draws the vertical last, keeping all four source medians", () => {
    const letter = zh("车");
    expectPinnedSource(letter, "%E8%BD%A6", [6, 4, 9, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [1, -1],
      [1, 0],
      [0, -1],
    ]);
  });

  it("Chinese 天 writes the upper horizontal before 大, keeping all four source medians", () => {
    const letter = zh("天");
    expectPinnedSource(letter, "%E5%A4%A9", [6, 5, 6, 5]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [1, 0],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 1, 4, zh("大"), 0);
  });

  it("Chinese 今 writes the roof as 人 before the dot and the turning stroke, keeping all four source medians", () => {
    const letter = zh("今");
    expectPinnedSource(letter, "%E4%BB%8A", [5, 4, 3, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [1, -1],
      [1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 2, zh("人"), 0);
  });

  it("Chinese 星 closes 日 before 生, keeping all nine source medians", () => {
    const letter = zh("星");
    expectPinnedSource(letter, "%E6%98%9F", [2, 3, 2, 2, 5, 3, 4, 5, 6]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
      [-1, -1],
      [1, 0],
      [1, 0],
      [0, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 4, zh("日"), 0);
    expectSameHeadings(letter, 4, 9, zh("生"), 0);
    expectAbove(letter.strokes.slice(0, 4), letter.strokes.slice(4));
  });

  it("Chinese 期 writes 其 before 月, keeping all twelve source medians", () => {
    const letter = zh("期");
    expectPinnedSource(letter, "%E6%9C%9F", [2, 2, 2, 2, 2, 2, 2, 2, 4, 6, 2, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [0, -1],
      [1, 0],
      [1, 0],
      [1, 0],
      [-1, -1],
      [1, -1],
      [0, -1],
      [0, -1],
      [1, 0],
      [1, 0],
    ]);
    expectSameHeadings(letter, 8, 12, zh("明"), 4);
    expectLeftOf(letter.strokes.slice(0, 8), letter.strokes.slice(8));
  });

  it("Chinese 年 draws the long vertical last, keeping all six source medians", () => {
    const letter = zh("年");
    expectPinnedSource(letter, "%E5%B9%B4", [5, 4, 2, 2, 2, 5]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [1, 0],
      [1, 0],
      [0, -1],
      [1, 0],
      [0, -1],
    ]);
    expectSameHeadings(letter, 0, 3, zh("午"), 0);
  });

  it("Chinese 明 closes 日 before 月, keeping all eight source medians", () => {
    const letter = zh("明");
    expectPinnedSource(letter, "%E6%98%8E", [5, 5, 2, 2, 8, 10, 2, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
      [0, -1],
      [0, -1],
      [1, 0],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 4, zh("日"), 0);
  });

  it("Chinese 点 writes 占 before the four dots, keeping all nine source medians", () => {
    const letter = zh("点");
    expectPinnedSource(letter, "%E7%82%B9", [2, 2, 2, 3, 2, 2, 3, 2, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, 0],
      [0, -1],
      [1, 0],
      [1, 0],
      [-1, -1],
      [0, -1],
      [1, -1],
      [1, -1],
    ]);
    expectAbove(letter.strokes.slice(0, 5), letter.strokes.slice(5));
  });

  it("Chinese 午 draws the vertical last, after 年's first three strokes, keeping all four source medians", () => {
    const letter = zh("午");
    expectPinnedSource(letter, "%E5%8D%88", [5, 5, 6, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [1, 0],
      [1, 0],
      [0, -1],
    ]);
    expectSameHeadings(letter, 0, 3, zh("年"), 0);
  });

  it("Chinese 晚 closes 日 before 免, which ends with 儿, keeping all eleven source medians", () => {
    const letter = zh("晚");
    expectPinnedSource(letter, "%E6%99%9A", [5, 5, 2, 2, 5, 4, 2, 3, 2, 6, 9]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [0, -1],
      [1, 0],
      [1, 0],
      [-1, -1],
      [1, -1],
      [0, -1],
      [1, -1],
      [1, 0],
      [-1, -1],
      [1, -1],
    ]);
  });

  it("Chinese 手 draws the hooked vertical last, keeping all four source medians", () => {
    const letter = zh("手");
    expectPinnedSource(letter, "%E6%89%8B", [3, 5, 7, 7]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, 0],
      [1, 0],
      [1, 0],
      [0, -1],
    ]);
  });

  it("Chinese 说 writes 讠 before 兑, whose 口 and 儿 head as they do alone, keeping all nine source medians", () => {
    const letter = zh("说");
    expectPinnedSource(letter, "%E8%AF%B4", [3, 6, 2, 3, 2, 3, 2, 6, 8]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, -1],
      [1, -1],
      [-1, -1],
      [0, -1],
      [1, -1],
      [1, 0],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 2, zh("讠"), 0);
    expectSameHeadings(letter, 4, 7, zh("口"), 0);
  });

  it("Chinese 话 writes 讠 before 舌, closing its 口 last, keeping all eight source medians", () => {
    const letter = zh("话");
    expectPinnedSource(letter, "%E8%AF%9D", [2, 5, 2, 2, 2, 2, 3, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, -1],
      [-1, 0],
      [1, 0],
      [0, -1],
      [0, -1],
      [1, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 2, zh("讠"), 0);
    expectSameHeadings(letter, 5, 8, zh("口"), 0);
    expectLeftOf(letter.strokes.slice(0, 2), letter.strokes.slice(2));
  });

  it("Chinese 想 writes 木 and 目 before 心, keeping all thirteen source medians", () => {
    const letter = zh("想");
    expectPinnedSource(letter, "%E6%83%B3", [3, 4, 4, 3, 3, 6, 4, 3, 2, 3, 7, 3, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [-1, -1],
      [1, -1],
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
      [1, 0],
      [0, -1],
      [1, 0],
      [1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 4, zh("机"), 0);
    expectAbove(letter.strokes.slice(0, 9), letter.strokes.slice(9));
  });

  it("Chinese 机 writes 木 before 几, keeping all six source medians", () => {
    const letter = zh("机");
    expectPinnedSource(letter, "%E6%9C%BA", [3, 3, 4, 2, 4, 8]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [-1, -1],
      [1, -1],
      [0, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 4, zh("想"), 0);
  });

  it("Chinese 白 draws the short top stroke before 日, keeping all five source medians", () => {
    const letter = zh("白");
    expectPinnedSource(letter, "%E7%99%BD", [3, 2, 3, 2, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
    ]);
    expectSameHeadings(letter, 1, 5, zh("日"), 0);
  });

  it("Chinese 长 draws the vertical after the horizontal it crosses, keeping all four source medians", () => {
    const letter = zh("长");
    expectPinnedSource(letter, "%E9%95%BF", [4, 8, 5, 6]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [1, 0],
      [0, -1],
      [1, -1],
    ]);
  });

  it("Chinese 快 writes 忄 before 夬, keeping all seven source medians", () => {
    const letter = zh("快");
    expectPinnedSource(letter, "%E5%BF%AB", [3, 2, 3, 3, 6, 6, 6]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [0, -1],
      [1, -1],
      [1, 0],
      [0, -1],
      [1, -1],
    ]);
  });

  it("Chinese 电 closes 日 before the stroke through its middle, keeping all five source medians", () => {
    const letter = zh("电");
    expectPinnedSource(letter, "%E7%94%B5", [2, 3, 2, 2, 11]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 4, zh("日"), 0);
  });

  it("Chinese 打 writes 扌 before 丁, keeping all five source medians", () => {
    const letter = zh("打");
    expectPinnedSource(letter, "%E6%89%93", [2, 4, 3, 2, 5]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [1, 0],
      [1, 0],
      [0, -1],
    ]);
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 起 writes 走, its long sweep included, before 己, keeping all ten source medians", () => {
    const letter = zh("起");
    expectPinnedSource(letter, "%E8%B5%B7", [2, 2, 2, 2, 2, 3, 5, 3, 2, 6]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, 0],
      [1, -1],
      [1, 0],
      [1, -1],
    ]);
  });

  it("Chinese 门 draws the three strokes that close 们, keeping all three source medians", () => {
    const letter = zh("门");
    expectPinnedSource(letter, "%E9%97%A8", [3, 3, 9]);
    expect(letter.strokes.map(heading)).toEqual([[1, -1], [0, -1], [0, -1]]);
    expectSameHeadings(letter, 0, 3, zh("们"), 2);
  });

  it("Chinese 问 writes 门 before the 口 inside it, keeping all six source medians", () => {
    const letter = zh("问");
    expectPinnedSource(letter, "%E9%97%AE", [2, 3, 8, 4, 6, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [0, -1],
      [1, -1],
      [0, -1],
      [1, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 3, 6, zh("口"), 0);
  });

  it("Chinese 气 draws the turning last stroke after the three short ones, keeping all four source medians", () => {
    const letter = zh("气");
    expectPinnedSource(letter, "%E6%B0%94", [5, 2, 2, 8]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [1, 0],
      [1, 0],
      [1, -1],
    ]);
  });

  it("Chinese 雨 draws the frame and the middle vertical before the four dots, keeping all eight source medians", () => {
    const letter = zh("雨");
    expectPinnedSource(letter, "%E9%9B%A8", [6, 5, 11, 4, 3, 3, 2, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [1, -1],
      [0, -1],
      [1, -1],
      [1, -1],
      [1, -1],
      [1, -1],
    ]);
  });

  it("Chinese 马 draws the three strokes that close 吗 and 妈, keeping all three source medians", () => {
    const letter = zh("马");
    expectPinnedSource(letter, "%E9%A9%AC", [6, 12, 5]);
    expect(letter.strokes.map(heading)).toEqual([[1, -1], [1, -1], [1, 0]]);
  });

  it("Chinese 山 draws the middle peak first, keeping all three source medians", () => {
    const letter = zh("山");
    expectPinnedSource(letter, "%E5%B1%B1", [2, 3, 2]);
    expect(letter.strokes.map(heading)).toEqual([[0, -1], [1, -1], [0, -1]]);
  });

  it("Chinese 做 writes 亻, then 古, then 攵, keeping all eleven source medians", () => {
    const letter = zh("做");
    expectPinnedSource(letter, "%E5%81%9A", [5, 3, 5, 4, 2, 3, 2, 6, 3, 4, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [0, -1],
      [1, 0],
      [0, -1],
      [0, -1],
      [1, -1],
      [1, 0],
      [0, -1],
      [1, 0],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 2, zh("亻"), 0);
    expectSameHeadings(letter, 4, 7, zh("口"), 0);
    expectLeftOf(letter.strokes.slice(0, 2), letter.strokes.slice(2));
  });

  it("Chinese 火 draws the two short side strokes before 人, keeping all four source medians", () => {
    const letter = zh("火");
    expectPinnedSource(letter, "%E7%81%AB", [3, 3, 9, 6]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [-1, -1],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 2, 4, zh("人"), 0);
  });

  it("Chinese 开 draws both horizontals before both legs, keeping all four source medians", () => {
    const letter = zh("开");
    expectPinnedSource(letter, "%E5%BC%80", [3, 8, 8, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [1, 0],
      [0, -1],
      [0, -1],
    ]);
  });

  it("Chinese 下 draws the horizontal, then the vertical, then the dot, keeping all three source medians", () => {
    const letter = zh("下");
    expectPinnedSource(letter, "%E4%B8%8B", [6, 3, 2]);
    expect(letter.strokes.map(heading)).toEqual([[1, 0], [0, -1], [1, -1]]);
  });

  it("Chinese 进 writes 井 before the walking piece 辶, keeping all seven source medians", () => {
    const letter = zh("进");
    expectPinnedSource(letter, "%E8%BF%9B", [2, 2, 5, 3, 3, 5, 8]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [1, 0],
      [0, -1],
      [0, -1],
      [1, -1],
      [0, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 4, 7, zh("这"), 4);
  });

  it("Chinese 客 writes 宀 before 各, closing its 口 last, keeping all nine source medians", () => {
    const letter = zh("客");
    expectPinnedSource(letter, "%E5%AE%A2", [3, 3, 3, 4, 7, 6, 2, 3, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [0, -1],
      [1, 0],
      [-1, -1],
      [-1, -1],
      [1, 0],
      [0, -1],
      [1, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 3, zh("宀"), 0);
    expectSameHeadings(letter, 6, 9, zh("口"), 0);
  });

  it("Chinese 乐 writes the top two strokes before 小, keeping all five source medians", () => {
    const letter = zh("乐");
    expectPinnedSource(letter, "%E4%B9%90", [6, 7, 6, 3, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, 0],
      [1, -1],
      [0, -1],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 2, 5, zh("小"), 0);
  });

  it("Chinese 会 writes the roof as 今 does and 厶 as 去 does, keeping all six source medians", () => {
    const letter = zh("会");
    expectPinnedSource(letter, "%E4%BC%9A", [5, 5, 2, 2, 4, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [1, -1],
      [1, 0],
      [1, 0],
      [1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 2, zh("今"), 0);
    expectSameHeadings(letter, 4, 6, zh("去"), 3);
  });

  it("Chinese 课 writes 讠 before 果, closing 田 before 木's vertical, keeping all ten source medians", () => {
    const letter = zh("课");
    expectPinnedSource(letter, "%E8%AF%BE", [3, 5, 2, 3, 2, 2, 2, 2, 3, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, -1],
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
      [1, 0],
      [0, -1],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 2, zh("讠"), 0);
    expectSameHeadings(letter, 2, 6, zh("日"), 0);
    expectLeftOf(letter.strokes.slice(0, 2), letter.strokes.slice(2));
  });

  it("Chinese 里 closes 田 before the long vertical and 土, keeping all seven source medians", () => {
    const letter = zh("里");
    expectPinnedSource(letter, "%E9%87%8C", [2, 3, 2, 2, 2, 2, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
      [0, -1],
      [1, 0],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 4, zh("日"), 0);
  });

  it("Chinese 前 writes the top, then 月, then 刂, keeping all nine source medians", () => {
    const letter = zh("前");
    expectPinnedSource(letter, "%E5%89%8D", [4, 5, 2, 2, 5, 2, 2, 2, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [-1, -1],
      [1, 0],
      [0, -1],
      [0, -1],
      [1, 0],
      [1, 0],
      [0, -1],
      [0, -1],
    ]);
    expectSameHeadings(letter, 4, 7, zh("明"), 5);
  });
});

// The Chinese A1 core (chapters 85-97) writes thirteen more characters, one
// a chapter, each right after the word that anchors it. Each case pins the
// same things the stage-2 cases do, with the same shared checks; a piece is
// compared with the character the reader wrote it in only where Noto's
// shapes let the slice head as the piece does alone (哪's narrow 口, 没's
// steep rising stroke, 两's upright little 人 and 时's tall 日 do not, so
// those comparisons are left out rather than loosened).
describe("A1 core handwriting ductus", () => {
  it("covers every character the A1 core writes, each once", () => {
    const core = [...A1_CORE];
    expect(core).toHaveLength(13);
    expect(new Set(core).size).toBe(13);
    expect(core.every((glyph) => zh(glyph) !== undefined)).toBe(true);
    expect(core.some((glyph) => STAGE_TWO.includes(glyph))).toBe(false);
  });

  it("Chinese 在 opens as 有 does, then writes 土, keeping all six source medians", () => {
    const letter = zh("在");
    expectPinnedSource(letter, "%E5%9C%A8", [2, 4, 4, 2, 4, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [-1, -1],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 2, zh("有"), 0);
  });

  it("Chinese 哪 closes 口 before writing the whole of 那, keeping all nine source medians", () => {
    const letter = zh("哪");
    expectPinnedSource(letter, "%E5%93%AA", [2, 3, 2, 5, 2, 2, 5, 7, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, 0],
      [1, 0],
      [0, -1],
      [0, -1],
      [0, -1],
    ]);
    expectSameHeadings(letter, 3, 9, zh("那"), 0);
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 没 writes 氵, then the small roof, then 又, keeping all seven source medians", () => {
    const letter = zh("没");
    expectPinnedSource(letter, "%E6%B2%A1", [3, 3, 3, 6, 10, 5, 7]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, -1],
      [0, 1],
      [0, -1],
      [1, -1],
      [0, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 5, 7, zh("对"), 0);
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 两 draws the frame before the two 人 inside it, keeping all seven source medians", () => {
    const letter = zh("两");
    expectPinnedSource(letter, "%E4%B8%A4", [6, 5, 14, 5, 3, 7, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [1, -1],
      [0, -1],
      [1, -1],
      [0, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 3, zh("雨"), 0);
  });

  it("Chinese 给 writes 纟 before 合, closing its 口 last, keeping all nine source medians", () => {
    const letter = zh("给");
    expectPinnedSource(letter, "%E7%BB%99", [5, 5, 5, 3, 2, 2, 2, 3, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [0, -1],
      [1, 0],
      [-1, -1],
      [1, -1],
      [1, 0],
      [0, -1],
      [1, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 3, 5, zh("人"), 0);
    expectSameHeadings(letter, 6, 9, zh("口"), 0);
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 能 writes 厶 and 月 before the two 匕, keeping all ten source medians", () => {
    const letter = zh("能");
    expectPinnedSource(letter, "%E8%83%BD", [5, 3, 4, 8, 3, 4, 4, 10, 3, 11]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, -1],
      [0, -1],
      [0, -1],
      [1, 0],
      [1, 0],
      [-1, -1],
      [1, -1],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 2, 6, zh("明"), 4);
    expectLeftOf(letter.strokes.slice(0, 6), letter.strokes.slice(6));
  });

  it("Chinese 要 closes 覀's box before 女, keeping all nine source medians", () => {
    const letter = zh("要");
    expectPinnedSource(letter, "%E8%A6%81", [2, 2, 3, 2, 2, 2, 7, 6, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [1, 0],
      [0, -1],
      [0, -1],
      [1, 0],
      [1, -1],
      [-1, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 6, 9, zh("女"), 0);
    expectAbove(letter.strokes.slice(0, 6), letter.strokes.slice(6));
  });

  it("Chinese 也 draws the three strokes that close 他, keeping all three source medians", () => {
    const letter = zh("也");
    expectPinnedSource(letter, "%E4%B9%9F", [8, 3, 8]);
    expect(letter.strokes.map(heading)).toEqual([[1, 0], [0, -1], [1, -1]]);
    expectSameHeadings(letter, 0, 3, zh("他"), 2);
  });

  it("Chinese 太 writes 大 before its dot, keeping all four source medians", () => {
    const letter = zh("太");
    expectPinnedSource(letter, "%E5%A4%AA", [6, 10, 7, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [-1, -1],
      [1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 3, zh("大"), 0);
  });

  it("Chinese 都 writes 者, closing its 日, before 阝, keeping all ten source medians", () => {
    const letter = zh("都");
    expectPinnedSource(letter, "%E9%83%BD", [2, 2, 2, 6, 2, 3, 2, 2, 9, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [1, 0],
      [-1, -1],
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
      [0, -1],
      [0, -1],
    ]);
    expectSameHeadings(letter, 4, 8, zh("日"), 0);
    expectSameHeadings(letter, 8, 10, zh("那"), 4);
    expectLeftOf(letter.strokes.slice(0, 8), letter.strokes.slice(8));
  });

  it("Chinese 为 draws its hooked stroke as 男's 力 does, keeping all four source medians", () => {
    const letter = zh("为");
    expectPinnedSource(letter, "%E4%B8%BA", [2, 6, 7, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [0, -1],
      [1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 2, 3, zh("男"), 5);
  });

  it("Chinese 因 opens the enclosure as 国 does and closes it after 大, keeping all six source medians", () => {
    const letter = zh("因");
    expectPinnedSource(letter, "%E5%9B%A0", [4, 9, 3, 6, 3, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [1, 0],
      [-1, -1],
      [1, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 2, zh("国"), 0);
    expectSameHeadings(letter, 2, 5, zh("大"), 0);
  });

  it("Chinese 时 writes 日 before 寸, as 对 ends, keeping all seven source medians", () => {
    const letter = zh("时");
    expectPinnedSource(letter, "%E6%97%B6", [5, 5, 3, 3, 4, 6, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [0, -1],
      [1, 0],
      [1, 0],
      [1, 0],
      [0, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 4, 7, zh("对"), 2);
    expectLeftOf(letter.strokes.slice(0, 4), letter.strokes.slice(4));
  });
});

// A1 vocabulary writes twenty-four more characters, tranche c (chapters
// 99-119) then tranche d (chapters 126-140), at most one a chapter, each
// right after the word that anchors it. Each
// case pins the same things the stage-2 and A1-core cases do, with the same
// shared checks. A piece is compared with the character the reader wrote it
// in only where Noto's shapes let the slice head as the piece does there:
// 洗's 儿 heads as 说's and 晚's do (not as 儿 alone), 边's 辶 as 这's and
// 进's, 网's frame as 两's. 用's hooked frame, 站's 占, 外's 夕, 边's 力,
// 出's two halves and 心 against 想's foot head differently in Noto, as do
// 音's 立 against 站's, 吵's 口 and 少 against the characters alone, and
// 节's 艹 against 菜's; those comparisons are left out rather than
// loosened.
describe("A1 vocabulary handwriting ductus", () => {
  it("covers every character A1 vocabulary writes, each once", () => {
    const vocabulary = [...A1_VOCABULARY];
    expect(vocabulary).toHaveLength(24);
    expect(new Set(vocabulary).size).toBe(24);
    expect(vocabulary.every((glyph) => zh(glyph) !== undefined)).toBe(true);
    expect(
      vocabulary.some(
        (glyph) => STAGE_TWO.includes(glyph) || A1_CORE.includes(glyph),
      ),
    ).toBe(false);
  });

  it("Chinese 洗 writes 氵 before 先, which ends with 儿, keeping all nine source medians", () => {
    const letter = zh("洗");
    expectPinnedSource(letter, "%E6%B4%97", [3, 3, 4, 3, 5, 4, 5, 7, 9]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, -1],
      [0, 1],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, 0],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 3, zh("没"), 0);
    expectSameHeadings(letter, 7, 9, zh("说"), 7);
    expectSameHeadings(letter, 7, 9, zh("晚"), 9);
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 菜 writes 艹, then 爫, then 木, keeping all eleven source medians", () => {
    const letter = zh("菜");
    expectPinnedSource(letter, "%E8%8F%9C", [6, 4, 5, 4, 2, 2, 3, 4, 4, 4, 5]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [1, -1],
      [0, -1],
      [-1, 0],
      [1, -1],
      [1, -1],
      [-1, -1],
      [1, 0],
      [0, -1],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 7, 11, zh("本"), 0);
    expectAbove(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 用 draws 月's frame before its bars and middle vertical, keeping all five source medians", () => {
    const letter = zh("用");
    expectPinnedSource(letter, "%E7%94%A8", [6, 10, 4, 4, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
      [0, -1],
    ]);
  });

  it("Chinese 本 writes 木 before the short horizontal across its foot, keeping all five source medians", () => {
    const letter = zh("本");
    expectPinnedSource(letter, "%E6%9C%AC", [7, 3, 5, 4, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [-1, -1],
      [1, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 4, zh("菜"), 7);
  });

  it("Chinese 站 writes 立 before 占, as 点 opens, keeping all ten source medians", () => {
    const letter = zh("站");
    expectPinnedSource(letter, "%E7%AB%99", [3, 3, 2, 4, 5, 4, 3, 4, 7, 5]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, 0],
      [0, -1],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 7, 10, zh("口"), 0);
    expectLeftOf(letter.strokes.slice(0, 5), letter.strokes.slice(5));
  });

  it("Chinese 试 writes 讠 before 式, its dot last, keeping all eight source medians", () => {
    const letter = zh("试");
    expectPinnedSource(letter, "%E8%AF%95", [2, 6, 2, 3, 2, 6, 9, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, -1],
      [1, 0],
      [1, 0],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 2, zh("讠"), 0);
    expectLeftOf(letter.strokes.slice(0, 2), letter.strokes.slice(2));
  });

  it("Chinese 网 draws the frame before the two crosses inside it, keeping all six source medians", () => {
    const letter = zh("网");
    expectPinnedSource(letter, "%E7%BD%91", [5, 12, 5, 3, 5, 5]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [-1, -1],
      [1, -1],
      [0, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 2, zh("两"), 1);
  });

  it("Chinese 外 writes 夕 as 多 opens, then 卜, keeping all five source medians", () => {
    const letter = zh("外");
    expectPinnedSource(letter, "%E5%A4%96", [5, 10, 2, 3, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [0, -1],
      [1, -1],
      [0, -1],
      [1, -1],
    ]);
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 边 writes 力 before the walking piece 辶, keeping all five source medians", () => {
    const letter = zh("边");
    expectPinnedSource(letter, "%E8%BE%B9", [7, 5, 3, 5, 9]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [0, -1],
      [1, -1],
      [0, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 2, 5, zh("这"), 4);
    expectSameHeadings(letter, 2, 5, zh("进"), 4);
  });

  it("Chinese 出 draws the upper half, the middle vertical, then the lower half, keeping all five source medians", () => {
    const letter = zh("出");
    expectPinnedSource(letter, "%E5%87%BA", [3, 2, 4, 9, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [0, -1],
      [0, -1],
      [1, 0],
      [0, -1],
    ]);
  });

  it("Chinese 心 draws the four strokes at the foot of 想, keeping all four source medians", () => {
    const letter = zh("心");
    expectPinnedSource(letter, "%E5%BF%83", [3, 8, 2, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [1, -1],
      [0, -1],
    ]);
  });

  it("Chinese 包 writes 勹 before 巳, ending as 也 does, keeping all five source medians", () => {
    const letter = zh("包");
    expectPinnedSource(letter, "%E5%8C%85", [7, 7, 3, 2, 8]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [1, -1],
      [1, -1],
      [1, 0],
      [1, -1],
    ]);
    expectSameHeadings(letter, 4, 5, zh("也"), 2);
  });

  it("Chinese 以 writes its left piece before its right, keeping all four source medians", () => {
    const letter = zh("以");
    expectPinnedSource(letter, "%E4%BB%A5", [6, 3, 8, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, -1],
      [-1, -1],
      [1, -1],
    ]);
  });

  it("Chinese 些 writes 此 before 二, keeping all eight source medians", () => {
    const letter = zh("些");
    expectPinnedSource(letter, "%E4%BA%9B", [5, 4, 5, 4, 5, 9, 4, 5]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, 0],
      [0, -1],
      [1, 0],
      [-1, -1],
      [1, -1],
      [1, 0],
      [1, 0],
    ]);
    expectSameHeadings(letter, 4, 6, zh("能"), 6);
    expectSameHeadings(letter, 4, 6, zh("能"), 8);
    expectSameHeadings(letter, 6, 8, zh("二"), 0);
    expectAbove(letter.strokes.slice(0, 6), letter.strokes.slice(6));
  });

  it("Chinese 音 writes 立 before 日, keeping all nine source medians", () => {
    const letter = zh("音");
    expectPinnedSource(letter, "%E9%9F%B3", [3, 3, 2, 2, 5, 2, 3, 2, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, 0],
      [0, -1],
      [-1, -1],
      [1, 0],
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
    ]);
    expectSameHeadings(letter, 5, 9, zh("日"), 0);
    expectAbove(letter.strokes.slice(0, 5), letter.strokes.slice(5));
  });

  it("Chinese 吵 closes 口 before writing 少, keeping all seven source medians", () => {
    const letter = zh("吵");
    expectPinnedSource(letter, "%E5%90%B5", [2, 3, 2, 2, 3, 2, 6]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [0, -1],
      [1, 0],
      [0, -1],
      [0, -1],
      [1, -1],
      [-1, -1],
    ]);
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 病 writes 疒 before 丙, its small 人 last, keeping all ten source medians", () => {
    const letter = zh("病");
    expectPinnedSource(letter, "%E7%97%85", [2, 3, 4, 2, 4, 4, 2, 5, 6, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, 0],
      [0, -1],
      [0, -1],
      [1, 1],
      [1, 0],
      [0, -1],
      [1, -1],
      [0, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 6, 8, zh("两"), 1);
    expectSameHeadings(letter, 8, 10, zh("两"), 3);
  });

  it("Chinese 春 draws 三's bars and 人 before 日, keeping all nine source medians", () => {
    const letter = zh("春");
    expectPinnedSource(letter, "%E6%98%A5", [2, 2, 2, 5, 5, 2, 3, 2, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [1, 0],
      [1, 0],
      [-1, -1],
      [1, -1],
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 3, zh("三"), 0);
    expectSameHeadings(letter, 3, 5, zh("人"), 0);
    expectSameHeadings(letter, 5, 9, zh("日"), 0);
  });

  it("Chinese 千 draws its top stroke before 十, keeping all three source medians", () => {
    const letter = zh("千");
    expectPinnedSource(letter, "%E5%8D%83", [3, 6, 5]);
    expect(letter.strokes.map(heading)).toEqual([[-1, 0], [1, 0], [0, -1]]);
    expectSameHeadings(letter, 1, 3, zh("十"), 0);
  });

  it("Chinese 扫 writes 扌 as 打 opens, then its three bars, keeping all six source medians", () => {
    const letter = zh("扫");
    expectPinnedSource(letter, "%E6%89%AB", [2, 5, 2, 3, 2, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [1, 0],
      [1, -1],
      [1, 0],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 3, zh("打"), 0);
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 重 writes 千's top, then 里, keeping all nine source medians", () => {
    const letter = zh("重");
    expectPinnedSource(letter, "%E9%87%8D", [6, 6, 2, 3, 2, 2, 4, 2, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, 0],
      [1, 0],
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
      [0, -1],
      [1, 0],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 2, zh("千"), 0);
    expectSameHeadings(letter, 2, 9, zh("里"), 0);
  });

  it("Chinese 节 writes 艹 before its hooked stroke and long vertical, keeping all five source medians", () => {
    const letter = zh("节");
    expectPinnedSource(letter, "%E8%8A%82", [6, 5, 5, 6, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [0, -1],
      [1, -1],
      [0, -1],
    ]);
    expectAbove(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 每 closes its box's two turns before the dots and the long bar, keeping all seven source medians", () => {
    const letter = zh("每");
    expectPinnedSource(letter, "%E6%AF%8F", [6, 2, 3, 6, 2, 2, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [1, 0],
      [1, -1],
      [1, -1],
      [1, -1],
      [1, 0],
      [1, -1],
    ]);
  });

  it("Chinese 半 draws the long vertical last, as 十 and 千 do, keeping all five source medians", () => {
    const letter = zh("半");
    expectPinnedSource(letter, "%E5%8D%8A", [3, 2, 2, 2, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [-1, -1],
      [1, 0],
      [1, 0],
      [0, -1],
    ]);
    expectSameHeadings(letter, 4, 5, zh("十"), 1);
    expectSameHeadings(letter, 4, 5, zh("千"), 2);
  });
});

// The Chinese A2 core (chapters 147-156) writes ten more characters, one a
// chapter, each right after the word that anchors it. Each case pins the
// same things the earlier blocks do, with the same shared checks. A piece is
// compared with the character the reader wrote it in only where Noto's
// shapes let the slice head as the piece does there: 月 alone heads as it
// does in 明, 期 and 前; 得's 彳 as 很's; 寸 as in 对 and 时; 匕 as in 能
// and 些; 元's 儿 as 说's. 过's 辶 against 这's, 从's 人 against 人 alone,
// and 元's 儿 against 儿 alone head differently in Noto, and 从's two 人 and
// 元's 二 and 儿 overlap, so those checks are left out rather than
// loosened.
describe("A2 core handwriting ductus", () => {
  it("covers every character the A2 core writes, each once", () => {
    const core = [...A2_CORE];
    expect(core).toHaveLength(10);
    expect(new Set(core).size).toBe(10);
    expect(core.every((glyph) => zh(glyph) !== undefined)).toBe(true);
    expect(
      core.some(
        (glyph) =>
          STAGE_TWO.includes(glyph) ||
          A1_CORE.includes(glyph) ||
          A1_VOCABULARY.includes(glyph),
      ),
    ).toBe(false);
  });

  it("Chinese 后 draws its frame before the 口 inside it, keeping all six source medians", () => {
    const letter = zh("后");
    expectPinnedSource(letter, "%E5%90%8E", [5, 5, 5, 2, 3, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, 0],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 3, 6, zh("口"), 0);
  });

  it("Chinese 月 draws the four strokes it has in 明 and 期, keeping all four source medians", () => {
    const letter = zh("月");
    expectPinnedSource(letter, "%E6%9C%88", [5, 11, 3, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [0, -1],
      [1, 0],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 4, zh("明"), 4);
    expectSameHeadings(letter, 0, 4, zh("期"), 8);
    expectSameHeadings(letter, 0, 4, zh("前"), 3);
  });

  it("Chinese 正 draws the long bottom horizontal last, keeping all five source medians", () => {
    const letter = zh("正");
    expectPinnedSource(letter, "%E6%AD%A3", [5, 4, 5, 4, 6]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 1, 3, zh("上"), 0);
  });

  it("Chinese 过 writes 寸 before the walking piece 辶, keeping all six source medians", () => {
    const letter = zh("过");
    expectPinnedSource(letter, "%E8%BF%87", [6, 5, 3, 3, 7, 8]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [1, -1],
      [1, -1],
      [1, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 3, zh("对"), 2);
    expectSameHeadings(letter, 0, 3, zh("时"), 4);
  });

  it("Chinese 到 writes 至 before the knife piece 刂, keeping all eight source medians", () => {
    const letter = zh("到");
    expectPinnedSource(letter, "%E5%88%B0", [4, 8, 3, 3, 4, 5, 4, 7]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [1, -1],
      [1, -1],
      [1, 0],
      [0, -1],
      [1, 0],
      [0, -1],
      [0, -1],
    ]);
    expectSameHeadings(letter, 6, 8, zh("前"), 7);
    expectLeftOf(letter.strokes.slice(0, 6), letter.strokes.slice(6));
  });

  it("Chinese 得 writes 彳 as 很 opens, then 日 and 寸, keeping all eleven source medians", () => {
    const letter = zh("得");
    expectPinnedSource(letter, "%E5%BE%97", [5, 4, 2, 2, 3, 2, 2, 2, 2, 7, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [-1, -1],
      [0, -1],
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
      [1, 0],
      [1, 0],
      [0, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 3, zh("很"), 0);
    expectSameHeadings(letter, 3, 7, zh("日"), 0);
    expectSameHeadings(letter, 8, 11, zh("对"), 2);
    expectSameHeadings(letter, 8, 11, zh("时"), 4);
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 比 writes the left 匕 before the right one, keeping all four source medians", () => {
    const letter = zh("比");
    expectPinnedSource(letter, "%E6%AF%94", [4, 5, 4, 12]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 2, 4, zh("能"), 6);
    expectSameHeadings(letter, 2, 4, zh("些"), 4);
    expectLeftOf(letter.strokes.slice(0, 2), letter.strokes.slice(2));
  });

  it("Chinese 还 writes 不 before the walking piece 辶, keeping all seven source medians", () => {
    const letter = zh("还");
    expectPinnedSource(letter, "%E8%BF%98", [6, 7, 3, 3, 3, 9, 7]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [-1, -1],
      [0, -1],
      [1, -1],
      [1, -1],
      [0, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 4, zh("不"), 0);
    expectSameHeadings(letter, 4, 7, zh("这"), 4);
  });

  it("Chinese 从 writes the narrower left 人 before the right one, keeping all four source medians", () => {
    const letter = zh("从");
    expectPinnedSource(letter, "%E4%BB%8E", [6, 4, 6, 5]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [0, -1],
      [1, -1],
    ]);
  });

  it("Chinese 元 writes 二 before 儿, keeping all four source medians", () => {
    const letter = zh("元");
    expectPinnedSource(letter, "%E5%85%83", [3, 4, 6, 11]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [1, 0],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 2, zh("二"), 0);
    expectSameHeadings(letter, 2, 4, zh("说"), 7);
  });
});

// Chinese A2 vocabulary writes twenty-four more characters, twelve in
// tranche e (chapters 158-187) and twelve in tranche f (chapters 188-217),
// each right after the word that anchors it. Each case pins the same things
// the earlier blocks do, with the same shared checks. A piece is compared
// with the character the reader wrote it in only where Noto's shapes let the
// slice head as the piece does there: 文's dot and horizontal in 衣 and 离;
// 人, the roof of 今, in 茶 and 冷; 见 in 觉; 机's tree piece and 不 in 杯;
// 亻 in 候; 常's cover as 觉's; 问's 门 and 日 in 间; 占's first two strokes
// as 点's, and 口, in 店; 国's frame and 口 in 回; both 口 in 路; 二 in 票;
// 打's hand piece in 找; 天 in 关; 车's turning stroke in 东; the opening of
// 能 in 公; 在's 土 in 地. Noto's wide 口 in 事, 常 and 号, the covers of 觉
// and 常 against 学's, 间's 门 against 门 alone, 茶's 艹 against 菜's, 店's
// 口 inside 点's 占, 路's right side against 客's, 公's 八 against 八 alone
// and 地's 也 against 也 and 他 head differently, and 茶's 艹, 杯's two
// pieces and 关's top strokes overlap what follows them, so those checks are
// left out rather than loosened.
describe("A2 vocabulary handwriting ductus", () => {
  it("covers every character A2 vocabulary writes, each once", () => {
    const vocabulary = [...A2_VOCABULARY];
    expect(vocabulary).toHaveLength(24);
    expect(new Set(vocabulary).size).toBe(24);
    expect(vocabulary.every((glyph) => zh(glyph) !== undefined)).toBe(true);
    expect(
      vocabulary.some(
        (glyph) =>
          STAGE_TWO.includes(glyph) ||
          A1_CORE.includes(glyph) ||
          A1_VOCABULARY.includes(glyph) ||
          A2_CORE.includes(glyph),
      ),
    ).toBe(false);
  });

  it("Chinese 房 writes 户 before 方, keeping all eight source medians", () => {
    const letter = zh("房");
    expectPinnedSource(letter, "%E6%88%BF", [3, 3, 2, 4, 3, 5, 6, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, 0],
      [1, 0],
      [0, -1],
      [1, -1],
      [1, 0],
      [1, -1],
      [-1, -1],
    ]);
  });

  it("Chinese 米 draws the two short strokes before the cross and its legs, keeping all six source medians", () => {
    const letter = zh("米");
    expectPinnedSource(letter, "%E7%B1%B3", [3, 3, 6, 3, 4, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [-1, -1],
      [1, 0],
      [0, -1],
      [-1, -1],
      [1, -1],
    ]);
  });

  it("Chinese 衣 draws a dot and a horizontal before its body of four, keeping all six source medians", () => {
    const letter = zh("衣");
    expectPinnedSource(letter, "%E8%A1%A3", [3, 6, 4, 4, 5, 7]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, 0],
      [-1, -1],
      [1, -1],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 2, zh("文"), 0);
    expectSameHeadings(letter, 0, 2, zh("这"), 0);
  });

  it("Chinese 茶 writes 艹, then 人, then the four strokes at the foot, keeping all nine source medians", () => {
    const letter = zh("茶");
    expectPinnedSource(letter, "%E8%8C%B6", [2, 2, 2, 5, 3, 6, 6, 3, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [0, -1],
      [-1, -1],
      [1, -1],
      [1, 0],
      [0, -1],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 3, 5, zh("人"), 0);
    expectSameHeadings(letter, 3, 5, zh("今"), 0);
  });

  it("Chinese 事 draws its long hooked vertical last, keeping all eight source medians", () => {
    const letter = zh("事");
    expectPinnedSource(letter, "%E4%BA%8B", [6, 2, 4, 2, 8, 6, 5, 6]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [1, 0],
      [1, 0],
      [1, -1],
      [1, 0],
      [1, 0],
      [0, -1],
    ]);
  });

  it("Chinese 觉 writes the three short strokes and the cover before 见, keeping all nine source medians", () => {
    const letter = zh("觉");
    expectPinnedSource(letter, "%E8%A7%89", [2, 2, 2, 2, 3, 2, 3, 7, 9]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, -1],
      [-1, -1],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, -1],
      [-1, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 5, 9, zh("见"), 0);
    expectAbove(letter.strokes.slice(0, 5), letter.strokes.slice(5));
  });

  it("Chinese 杯 writes the tree piece before 不, keeping all eight source medians", () => {
    const letter = zh("杯");
    expectPinnedSource(letter, "%E6%9D%AF", [5, 3, 4, 3, 2, 4, 2, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [-1, -1],
      [1, -1],
      [1, 0],
      [-1, -1],
      [0, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 4, zh("机"), 0);
    expectSameHeadings(letter, 4, 8, zh("不"), 0);
    expectSameHeadings(letter, 4, 8, zh("还"), 0);
  });

  it("Chinese 冷 writes the ice piece before the roof of 今, keeping all seven source medians", () => {
    const letter = zh("冷");
    expectPinnedSource(letter, "%E5%86%B7", [4, 4, 5, 3, 3, 5, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [0, 1],
      [-1, -1],
      [1, -1],
      [1, -1],
      [1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 2, 4, zh("今"), 0);
    expectSameHeadings(letter, 2, 4, zh("人"), 0);
    expectLeftOf(letter.strokes.slice(0, 2), letter.strokes.slice(2));
  });

  it("Chinese 候 writes 亻 and a thin vertical before the right side, keeping all ten source medians", () => {
    const letter = zh("候");
    expectPinnedSource(letter, "%E5%80%99", [5, 4, 3, 6, 4, 4, 3, 4, 7, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [0, -1],
      [0, -1],
      [1, -1],
      [1, 0],
      [-1, -1],
      [1, 0],
      [1, 0],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 2, zh("亻"), 0);
    expectSameHeadings(letter, 0, 2, zh("你"), 0);
    expectLeftOf(letter.strokes.slice(0, 2), letter.strokes.slice(2));
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 面 closes its frame last, keeping all nine source medians", () => {
    const letter = zh("面");
    expectPinnedSource(letter, "%E9%9D%A2", [6, 5, 5, 14, 3, 3, 3, 3, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [-1, -1],
      [0, -1],
      [1, -1],
      [0, -1],
      [0, -1],
      [1, 0],
      [1, 0],
      [1, 0],
    ]);
  });

  it("Chinese 常 writes the top, the cover of 学 and 口 before 巾, keeping all eleven source medians", () => {
    const letter = zh("常");
    expectPinnedSource(letter, "%E5%B8%B8", [2, 2, 2, 2, 3, 2, 3, 2, 2, 4, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [-1, -1],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, 0],
      [1, 0],
      [0, -1],
      [1, -1],
      [0, -1],
    ]);
    expectSameHeadings(letter, 3, 5, zh("觉"), 3);
    expectAbove(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 间 writes 门 before the 日 inside it, keeping all seven source medians", () => {
    const letter = zh("间");
    expectPinnedSource(letter, "%E9%97%B4", [3, 2, 4, 2, 3, 2, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [0, -1],
      [1, -1],
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 3, zh("问"), 0);
    expectSameHeadings(letter, 3, 7, zh("日"), 0);
  });

  it("Chinese 钱 writes the metal piece before its right side, keeping all ten source medians", () => {
    const letter = zh("钱");
    expectPinnedSource(letter, "%E9%92%B1", [3, 2, 2, 2, 3, 2, 2, 10, 5, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [1, 0],
      [1, 0],
      [1, 0],
      [0, -1],
      [1, 0],
      [1, 0],
      [0, -1],
      [-1, -1],
      [1, -1],
    ]);
    expectLeftOf(letter.strokes.slice(0, 5), letter.strokes.slice(5));
  });

  it("Chinese 号 closes 口 before the long horizontal and the turning stroke, keeping all five source medians", () => {
    const letter = zh("号");
    expectPinnedSource(letter, "%E5%8F%B7", [2, 3, 2, 2, 6]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, 0],
      [1, 0],
      [1, 0],
      [1, -1],
    ]);
    expectAbove(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 店 writes the roof, then 占's two strokes, then 口, keeping all eight source medians", () => {
    const letter = zh("店");
    expectPinnedSource(letter, "%E5%BA%97", [2, 2, 4, 2, 2, 2, 3, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, 0],
      [0, -1],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 3, 5, zh("点"), 0);
    expectSameHeadings(letter, 5, 8, zh("口"), 0);
  });

  it("Chinese 回 draws the frame, then 口 inside it, then the frame's foot, keeping all six source medians", () => {
    const letter = zh("回");
    expectPinnedSource(letter, "%E5%9B%9E", [2, 3, 2, 3, 2, 2]);
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [0, -1],
      [1, -1],
      [1, 0],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 2, zh("国"), 0);
    expectSameHeadings(letter, 2, 5, zh("口"), 0);
  });

  it("Chinese 路 writes the foot piece before the right side, each with 口, keeping all thirteen source medians", () => {
    const letter = zh("路");
    expectPinnedSource(
      letter,
      "%E8%B7%AF",
      [2, 3, 2, 2, 2, 2, 2, 4, 6, 4, 2, 3, 2],
    );
    expect(letter.strokes.map(heading)).toEqual([
      [0, -1],
      [1, -1],
      [1, 0],
      [0, -1],
      [1, 0],
      [0, -1],
      [1, 0],
      [-1, -1],
      [0, -1],
      [1, -1],
      [0, -1],
      [1, -1],
      [1, 0],
    ]);
    expectSameHeadings(letter, 0, 3, zh("口"), 0);
    expectSameHeadings(letter, 10, 13, zh("口"), 0);
    expectLeftOf(letter.strokes.slice(0, 7), letter.strokes.slice(7));
  });

  it("Chinese 票 writes the top piece before 二 and the hooked vertical, keeping all eleven source medians", () => {
    const letter = zh("票");
    expectPinnedSource(letter, "%E7%A5%A8", [2, 2, 3, 2, 2, 2, 2, 2, 5, 4, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [1, 0],
      [0, -1],
      [0, -1],
      [1, 0],
      [1, 0],
      [1, 0],
      [-1, -1],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 6, 8, zh("二"), 0);
    expectAbove(letter.strokes.slice(0, 6), letter.strokes.slice(6));
  });

  it("Chinese 找 writes the hand piece of 打 before its right side, keeping all seven source medians", () => {
    const letter = zh("找");
    expectPinnedSource(letter, "%E6%89%BE", [2, 4, 2, 4, 12, 5, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [1, 0],
      [1, 0],
      [1, -1],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 3, zh("打"), 0);
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });

  it("Chinese 关 writes the two short top strokes before 天, keeping all six source medians", () => {
    const letter = zh("关");
    expectPinnedSource(letter, "%E5%85%B3", [3, 3, 2, 2, 8, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [-1, -1],
      [1, 0],
      [1, 0],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 2, 6, zh("天"), 0);
  });

  it("Chinese 东 turns its second stroke as 车 does, keeping all five source medians", () => {
    const letter = zh("东");
    expectPinnedSource(letter, "%E4%B8%9C", [2, 4, 3, 3, 5]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [1, -1],
      [0, -1],
      [-1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 1, 2, zh("车"), 1);
  });

  it("Chinese 离 writes the cap of 文 first and the hooked frame last, keeping all ten source medians", () => {
    const letter = zh("离");
    expectPinnedSource(letter, "%E7%A6%BB", [3, 4, 5, 3, 6, 3, 4, 10, 7, 3]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, -1],
      [1, 0],
      [-1, -1],
      [1, -1],
      [1, 0],
      [0, -1],
      [0, -1],
      [1, -1],
      [1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 2, zh("文"), 0);
    expectSameHeadings(letter, 0, 2, zh("这"), 0);
  });

  it("Chinese 公 writes 八 before the two strokes that open 能, keeping all four source medians", () => {
    const letter = zh("公");
    expectPinnedSource(letter, "%E5%85%AC", [3, 5, 5, 4]);
    expect(letter.strokes.map(heading)).toEqual([
      [-1, -1],
      [1, -1],
      [1, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 2, 4, zh("能"), 0);
  });

  it("Chinese 地 writes the earth piece before 也, keeping all six source medians", () => {
    const letter = zh("地");
    expectPinnedSource(letter, "%E5%9C%B0", [2, 2, 2, 5, 2, 6]);
    expect(letter.strokes.map(heading)).toEqual([
      [1, 0],
      [0, -1],
      [1, 0],
      [1, -1],
      [0, -1],
      [1, -1],
    ]);
    expectSameHeadings(letter, 0, 2, zh("在"), 3);
    expectLeftOf(letter.strokes.slice(0, 3), letter.strokes.slice(3));
  });
});
