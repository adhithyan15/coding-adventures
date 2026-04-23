package com.codingadventures.respprotocol;

import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public final class RespCodec {
    private RespCodec() {}

    public static byte[] encode(RespValue value) {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        writeValue(out, value);
        return out.toByteArray();
    }

    public static DecodeResult decode(byte[] bytes) {
        return decode(bytes, 0);
    }

    public static DecodeResult decode(byte[] bytes, int offset) {
        if (offset >= bytes.length) {
            return null;
        }
        int cursor = offset;
        return switch (bytes[cursor]) {
            case '+' -> {
                LineResult line = readLine(bytes, cursor + 1);
                if (line == null) yield null;
                yield new DecodeResult(new RespValue.SimpleString(line.text), line.nextOffset);
            }
            case '-' -> {
                LineResult line = readLine(bytes, cursor + 1);
                if (line == null) yield null;
                yield new DecodeResult(new RespValue.ErrorString(line.text), line.nextOffset);
            }
            case ':' -> {
                LineResult line = readLine(bytes, cursor + 1);
                if (line == null) yield null;
                yield new DecodeResult(new RespValue.IntegerValue(Long.parseLong(line.text)), line.nextOffset);
            }
            case '$' -> decodeBulkString(bytes, cursor + 1);
            case '*' -> decodeArray(bytes, cursor + 1);
            default -> throw new IllegalArgumentException("Unknown RESP prefix: " + (char) bytes[cursor]);
        };
    }

    private static DecodeResult decodeBulkString(byte[] bytes, int offset) {
        LineResult line = readLine(bytes, offset);
        if (line == null) {
            return null;
        }
        int length = Integer.parseInt(line.text);
        if (length == -1) {
            return new DecodeResult(new RespValue.BulkString(null), line.nextOffset);
        }
        if (line.nextOffset + length + 2 > bytes.length) {
            return null;
        }
        byte[] payload = Arrays.copyOfRange(bytes, line.nextOffset, line.nextOffset + length);
        if (bytes[line.nextOffset + length] != '\r' || bytes[line.nextOffset + length + 1] != '\n') {
            throw new IllegalArgumentException("Invalid RESP bulk string terminator");
        }
        return new DecodeResult(new RespValue.BulkString(payload), line.nextOffset + length + 2);
    }

    private static DecodeResult decodeArray(byte[] bytes, int offset) {
        LineResult line = readLine(bytes, offset);
        if (line == null) {
            return null;
        }
        int count = Integer.parseInt(line.text);
        if (count == -1) {
            return new DecodeResult(new RespValue.ArrayValue(null), line.nextOffset);
        }
        List<RespValue> values = new ArrayList<>();
        int cursor = line.nextOffset;
        for (int i = 0; i < count; i++) {
            DecodeResult child = decode(bytes, cursor);
            if (child == null) {
                return null;
            }
            values.add(child.value());
            cursor = child.nextOffset();
        }
        return new DecodeResult(new RespValue.ArrayValue(values), cursor);
    }

    private static void writeValue(ByteArrayOutputStream out, RespValue value) {
        switch (value) {
            case RespValue.SimpleString simple -> writeLine(out, '+', simple.value());
            case RespValue.ErrorString error -> writeLine(out, '-', error.value());
            case RespValue.IntegerValue integer -> writeLine(out, ':', Long.toString(integer.value()));
            case RespValue.BulkString bulk -> {
                if (bulk.isNull()) {
                    writeRaw(out, "$-1\r\n");
                } else {
                    writeRaw(out, "$" + bulk.value().length + "\r\n");
                    out.writeBytes(bulk.value());
                    writeRaw(out, "\r\n");
                }
            }
            case RespValue.ArrayValue array -> {
                if (array.isNull()) {
                    writeRaw(out, "*-1\r\n");
                } else {
                    writeRaw(out, "*" + array.value().size() + "\r\n");
                    for (RespValue item : array.value()) {
                        writeValue(out, item);
                    }
                }
            }
        }
    }

    private static void writeLine(ByteArrayOutputStream out, char prefix, String value) {
        writeRaw(out, prefix + value + "\r\n");
    }

    private static void writeRaw(ByteArrayOutputStream out, String text) {
        out.writeBytes(text.getBytes(StandardCharsets.UTF_8));
    }

    private static LineResult readLine(byte[] bytes, int offset) {
        for (int i = offset; i + 1 < bytes.length; i++) {
            if (bytes[i] == '\r' && bytes[i + 1] == '\n') {
                return new LineResult(new String(bytes, offset, i - offset, StandardCharsets.UTF_8), i + 2);
            }
        }
        return null;
    }

    private record LineResult(String text, int nextOffset) {}
    public record DecodeResult(RespValue value, int nextOffset) {}
}
