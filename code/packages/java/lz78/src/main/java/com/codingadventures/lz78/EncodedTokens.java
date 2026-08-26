package com.codingadventures.lz78;

import java.util.List;

/** Deserialized LZ78 tokens together with the uncompressed byte length. */
public record EncodedTokens(List<Token> tokens, int originalLength) {
    public EncodedTokens {
        tokens = List.copyOf(tokens);
        if (originalLength < 0) {
            throw new IllegalArgumentException("original length must be non-negative");
        }
    }
}
