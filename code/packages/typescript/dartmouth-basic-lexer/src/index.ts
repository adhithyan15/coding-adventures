/**
 * Dartmouth BASIC Lexer -- tokenizes the 1964 Dartmouth BASIC language.
 *
 * Dartmouth BASIC was created by John G. Kemeny and Thomas E. Kurtz at
 * Dartmouth College in 1964. It ran on a GE-225 mainframe, accessed through
 * uppercase-only teletypes. It was the first programming language designed
 * specifically for non-science students.
 *
 * This lexer produces a flat stream of tokens from Dartmouth BASIC source
 * text, suitable for feeding into a Dartmouth BASIC parser.
 *
 * Usage:
 *
 *     import { tokenizeDartmouthBasic } from "@coding-adventures/dartmouth-basic-lexer";
 *
 *     const tokens = tokenizeDartmouthBasic("10 LET X = 5\n20 PRINT X\n30 END");
 *     // [LINE_NUM("10"), KEYWORD("LET"), NAME("X"), EQ("="), NUMBER("5"), NEWLINE,
 *     //  LINE_NUM("20"), KEYWORD("PRINT"), NAME("X"), NEWLINE,
 *     //  LINE_NUM("30"), KEYWORD("END"), NEWLINE, EOF]
 */

export { tokenizeDartmouthBasic, createDartmouthBasicLexer } from "./tokenizer.js";
