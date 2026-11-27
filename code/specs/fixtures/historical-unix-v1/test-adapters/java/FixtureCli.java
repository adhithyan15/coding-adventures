// Fixture-only trace adapter, compiled temporarily by the Java package BUILD.
package codingadventures.unixhistory;

import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

public final class FixtureCli {
    private FixtureCli() {}

    public static void main(String[] args) {
        var command = args.length == 0 ? "cat /hello" : String.join(" ", args);
        var result = UnixHistory.run(command);
        System.out.println(json(Map.of("output", result.output(), "events", result.events())));
    }

    private static String json(Object value) {
        if (value instanceof String text) {
            var encoded = new StringBuilder("\"");
            for (int i = 0; i < text.length(); i++) {
                char c = text.charAt(i);
                if (c < 32 || c == '"' || c == '\\') encoded.append(String.format("\\u%04x", (int) c));
                else encoded.append(c);
            }
            return encoded.append('"').toString();
        }
        if (value instanceof Number number) return number.toString();
        if (value instanceof List<?> list) {
            return list.stream().map(FixtureCli::json).collect(Collectors.joining(",", "[", "]"));
        }
        if (value instanceof Map<?, ?> map) {
            return map.entrySet().stream().map(item -> json(item.getKey()) + ":" + json(item.getValue()))
                .collect(Collectors.joining(",", "{", "}"));
        }
        throw new IllegalArgumentException("unknown trace field type");
    }
}
