/**
 * XML Lexer — tokenizes XML using pattern groups and callback hooks.
 *
 * This package is the first **callback-driven** lexer wrapper in TypeScript.
 * Unlike the JSON lexer (which uses a flat pattern list), the XML lexer uses
 * **pattern groups** and an **on-token callback** to handle XML's context-
 * sensitive lexical structure.
 *
 * Usage:
 *
 *     import { tokenizeXML } from "@coding-adventures/xml-lexer";
 *
 *     const tokens = tokenizeXML('<div class="main">Hello &amp; world</div>');
 */

export { createXMLLexer, tokenizeXML, xmlOnToken } from "./tokenizer.js";
