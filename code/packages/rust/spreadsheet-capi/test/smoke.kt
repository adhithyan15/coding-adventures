// smoke.kt — prove the spreadsheet C ABI is callable from Kotlin/JVM via the
// Java Foreign Function & Memory API (the path Compose Desktop / Android use,
// without hand-written JNI glue). Compiled + run by verify-native.sh with the
// library path in CAPI_LIB. Requires JDK 21+ (--enable-preview for FFM).
import java.lang.foreign.*
import java.lang.invoke.MethodHandle

object Engine {
    private val linker = Linker.nativeLinker()
    private val lib = SymbolLookup.libraryLookup(System.getenv("CAPI_LIB"), Arena.global())

    private fun handle(name: String, desc: FunctionDescriptor): MethodHandle =
        linker.downcallHandle(lib.find(name).get(), desc)

    private val P = ValueLayout.ADDRESS
    private val scNew = handle("sc_session_new", FunctionDescriptor.of(P))
    private val scFree = handle("sc_session_free", FunctionDescriptor.ofVoid(P))
    private val scSet = handle("sc_set_cell", FunctionDescriptor.of(P, P, P, P))
    private val scGet = handle("sc_get_value", FunctionDescriptor.of(P, P, P))
    private val scStrFree = handle("sc_string_free", FunctionDescriptor.ofVoid(P))

    fun newSession(): MemorySegment = scNew.invoke() as MemorySegment
    fun freeSession(s: MemorySegment) { scFree.invoke(s) }

    private fun Arena.cstr(s: String): MemorySegment = allocateUtf8String(s)

    private fun take(p: MemorySegment): String {
        if (p.address() == 0L) return "(null)"
        val str = p.reinterpret(Long.MAX_VALUE).getUtf8String(0)
        scStrFree.invoke(p)
        return str
    }

    fun set(s: MemorySegment, a1: String, raw: String) = Arena.ofConfined().use { a ->
        take(scSet.invoke(s, a.cstr(a1), a.cstr(raw)) as MemorySegment)
    }
    fun value(s: MemorySegment, a1: String): String = Arena.ofConfined().use { a ->
        take(scGet.invoke(s, a.cstr(a1)) as MemorySegment)
    }
}

fun main() {
    val s = Engine.newSession()
    for ((a, v) in listOf("B1" to "15", "B2" to "8", "B3" to "12", "B4" to "4", "B5" to "7"))
        Engine.set(s, a, v)
    Engine.set(s, "B6", "=SUM(B1:B5)")
    Engine.set(s, "B7", "=AVERAGE(B1:B5)")
    Engine.set(s, "C1", "=1/0")

    var failures = 0
    fun check(label: String, got: String, needle: String) {
        val ok = got.contains(needle)
        if (!ok) failures++
        println("${if (ok) "ok  " else "FAIL"}  $label: $got")
    }
    check("B6 SUM",        Engine.value(s, "B6"), "\"value\":46")
    check("B7 AVERAGE",    Engine.value(s, "B7"), "\"value\":9.2")
    check("C1 div-by-0",   Engine.value(s, "C1"), "#DIV/0!")
    Engine.set(s, "B1", "115")
    check("B6 after edit", Engine.value(s, "B6"), "\"value\":146")

    Engine.freeSession(s)
    println(if (failures == 0) "\nALL PASS" else "\n$failures FAILURE(S)")
    kotlin.system.exitProcess(if (failures == 0) 0 else 1)
}
