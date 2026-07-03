// smoke.swift — prove the spreadsheet C ABI is callable from Swift (the path
// SwiftUI uses), computing the same results as the Rust/WASM/TS engines.
// Compiled + run by verify-native.sh via the C header (clang importer).
import Foundation

func value(_ s: OpaquePointer?, _ a1: String) -> String {
    guard let p = sc_get_value(s, a1) else { return "(null)" }
    defer { sc_string_free(p) }
    return String(cString: p)
}

let s = sc_session_new()
for (a, v) in [("B1", "15"), ("B2", "8"), ("B3", "12"), ("B4", "4"), ("B5", "7")] {
    sc_string_free(sc_set_cell(s, a, v))
}
sc_string_free(sc_set_cell(s, "B6", "=SUM(B1:B5)"))
sc_string_free(sc_set_cell(s, "B7", "=AVERAGE(B1:B5)"))
sc_string_free(sc_set_cell(s, "C1", "=1/0"))

var failures = 0
func check(_ label: String, _ got: String, _ needle: String) {
    let ok = got.contains(needle)
    if !ok { failures += 1 }
    print("\(ok ? "ok  " : "FAIL")  \(label): \(got)")
}

check("B6 SUM",        value(s, "B6"), "\"value\":46")
check("B7 AVERAGE",    value(s, "B7"), "\"value\":9.2")
check("C1 div-by-0",   value(s, "C1"), "#DIV/0!")
sc_string_free(sc_set_cell(s, "B1", "115"))
check("B6 after edit", value(s, "B6"), "\"value\":146")

sc_session_free(s)
print(failures == 0 ? "\nALL PASS" : "\n\(failures) FAILURE(S)")
exit(failures == 0 ? 0 : 1)
