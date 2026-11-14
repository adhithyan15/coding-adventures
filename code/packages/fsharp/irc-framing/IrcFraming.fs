namespace CodingAdventures.IrcFraming.FSharp

open System

[<RequireQualifiedAccess>]
module IrcFraming =
    [<Literal>]
    let VERSION = "0.1.0"

    /// RFC 1459 / RFC 2812 section 2.3: 512 bytes including CRLF leaves 510 of content.
    [<Literal>]
    let MAX_CONTENT_BYTES = 510

    /// The most bytes held for one line that has not yet seen its LF: 510 content bytes plus a CR.
    /// A legal line can be 510 bytes plus a CR whose LF has not arrived yet, i.e. 511 bytes.
    /// One more byte with no LF and the line can no longer be legal (SEC-011).
    [<Literal>]
    let MAX_PENDING_BYTES = MAX_CONTENT_BYTES + 1

type Framer() =
    let buffer = ResizeArray<byte>()

    // Length of the unterminated tail of `buffer` (bytes after the last LF).
    // Invariant: tailLen <= MAX_PENDING_BYTES after every Feed.
    let mutable tailLen = 0

    // True while we are skipping the rest of an overlong line: everything up
    // to and including the next LF is thrown away, unread.
    let mutable discarding = false

    member _.BufferSize = buffer.Count

    /// Append raw bytes to the internal buffer.
    ///
    /// Memory bound (SEC-011, CWE-400/770): the unterminated tail (bytes after the last LF) never
    /// exceeds MAX_PENDING_BYTES. When it would, the partial line is dropped and the framer
    /// discards input up to the next LF: fail closed, the same silent discard that already applies
    /// to a complete overlong line, so the API is unchanged. Complete lines already buffered are
    /// kept; the caller bounds those by calling Frames after each read.
    member _.Feed(data: byte array) =
        if isNull data then nullArg "data"

        // Step 1: while discarding an overlong line, skip to just past its LF.
        let mutable start = 0
        let mutable skipAll = false

        if discarding then
            let pos = System.Array.IndexOf(data, byte '\n')

            if pos < 0 then
                skipAll <- true // still inside the overlong line
            else
                discarding <- false
                start <- pos + 1

        if not skipAll then
            // Step 2: split data[start..] at its last LF. The head (through that LF) completes
            // lines and is stored; only the tail can be flooded.
            //
            //   data = [ ...complete lines... LF ][ unterminated tail ]
            //                                     ^ only this part can be flooded
            let lastLf = System.Array.LastIndexOf(data, byte '\n')
            let headEnd = if lastLf >= start then lastLf + 1 else start
            let headLen = headEnd - start
            let tailSize = data.Length - headEnd

            if headLen > 0 then
                buffer.AddRange(ArraySegment<byte>(data, start, headLen))
                tailLen <- 0

            // Step 3: store the tail only if the pending line still fits. We check *before*
            // copying so a huge single feed is never copied.
            if tailSize > IrcFraming.MAX_PENDING_BYTES - tailLen then
                buffer.RemoveRange(buffer.Count - tailLen, tailLen) // drop the partial line
                tailLen <- 0
                discarding <- true
            else
                buffer.AddRange(ArraySegment<byte>(data, headEnd, tailSize))
                tailLen <- tailLen + tailSize

    member _.Frames() =
        let frames = ResizeArray<byte array>()
        let mutable keepGoing = true

        while keepGoing do
            let mutable lfPos = -1
            let mutable index = 0

            while lfPos = -1 && index < buffer.Count do
                if buffer[index] = byte '\n' then
                    lfPos <- index
                else
                    index <- index + 1

            if lfPos < 0 then
                keepGoing <- false
            else
                let contentEnd =
                    if lfPos > 0 && buffer[lfPos - 1] = byte '\r' then
                        lfPos - 1
                    else
                        lfPos

                let line = buffer.GetRange(0, contentEnd).ToArray()
                buffer.RemoveRange(0, lfPos + 1)

                if line.Length <= IrcFraming.MAX_CONTENT_BYTES then
                    frames.Add(line)

        // Only the unterminated tail remains in the buffer.
        tailLen <- buffer.Count
        frames |> Seq.toList

    /// Discard all buffered partial input, including the skip-to-LF discard state.
    member _.Reset() =
        buffer.Clear()
        tailLen <- 0
        discarding <- false
