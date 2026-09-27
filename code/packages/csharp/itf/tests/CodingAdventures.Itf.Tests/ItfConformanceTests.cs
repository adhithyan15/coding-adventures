using System.Security.Cryptography;
using System.Text;
using System.Text.Json;

namespace CodingAdventures.Itf.Tests;

public sealed class ItfConformanceTests
{
    [Fact]
    public void SharedItfCorpusConforms()
    {
        using var fixture = JsonDocument.Parse(File.ReadAllText(FindFixture()));
        foreach (var testCase in fixture.RootElement.GetProperty("cases").EnumerateArray()
                     .Where(item => item.GetProperty("symbology").GetString() == "itf"))
        {
            var input = MaterializeInput(testCase.GetProperty("input"));
            var expected = testCase.GetProperty("expected");
            if (expected.TryGetProperty("error", out var error))
            {
                var exception = Assert.Throws<InvalidItfInputException>(() => Itf.NormalizeItf(input));
                Assert.Equal(error.GetString(), exception.ErrorId);
                continue;
            }

            var normalized = Itf.NormalizeItf(input);
            var modules = "1010" + string.Concat(Itf.EncodeItf(input).Select(pair => pair.BinaryPattern)) + "11101";
            var runs = RunLengths(modules);

            if (expected.TryGetProperty("normalized", out var expectedNormalized))
            {
                Assert.Equal(expectedNormalized.GetString(), normalized);
                Assert.Equal(expected.GetProperty("modules").GetString(), modules);
                Assert.Equal(expected.GetProperty("run_lengths").EnumerateArray().Select(item => item.GetInt32()), runs);
            }
            else
            {
                Assert.Equal(expected.GetProperty("normalized_sha256").GetString(), Sha256(normalized));
                Assert.Equal(expected.GetProperty("run_count").GetInt32(), runs.Count);
                Assert.Equal(expected.GetProperty("run_lengths_sha256").GetString(), Sha256(JsonSerializer.Serialize(runs)));
            }

            Assert.Equal(expected.GetProperty("module_count").GetInt32(), modules.Length);
            Assert.Equal(expected.GetProperty("module_sha256").GetString(), Sha256(modules));
        }
    }

    private static string MaterializeInput(JsonElement input) =>
        input.TryGetProperty("text", out var text)
            ? text.GetString()!
            : string.Concat(Enumerable.Repeat(
                input.GetProperty("repeat").GetProperty("text").GetString()!,
                input.GetProperty("repeat").GetProperty("count").GetInt32()));

    private static List<int> RunLengths(string modules)
    {
        var result = new List<int>();
        char? previous = null;
        foreach (var bit in modules)
        {
            if (bit != previous)
            {
                result.Add(1);
                previous = bit;
            }
            else
            {
                result[^1]++;
            }
        }
        return result;
    }

    private static string Sha256(string value) =>
        Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(value))).ToLowerInvariant();

    private static string FindFixture()
    {
        var current = new DirectoryInfo(AppContext.BaseDirectory);
        while (current is not null)
        {
            var candidate = Path.Combine(current.FullName, "code", "specs", "fixtures", "barcode-symbologies-v1", "cases.json");
            if (File.Exists(candidate))
            {
                return candidate;
            }
            current = current.Parent;
        }
        throw new FileNotFoundException("barcode-symbologies-v1 cases.json was not found");
    }
}
