using System.Text;

namespace CodingAdventures.HistoricalWeb;

/// <summary>In-memory codecs for the CERN 1991 HTTP/0.9 wire profile.</summary>
public static class Http09
{
    private static ReadOnlySpan<byte> RequestPrefix => "GET "u8;
    private static ReadOnlySpan<byte> CarriageReturnLineFeed => "\r\n"u8;

    /// <summary>
    /// Encodes a nonempty printable-ASCII document address. The caller must
    /// keep the input bytes stable until the method returns.
    /// </summary>
    public static byte[] EncodeRequest(ReadOnlySpan<byte> address)
    {
        if (address.IsEmpty)
        {
            throw new HistoricalWebException(
                HistoricalWebErrorKind.InvalidAddress,
                null,
                "HTTP/0.9 address must not be empty.");
        }

        for (var i = 0; i < address.Length; i++)
        {
            if (address[i] is < 0x21 or > 0x7e)
            {
                throw new HistoricalWebException(
                    HistoricalWebErrorKind.InvalidAddress,
                    i,
                    "HTTP/0.9 address must contain printable ASCII bytes.");
            }
        }

        // Four prefix bytes, the address, then CRLF. Check before allocation.
        int addressEnd;
        int requestLength;
        checked
        {
            addressEnd = RequestPrefix.Length + address.Length;
            requestLength = addressEnd + CarriageReturnLineFeed.Length;
        }

        var request = new byte[requestLength];
        RequestPrefix.CopyTo(request);
        address.CopyTo(request.AsSpan(RequestPrefix.Length, address.Length));
        CarriageReturnLineFeed.CopyTo(request.AsSpan(addressEnd, CarriageReturnLineFeed.Length));
        return request;
    }

    /// <summary>
    /// Parses one request and returns a fresh copy of its address bytes. The
    /// caller must keep the input bytes stable until the method returns.
    /// </summary>
    public static byte[] ParseRequest(ReadOnlySpan<byte> request)
    {
        var prefix = RequestPrefix;
        var compared = Math.Min(request.Length, prefix.Length);
        for (var i = 0; i < compared; i++)
        {
            if (request[i] != prefix[i])
            {
                throw InvalidRequest(i);
            }
        }

        if (request.Length < prefix.Length)
        {
            throw InvalidRequest(request.Length);
        }

        var framedLength = request.Length;
        if (request.EndsWith(CarriageReturnLineFeed))
        {
            framedLength -= CarriageReturnLineFeed.Length;
        }
        else if (request[^1] == (byte)'\n')
        {
            framedLength--;
        }
        else
        {
            throw InvalidRequest(request.Length);
        }

        if (framedLength <= prefix.Length)
        {
            throw InvalidRequest(prefix.Length);
        }

        var address = request[prefix.Length..framedLength];
        for (var i = 0; i < address.Length; i++)
        {
            if (address[i] is < 0x21 or > 0x7e)
            {
                throw InvalidRequest(prefix.Length + i);
            }
        }

        return address.ToArray();
    }

    /// <summary>
    /// Decodes an ASCII response body without changing its bytes. The caller
    /// must keep the input bytes stable until the method returns.
    /// </summary>
    public static string DecodeResponse(ReadOnlySpan<byte> body)
    {
        for (var i = 0; i < body.Length; i++)
        {
            if (body[i] > 0x7f)
            {
                throw new HistoricalWebException(
                    HistoricalWebErrorKind.InvalidResponse,
                    i,
                    "HTTP/0.9 response body must contain ASCII bytes.");
            }
        }

        return body.IsEmpty ? string.Empty : Encoding.ASCII.GetString(body);
    }

    private static HistoricalWebException InvalidRequest(int offset) => new(
        HistoricalWebErrorKind.InvalidRequest,
        offset,
        "Malformed HTTP/0.9 request.");
}
