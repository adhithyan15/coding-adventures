package com.codingadventures.lz78;

import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/** A deliberately small, byte-oriented implementation of LZ78. */
public final class Lz78 {
    public static final int MAX_DICTIONARY_SIZE = 65_536;
    public static final int DEFAULT_MAX_OUTPUT = 256 * 1024 * 1024;

    private Lz78() {}

    public static List<Token> encode(byte[] input) {
        return encode(input, MAX_DICTIONARY_SIZE);
    }

    public static List<Token> encode(byte[] input, int maxDictionarySize) {
        if (maxDictionarySize < 1 || maxDictionarySize > MAX_DICTIONARY_SIZE) {
            throw new IllegalArgumentException("dictionary size must be between 1 and 65536");
        }
        Map<ByteSequence, Integer> dictionary = new HashMap<>();
        dictionary.put(new ByteSequence(new byte[0]), 0);
        List<Token> tokens = new ArrayList<>();
        int offset = 0;
        int dictionarySize = 1;
        while (offset < input.length) {
            int phraseIndex = 0;
            int cursor = offset;
            while (cursor < input.length) {
                Integer candidate = dictionary.get(
                        new ByteSequence(Arrays.copyOfRange(input, offset, cursor + 1)));
                if (candidate == null) {
                    break;
                }
                phraseIndex = candidate;
                cursor++;
            }
            if (cursor == input.length) {
                tokens.add(new Token(phraseIndex, 0));
                break;
            }
            int nextByte = Byte.toUnsignedInt(input[cursor]);
            tokens.add(new Token(phraseIndex, nextByte));
            if (dictionarySize < maxDictionarySize) {
                dictionary.put(
                        new ByteSequence(Arrays.copyOfRange(input, offset, cursor + 1)),
                        dictionarySize++);
            }
            offset = cursor + 1;
        }
        return List.copyOf(tokens);
    }

    public static byte[] decode(List<Token> tokens, int originalLength) {
        return decode(tokens, originalLength, DEFAULT_MAX_OUTPUT);
    }

    public static byte[] decode(List<Token> tokens, int originalLength, int maxOutput) {
        if (originalLength < 0) {
            throw new IllegalArgumentException("original length must be non-negative");
        }
        if (maxOutput < 0 || originalLength > maxOutput) {
            throw new IllegalArgumentException("decoded data exceeds configured limit");
        }
        List<byte[]> dictionary = new ArrayList<>();
        dictionary.add(new byte[0]);
        ByteArrayOutputStream output = new ByteArrayOutputStream(Math.min(originalLength, 8192));
        for (Token token : tokens) {
            if (token.dictionaryIndex() >= dictionary.size()) {
                throw new IllegalArgumentException("token references an unavailable dictionary entry");
            }
            byte[] phrase = dictionary.get(token.dictionaryIndex());
            if (phrase.length > originalLength - output.size()) {
                throw new IllegalArgumentException("decoded data exceeds declared length");
            }
            output.writeBytes(phrase);
            boolean hasNextByte = output.size() < originalLength;
            byte[] entry = phrase;
            if (hasNextByte) {
                output.write(token.nextByte());
                entry = Arrays.copyOf(phrase, phrase.length + 1);
                entry[entry.length - 1] = (byte) token.nextByte();
            }
            if (dictionary.size() < MAX_DICTIONARY_SIZE && hasNextByte) {
                dictionary.add(entry);
            }
        }
        if (output.size() != originalLength) {
            throw new IllegalArgumentException("decoded data does not match declared length");
        }
        return output.toByteArray();
    }

    public static byte[] serialize(List<Token> tokens, int originalLength) {
        if (originalLength < 0) {
            throw new IllegalArgumentException("original length must be non-negative");
        }
        long wireLength = 8L + 4L * tokens.size();
        if (wireLength > Integer.MAX_VALUE) {
            throw new IllegalArgumentException("token stream is too large");
        }
        ByteBuffer buffer = ByteBuffer.allocate((int) wireLength).order(ByteOrder.BIG_ENDIAN);
        buffer.putInt(originalLength);
        buffer.putInt(tokens.size());
        for (Token token : tokens) {
            buffer.putShort((short) token.dictionaryIndex());
            buffer.put((byte) token.nextByte());
            buffer.put((byte) 0);
        }
        return buffer.array();
    }

    public static EncodedTokens deserialize(byte[] data) {
        if (data.length < 8) {
            throw new IllegalArgumentException("truncated LZ78 header");
        }
        ByteBuffer buffer = ByteBuffer.wrap(data).order(ByteOrder.BIG_ENDIAN);
        int originalLength = buffer.getInt();
        int tokenCount = buffer.getInt();
        if (originalLength < 0 || tokenCount < 0) {
            throw new IllegalArgumentException("negative LZ78 header field");
        }
        long expectedLength = 8L + 4L * tokenCount;
        if (expectedLength != data.length) {
            throw new IllegalArgumentException("LZ78 stream length does not match its header");
        }
        List<Token> tokens = new ArrayList<>(tokenCount);
        for (int index = 0; index < tokenCount; index++) {
            int dictionaryIndex = Short.toUnsignedInt(buffer.getShort());
            int nextByte = Byte.toUnsignedInt(buffer.get());
            if (buffer.get() != 0) {
                throw new IllegalArgumentException("reserved LZ78 byte must be zero");
            }
            tokens.add(new Token(dictionaryIndex, nextByte));
        }
        return new EncodedTokens(tokens, originalLength);
    }

    public static byte[] compress(byte[] input) {
        return compress(input, MAX_DICTIONARY_SIZE);
    }

    public static byte[] compress(byte[] input, int maxDictionarySize) {
        return serialize(encode(input, maxDictionarySize), input.length);
    }

    public static byte[] decompress(byte[] data) {
        return decompress(data, DEFAULT_MAX_OUTPUT);
    }

    public static byte[] decompress(byte[] data, int maxOutput) {
        EncodedTokens encoded = deserialize(data);
        return decode(encoded.tokens(), encoded.originalLength(), maxOutput);
    }

    private record ByteSequence(byte[] bytes) {
        private ByteSequence {
            bytes = bytes.clone();
        }

        @Override
        public boolean equals(Object other) {
            return other instanceof ByteSequence sequence && Arrays.equals(bytes, sequence.bytes);
        }

        @Override
        public int hashCode() {
            return Arrays.hashCode(bytes);
        }
    }
}
