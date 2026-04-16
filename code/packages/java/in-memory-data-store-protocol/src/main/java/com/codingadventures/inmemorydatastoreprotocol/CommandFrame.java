package com.codingadventures.inmemorydatastoreprotocol;

import com.codingadventures.respprotocol.RespValue;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;

public record CommandFrame(String command, List<byte[]> args) {
    public static CommandFrame fromParts(List<byte[]> parts) {
        if (parts.isEmpty()) {
            return null;
        }
        String command = new String(parts.getFirst(), StandardCharsets.UTF_8).toUpperCase(Locale.ROOT);
        return new CommandFrame(command, new ArrayList<>(parts.subList(1, parts.size())));
    }

    public static CommandFrame fromRespValue(RespValue value) {
        if (!(value instanceof RespValue.ArrayValue array) || array.isNull()) {
            return null;
        }
        List<byte[]> parts = new ArrayList<>();
        for (RespValue item : array.value()) {
            if (!(item instanceof RespValue.BulkString bulk) || bulk.isNull()) {
                return null;
            }
            parts.add(bulk.value());
        }
        return fromParts(parts);
    }
}
