package com.codingadventures.lz78;

/** A single LZ78 dictionary reference and following byte. */
public record Token(int dictionaryIndex, int nextByte) {
    public Token {
        if (dictionaryIndex < 0 || dictionaryIndex > 0xffff) {
            throw new IllegalArgumentException("dictionary index must fit in 16 bits");
        }
        if (nextByte < 0 || nextByte > 0xff) {
            throw new IllegalArgumentException("next byte must fit in 8 bits");
        }
    }
}
