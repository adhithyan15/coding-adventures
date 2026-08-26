package com.codingadventures.canonicalcbor;

import java.util.Arrays;
import java.util.List;
import java.util.Objects;

/**
 * The deliberately small value algebra supported by CBR01.
 *
 * <p>CBOR's negative integers are represented as the unsigned argument
 * {@code n} in {@code -1 - n}. Java has no unsigned {@code long} primitive,
 * so {@link Unsigned}, {@link Negative}, and {@link Tag} preserve all 64 bits
 * in a signed {@code long}; the codec compares and shifts those bits with the
 * JDK's unsigned operations.</p>
 */
public sealed interface CborValue permits CborValue.Unsigned, CborValue.Negative,
        CborValue.Bytes, CborValue.Text, CborValue.Array, CborValue.Map,
        CborValue.Tag, CborValue.Bool, CborValue.Null {

    /** Major type 0. */
    record Unsigned(long value) implements CborValue { }

    /** Major type 1, where the represented mathematical value is {@code -1 - value}. */
    record Negative(long value) implements CborValue { }

    /** Major type 2 with defensive-copy value semantics. */
    final class Bytes implements CborValue {
        private final byte[] value;

        public Bytes(byte[] value) {
            this.value = Objects.requireNonNull(value, "value").clone();
        }

        public byte[] value() {
            return value.clone();
        }

        byte[] rawValue() {
            return value;
        }

        @Override
        public boolean equals(Object other) {
            return other instanceof Bytes bytes && Arrays.equals(value, bytes.value);
        }

        @Override
        public int hashCode() {
            return Arrays.hashCode(value);
        }

        @Override
        public String toString() {
            return "Bytes[length=" + value.length + "]";
        }
    }

    /** Major type 3. Java strings always provide Unicode scalar text to the encoder. */
    record Text(String value) implements CborValue {
        public Text {
            Objects.requireNonNull(value, "value");
        }
    }

    /** Major type 4. */
    record Array(List<CborValue> values) implements CborValue {
        public Array {
            values = List.copyOf(values);
        }
    }

    /** One pre-canonicalization map entry. */
    record MapEntry(CborValue key, CborValue value) {
        public MapEntry {
            Objects.requireNonNull(key, "key");
            Objects.requireNonNull(value, "value");
        }
    }

    /** Major type 5. The encoder sorts entries by their encoded key bytes. */
    record Map(List<MapEntry> entries) implements CborValue {
        public Map {
            entries = List.copyOf(entries);
        }
    }

    /** Major type 6. Tag semantics remain opaque to this package. */
    record Tag(long number, CborValue value) implements CborValue {
        public Tag {
            Objects.requireNonNull(value, "value");
        }
    }

    /** Major type 7 simple values 20 and 21. */
    record Bool(boolean value) implements CborValue { }

    /** Major type 7 simple value 22. */
    enum Null implements CborValue {
        INSTANCE
    }
}

