### Fixed — a line break in an emitted C# string broke the build

`escape_csharp_string` passed raw line breaks through, and its comment
claimed C# allows them. It does not (CS1010), and C# also treats U+0085,
U+2028 and U+2029 as line breaks. A label or fixture containing any of them
made the generated shell fail to build.

`\n`, `\r`, `\t` and the three separators are now escaped, and the comment
is corrected. Found by the review of #15428.

