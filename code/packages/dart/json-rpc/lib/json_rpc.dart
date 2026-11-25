/// JSON-RPC 2.0 transport layer for LSP-style servers.
///
/// Messages ([Message]/[Request]/[Response]/[Notification]) parse from
/// bytes with [parseMessage] and serialize with [messageToJson].
/// [MessageReader] and [MessageWriter] move Content-Length-framed
/// messages through in-memory byte buffers, and [Server] dispatches
/// requests and notifications to registered handlers. Parse and framing
/// failures surface as [ResponseError].
library;

export 'src/errors.dart';
export 'src/json_values.dart'
    hide
        jsonValuesEqual,
        jsonValueHash,
        checkJsonIntegers,
        checkNoLoneSurrogates,
        maxJsonDepth;
export 'src/message.dart';
export 'src/reader.dart';
export 'src/server.dart';
export 'src/writer.dart';
