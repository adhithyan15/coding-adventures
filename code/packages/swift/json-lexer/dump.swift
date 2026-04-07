import Foundation
import JsonLexer
import Lexer

let json = "{\"key\": 123}"
do {
    let tokens = try tokenizeJson(json)
    for (i, t) in tokens.enumerated() {
        print("\(i): [\(t.type)] = '\(t.value)'")
    }
} catch {
    print("Error: \(error)")
}
