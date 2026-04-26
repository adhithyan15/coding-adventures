import Foundation

public struct StandardScaler: Equatable {
    public let means: [Double]
    public let standardDeviations: [Double]
}

public struct MinMaxScaler: Equatable {
    public let minimums: [Double]
    public let maximums: [Double]
}

public enum FeatureNormalizationError: Error, Equatable {
    case emptyMatrix
    case raggedMatrix
    case widthMismatch
}

public enum FeatureNormalization {
    public static func fitStandardScaler(_ rows: [[Double]]) throws -> StandardScaler {
        let width = try validateMatrix(rows)
        let means = (0..<width).map { col in
            rows.reduce(0.0) { $0 + $1[col] } / Double(rows.count)
        }
        let standardDeviations = (0..<width).map { col in
            let variance = rows.reduce(0.0) { total, row in
                let diff = row[col] - means[col]
                return total + diff * diff
            } / Double(rows.count)
            return sqrt(variance)
        }
        return StandardScaler(means: means, standardDeviations: standardDeviations)
    }

    public static func transformStandard(_ rows: [[Double]], scaler: StandardScaler) throws -> [[Double]] {
        let width = try validateMatrix(rows)
        guard width == scaler.means.count, width == scaler.standardDeviations.count else {
            throw FeatureNormalizationError.widthMismatch
        }

        return rows.map { row in
            row.enumerated().map { col, value in
                let standardDeviation = scaler.standardDeviations[col]
                return standardDeviation == 0.0 ? 0.0 : (value - scaler.means[col]) / standardDeviation
            }
        }
    }

    public static func fitMinMaxScaler(_ rows: [[Double]]) throws -> MinMaxScaler {
        let width = try validateMatrix(rows)
        let minimums = (0..<width).map { col in rows.map { $0[col] }.min()! }
        let maximums = (0..<width).map { col in rows.map { $0[col] }.max()! }
        return MinMaxScaler(minimums: minimums, maximums: maximums)
    }

    public static func transformMinMax(_ rows: [[Double]], scaler: MinMaxScaler) throws -> [[Double]] {
        let width = try validateMatrix(rows)
        guard width == scaler.minimums.count, width == scaler.maximums.count else {
            throw FeatureNormalizationError.widthMismatch
        }

        return rows.map { row in
            row.enumerated().map { col, value in
                let span = scaler.maximums[col] - scaler.minimums[col]
                return span == 0.0 ? 0.0 : (value - scaler.minimums[col]) / span
            }
        }
    }

    private static func validateMatrix(_ rows: [[Double]]) throws -> Int {
        guard let first = rows.first, !first.isEmpty else {
            throw FeatureNormalizationError.emptyMatrix
        }

        let width = first.count
        guard rows.allSatisfy({ $0.count == width }) else {
            throw FeatureNormalizationError.raggedMatrix
        }
        return width
    }
}
