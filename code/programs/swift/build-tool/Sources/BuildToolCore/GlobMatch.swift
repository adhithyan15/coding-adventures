import Foundation

public enum GlobMatch {
    public static func matchPath(_ pattern: String, _ path: String) -> Bool {
        let normalizedPattern = normalize(pattern)
        let normalizedPath = normalize(path)
        let patternParts = splitPath(normalizedPattern)
        let pathParts = splitPath(normalizedPath)
        return matchSegments(patternParts, pathParts)
    }

    private static func normalize(_ value: String) -> String {
        var result = value.replacingOccurrences(of: "\\", with: "/")
        while result.hasSuffix("/") {
            result.removeLast()
        }
        return result
    }

    private static func splitPath(_ value: String) -> [String] {
        guard !value.isEmpty else {
            return []
        }
        return value
            .split(separator: "/")
            .map(String.init)
            .filter { !$0.isEmpty }
    }

    private static func matchSegments(_ pattern: [String], _ path: [String]) -> Bool {
        if pattern.isEmpty {
            return path.isEmpty
        }

        if path.isEmpty {
            return pattern.allSatisfy { $0 == "**" }
        }

        let head = pattern[0]
        if head == "**" {
            var rest = Array(pattern.dropFirst())
            while rest.first == "**" {
                rest.removeFirst()
            }
            for index in 0...path.count {
                if matchSegments(rest, Array(path.dropFirst(index))) {
                    return true
                }
            }
            return false
        }

        guard matchSegment(head, path[0]) else {
            return false
        }
        return matchSegments(Array(pattern.dropFirst()), Array(path.dropFirst()))
    }

    private static func matchSegment(_ pattern: String, _ segment: String) -> Bool {
        let regexPattern = "^" + convertSegmentToRegex(pattern) + "$"
        guard let regex = try? NSRegularExpression(pattern: regexPattern) else {
            return false
        }
        let range = NSRange(location: 0, length: segment.utf16.count)
        return regex.firstMatch(in: segment, options: [], range: range) != nil
    }

    private static func convertSegmentToRegex(_ pattern: String) -> String {
        var result = ""
        var index = pattern.startIndex

        while index < pattern.endIndex {
            let character = pattern[index]
            switch character {
            case "*":
                result += "[^/]*"
                index = pattern.index(after: index)
            case "?":
                result += "[^/]"
                index = pattern.index(after: index)
            case "[":
                let next = pattern.index(after: index)
                if let end = pattern[next...].firstIndex(of: "]") {
                    result += String(pattern[index...end])
                    index = pattern.index(after: end)
                } else {
                    result += "\\["
                    index = pattern.index(after: index)
                }
            default:
                if ".+(){}^$|\\".contains(character) {
                    result += "\\"
                }
                result.append(character)
                index = pattern.index(after: index)
            }
        }

        return result
    }
}
