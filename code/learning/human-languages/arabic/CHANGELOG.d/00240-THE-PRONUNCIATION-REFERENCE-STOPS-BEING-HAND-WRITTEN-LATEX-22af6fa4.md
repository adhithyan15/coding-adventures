## The pronunciation reference stops being hand-written LaTeX

`arabic/book/chapters/appendix-pronunciation.tex` was hand-authored and printed as a
`\chapter*`. It is now rendered from `arabic/pronunciation-reference.md`.

**The retired page had two pieces of empty markup.** Where ʿayn and hamza should
have been, it carried

    \emph{\`{}}    and    \emph{\'{}}

— a grave and an acute accent applied to nothing at all, printing as floating
marks over blank space in the two entries whose whole subject is those letters.
The Markdown writes them as ʿ (U+02BF) and ʾ (U+02BE), which Latin Modern has, so
the defect retires with the file.

Nothing the LaTeX said is lost, and the Markdown is much the richer source: it
adds a whole section on the light consonants close to English, the complete sun-
and moon-letter lists, the second assimilation example *al-nūr* → *an-nūr*, and
six more example words. The one hyphen that hard-wrapped across a line
(*sound-and-* / *script*) is joined, since reference prose is joined line by line
with a space and the page would have read "sound-and- script".

