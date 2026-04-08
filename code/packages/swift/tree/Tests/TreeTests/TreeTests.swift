import XCTest
@testable import Tree

final class TreeTests: XCTestCase {
    func testTreeBasics() throws {
        let t = Tree(root: "Program")
        XCTAssertEqual(t.root, "Program")
        XCTAssertEqual(t.size, 1)
        
        try t.addChild(parent: "Program", child: "Assignment")
        try t.addChild(parent: "Program", child: "Print")
        try t.addChild(parent: "Assignment", child: "Name")
        try t.addChild(parent: "Assignment", child: "BinaryOp")
        
        XCTAssertEqual(t.size, 5)
        XCTAssertEqual(try t.parent(of: "Assignment"), "Program")
        XCTAssertEqual(try t.children(of: "Program"), ["Assignment", "Print"])
        XCTAssertEqual(try t.siblings(of: "Assignment"), ["Print"])
        
        XCTAssertTrue(try t.isLeaf("Name"))
        XCTAssertFalse(try t.isLeaf("Assignment"))
        XCTAssertTrue(try t.isRoot("Program"))
        XCTAssertFalse(try t.isRoot("Name"))
        
        XCTAssertEqual(try t.depth(of: "Program"), 0)
        XCTAssertEqual(try t.depth(of: "Name"), 2)
        XCTAssertEqual(t.height(), 2)
        
        XCTAssertEqual(t.nodes(), ["Assignment", "BinaryOp", "Name", "Print", "Program"])
        XCTAssertEqual(t.leaves(), ["BinaryOp", "Name", "Print"])
    }
    
    func testTraversals() throws {
        let t = Tree(root: "A")
        try t.addChild(parent: "A", child: "B")
        try t.addChild(parent: "A", child: "C")
        try t.addChild(parent: "B", child: "D")
        try t.addChild(parent: "B", child: "E")
        
        XCTAssertEqual(t.preorder(), ["A", "B", "D", "E", "C"])
        XCTAssertEqual(t.postorder(), ["D", "E", "B", "C", "A"])
        XCTAssertEqual(t.levelOrder(), ["A", "B", "C", "D", "E"])
    }
    
    func testLCA() throws {
        let t = Tree(root: "A")
        try t.addChild(parent: "A", child: "B")
        try t.addChild(parent: "A", child: "C")
        try t.addChild(parent: "B", child: "D")
        try t.addChild(parent: "B", child: "E")
        
        XCTAssertEqual(try t.lca(a: "D", b: "E"), "B")
        XCTAssertEqual(try t.lca(a: "D", b: "C"), "A")
        XCTAssertEqual(try t.lca(a: "B", b: "D"), "B")
    }
    
    func testErrors() {
        let t = Tree(root: "A")
        XCTAssertThrowsError(try t.addChild(parent: "Missing", child: "B")) { error in
            XCTAssertEqual(error as? TreeError, .nodeNotFound("Missing"))
        }
        
        try! t.addChild(parent: "A", child: "B")
        XCTAssertThrowsError(try t.addChild(parent: "A", child: "B")) { error in
            XCTAssertEqual(error as? TreeError, .duplicateNode("B"))
        }
        
        XCTAssertThrowsError(try t.removeSubtree(node: "A")) { error in
            XCTAssertEqual(error as? TreeError, .rootRemoval)
        }
    }
    
    func testToAscii() throws {
        let t = Tree(root: "Program")
        try t.addChild(parent: "Program", child: "Assignment")
        try t.addChild(parent: "Program", child: "Print")
        try t.addChild(parent: "Assignment", child: "Name")
        try t.addChild(parent: "Assignment", child: "BinaryOp")
        
        let expected = """
Program
├── Assignment
│   ├── BinaryOp
│   └── Name
└── Print
"""
        XCTAssertEqual(t.toAscii(), expected)
    }
}
