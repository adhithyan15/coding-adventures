import Foundation

public enum GlobMatch {
    private enum SegmentToken {
        case invalid
        case star
        case anyScalar
        case literal(Unicode.Scalar)
        case characterClass(negated: Bool, members: [ClassMember])
    }

    private enum ClassMember {
        case scalar(Unicode.Scalar)
        case range(ClosedRange<UInt32>)
    }

    public static func matchPath(_ pattern: String, _ path: String) -> Bool {
        let patternParts = splitPath(normalize(pattern))
        let pathParts = splitPath(normalize(path))
        var previous = Array(repeating: false, count: pathParts.count + 1)
        previous[0] = true

        for patternPart in patternParts {
            var current = Array(repeating: false, count: pathParts.count + 1)
            if patternPart == "**" {
                current[0] = previous[0]
                for pathIndex in pathParts.indices {
                    current[pathIndex + 1] =
                        previous[pathIndex + 1] || current[pathIndex]
                }
            } else {
                for pathIndex in pathParts.indices where previous[pathIndex] {
                    current[pathIndex + 1] = matchSegment(
                        patternPart,
                        pathParts[pathIndex]
                    )
                }
            }
            previous = current
        }

        return previous[pathParts.count]
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
        return value.split(separator: "/")
            .map(String.init)
            .filter { !$0.isEmpty }
    }

    private static func matchSegment(_ pattern: String, _ segment: String) -> Bool {
        let tokens = tokenizeSegment(pattern)
        let scalars = Array(segment.unicodeScalars)
        var previous = Array(repeating: false, count: scalars.count + 1)
        previous[0] = true

        for token in tokens {
            var current = Array(repeating: false, count: scalars.count + 1)
            switch token {
            case .star:
                current[0] = previous[0]
                for scalarIndex in scalars.indices {
                    current[scalarIndex + 1] =
                        previous[scalarIndex + 1] || current[scalarIndex]
                }
            default:
                for scalarIndex in scalars.indices where previous[scalarIndex] {
                    current[scalarIndex + 1] = tokenMatches(
                        token,
                        scalars[scalarIndex]
                    )
                }
            }
            previous = current
        }

        return previous[scalars.count]
    }

    private static func tokenizeSegment(_ pattern: String) -> [SegmentToken] {
        let scalars = Array(pattern.unicodeScalars)
        var tokens: [SegmentToken] = []
        var index = 0
        while index < scalars.count {
            switch scalars[index].value {
            case 0x2A:
                tokens.append(.star)
                index += 1
            case 0x3F:
                tokens.append(.anyScalar)
                index += 1
            case 0x5B:
                guard
                    let parsed = parseCharacterClass(
                        scalars,
                        opening: index
                    )
                else {
                    tokens.append(.literal(scalars[index]))
                    index += 1
                    continue
                }
                tokens.append(parsed.token)
                index = parsed.nextIndex
            default:
                tokens.append(.literal(scalars[index]))
                index += 1
            }
        }
        return tokens
    }

    private static func parseCharacterClass(
        _ scalars: [Unicode.Scalar],
        opening: Int
    ) -> (token: SegmentToken, nextIndex: Int)? {
        var cursor = opening + 1
        let negated = cursor < scalars.count && scalars[cursor].value == 0x21
        if negated {
            cursor += 1
        }
        let bodyStart = cursor
        if cursor < scalars.count, scalars[cursor].value == 0x5D {
            cursor += 1
        }
        while cursor < scalars.count, scalars[cursor].value != 0x5D {
            cursor += 1
        }
        guard cursor < scalars.count else {
            return nil
        }

        let body = Array(scalars[bodyStart..<cursor])
        var members: [ClassMember] = []
        var memberIndex = 0
        while memberIndex < body.count {
            if memberIndex + 2 < body.count,
                body[memberIndex + 1].value == 0x2D
            {
                guard
                    body[memberIndex].value <= body[memberIndex + 2].value
                else {
                    return (.invalid, cursor + 1)
                }
                let lower = body[memberIndex].value
                let upper = body[memberIndex + 2].value
                members.append(
                    .range(lower...upper)
                )
                memberIndex += 3
            } else {
                members.append(.scalar(body[memberIndex]))
                memberIndex += 1
            }
        }
        return (
            .characterClass(negated: negated, members: members),
            cursor + 1
        )
    }

    private static func tokenMatches(_ token: SegmentToken, _ scalar: Unicode.Scalar) -> Bool {
        switch token {
        case .invalid:
            return false
        case .star, .anyScalar:
            return true
        case .literal(let expected):
            return expected == scalar
        case .characterClass(let negated, let members):
            let contained = members.contains { member in
                switch member {
                case .scalar(let expected):
                    return expected == scalar
                case .range(let range):
                    return range.contains(scalar.value)
                }
            }
            return negated ? !contained : contained
        }
    }
}
