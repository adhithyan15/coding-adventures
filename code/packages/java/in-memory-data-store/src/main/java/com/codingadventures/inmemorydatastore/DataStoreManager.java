package com.codingadventures.inmemorydatastore;

import com.codingadventures.inmemorydatastoreengine.DataStoreEngine;
import com.codingadventures.inmemorydatastoreprotocol.CommandFrame;
import com.codingadventures.inmemorydatastoreprotocol.EngineResponse;
import com.codingadventures.respprotocol.RespCodec;
import com.codingadventures.respprotocol.RespValue;

import java.io.Closeable;
import java.io.FileOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Set;

public final class DataStoreManager implements Closeable {
    private static final Set<String> AOF_COMMANDS = Set.of(
            "SET", "DEL", "RENAME", "INCR", "DECR", "INCRBY", "DECRBY", "APPEND",
            "HSET", "HDEL", "LPUSH", "RPUSH", "LPOP", "RPOP",
            "SADD", "SREM", "ZADD", "ZREM", "PFADD", "PFMERGE",
            "EXPIRE", "EXPIREAT", "PERSIST", "SELECT", "FLUSHDB", "FLUSHALL"
    );

    private final DataStoreEngine engine;
    private final FileOutputStream aofStream;
    private final AofSyncPolicy aofSyncPolicy;

    public DataStoreManager() {
        this.engine = new DataStoreEngine();
        this.aofStream = null;
        this.aofSyncPolicy = AofSyncPolicy.ALWAYS;
    }

    public DataStoreManager(Path aofPath) throws IOException {
        this(aofPath, AofSyncPolicy.ALWAYS);
    }

    public DataStoreManager(Path aofPath, AofSyncPolicy aofSyncPolicy) throws IOException {
        this.engine = new DataStoreEngine();
        this.aofSyncPolicy = aofSyncPolicy;
        replayAof(aofPath);
        Path parent = aofPath.toAbsolutePath().getParent();
        if (parent != null) {
            Files.createDirectories(parent);
        }
        this.aofStream = new FileOutputStream(
                aofPath.toFile(),
                true
        );
    }

    public EngineResponse executeFrame(CommandFrame frame) {
        EngineResponse response = engine.executeFrame(frame);
        appendToAof(frame, response);
        return response;
    }

    public EngineResponse executeParts(List<byte[]> parts) {
        return executeFrame(CommandFrame.fromParts(parts));
    }

    public byte[] executeRespBytes(byte[] request) {
        RespCodec.DecodeResult decoded = RespCodec.decode(request);
        if (decoded == null) {
            return RespCodec.encode(EngineResponse.error("ERR incomplete RESP frame").toRespValue());
        }
        CommandFrame frame = CommandFrame.fromRespValue(decoded.value());
        return RespCodec.encode(executeFrame(frame).toRespValue());
    }

    public RespValue executeRespValue(RespValue value) {
        return executeFrame(CommandFrame.fromRespValue(value)).toRespValue();
    }

    @Override
    public void close() throws IOException {
        if (aofStream != null) {
            aofStream.close();
        }
    }

    private void replayAof(Path aofPath) throws IOException {
        if (!Files.exists(aofPath)) {
            return;
        }
        byte[] bytes = Files.readAllBytes(aofPath);
        int offset = 0;
        while (offset < bytes.length) {
            RespCodec.DecodeResult decoded = RespCodec.decode(bytes, offset);
            if (decoded == null) {
                break;
            }
            engine.executeFrame(CommandFrame.fromRespValue(decoded.value()));
            offset = decoded.nextOffset();
        }
    }

    private void appendToAof(CommandFrame frame, EngineResponse response) {
        if (frame == null || aofStream == null || response instanceof EngineResponse.ErrorString || !AOF_COMMANDS.contains(frame.command())) {
            return;
        }
        try {
            aofStream.write(RespCodec.encode(commandValue(canonicalCommand(frame))));
            aofStream.flush();
            if (aofSyncPolicy == AofSyncPolicy.ALWAYS) {
                aofStream.getChannel().force(true);
            }
        } catch (IOException error) {
            throw new IllegalStateException("Failed to append command to AOF", error);
        }
    }

    private static CommandFrame canonicalCommand(CommandFrame frame) {
        if (!frame.command().equals("EXPIRE") || frame.args().size() != 2) {
            return frame;
        }
        long seconds = Long.parseLong(new String(frame.args().get(1), StandardCharsets.UTF_8));
        long absoluteSeconds = (DataStoreEngine.currentTimeMs() / 1000L) + seconds;
        return new CommandFrame(
                "EXPIREAT",
                List.of(
                        frame.args().getFirst(),
                        Long.toString(absoluteSeconds).getBytes(StandardCharsets.UTF_8)
                )
        );
    }

    private static RespValue.ArrayValue commandValue(CommandFrame frame) {
        ArrayList<RespValue> values = new ArrayList<>();
        values.add(new RespValue.BulkString(frame.command().getBytes(StandardCharsets.UTF_8)));
        for (byte[] arg : frame.args()) {
            values.add(new RespValue.BulkString(arg));
        }
        return new RespValue.ArrayValue(values);
    }

    public enum AofSyncPolicy {
        ALWAYS,
        NONE
    }
}
