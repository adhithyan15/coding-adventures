public enum TreeError: Error, CustomStringConvertible, Equatable {
    case nodeNotFound(String)
    case duplicateNode(String)
    case rootRemoval
    
    public var description: String {
        switch self {
        case .nodeNotFound(let node):
            return "Node not found in tree: '\(node)'"
        case .duplicateNode(let node):
            return "Node already exists in tree: '\(node)'"
        case .rootRemoval:
            return "Cannot remove the root node"
        }
    }
}
