package com.codingadventures.inmemorydatastore;

import com.codingadventures.inmemorydatastoreengine.DataStoreEngine;
import com.codingadventures.inmemorydatastoreprotocol.CommandFrame;
import com.codingadventures.inmemorydatastoreprotocol.EngineResponse;
import com.codingadventures.respprotocol.RespCodec;
import com.codingadventures.respprotocol.RespValue;

import java.util.List;

public final class DataStoreManager {
    private final DataStoreEngine engine = new DataStoreEngine();

    public EngineResponse executeFrame(CommandFrame frame) {
        return engine.executeFrame(frame);
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
}
