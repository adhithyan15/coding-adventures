## Chapter 21 — reading: words, answers, and the figures

Persian's reading rung, and the smallest of the eight so far, because the
alphabet is what bounds it.

Persian has taught **nine letters** -- ا ب ت س ل م ن ه و -- and ten digits. Every
other word in the track (چای, کلید, مادر, خورشید) needs a letter the script
ladder has not reached, and the practice name سارا needs ر. A reading lesson
cannot route around a letter the reader cannot decode, so the chapter was built
from the nine and the ten rather than from the track's vocabulary.

- **Words**: سلام, ممنون, بله, نه, نان, ماه. The lesson points at نان beside نه,
  which open with the same letter and differ at the end -- where a reader is
  most likely to stop looking.
- **Answers**: بله, نه, ممنون, سلام, اول, سوم. The list changes halfway: the
  first four are things you say to a person, the last two are things you read
  off a door.
- **Figures**: the ten digits, read as amounts. These are the only Persian most
  travellers read before they read any word at all, and the lesson names the one
  thing that surprises a beginner -- a number runs left to right inside a line
  that runs right to left, so ۱۰۰ is a one and two zeros in the order already
  expected.

pre-A1 reading moves 0/3 -> 3/3, floor raised. Persian declares no A1 shape yet.

One thing the local gates could not see. The figures lesson first carried the
Persian digits in its `romanization` field, and the book renderer puts
romanization in the OPTIONAL argument of `\section` -- which is the PDF
bookmark. Persian script there comes out as `\fa{...}`, and hyperref cannot
expand a macro into a PDF string, so the build raised thirteen
"Token not allowed in a PDF string" warnings against a track baseline of zero.

The convention every other Persian chapter already follows is that the short
title is Latin: `\section[avval]{\fa{اول} — first...}`. The romanization is now
`sefr tâ noh, dah, bist, sad`, and no bookmark in the chapter contains a macro.

Worth knowing for the next track: 27 generated chapter files across the corpus
DO put `\ar{...}` and similar in bookmarks, and their books pass because those
tracks carry a seeded warning baseline. Persian's baseline is zero, which is
worth keeping rather than seeding.

