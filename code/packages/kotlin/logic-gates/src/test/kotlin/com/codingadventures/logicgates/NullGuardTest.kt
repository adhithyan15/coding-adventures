package com.codingadventures.logicgates

import java.lang.reflect.Constructor
import java.lang.reflect.InvocationTargetException
import java.lang.reflect.Method
import java.lang.reflect.Modifier
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * The module is built with -Xno-param-assertions, so every public member with
 * a reference-typed parameter must guard null itself. This test enumerates
 * them by reflection and proves each one rejects null with a package-chosen
 * IllegalArgumentException (never a silent success or a raw NPE).
 */
class NullGuardTest {
    private fun defaultFor(t: Class<*>): Any? = when (t) {
        Int::class.javaPrimitiveType -> 1
        String::class.java -> "x"
        IntArray::class.java -> intArrayOf(1, 1)
        else -> error("unsupported param type $t")
    }

    private fun cause(block: () -> Unit): Throwable? =
        try { block(); null } catch (e: InvocationTargetException) { e.cause }

    private fun check(types: Array<Class<*>>, what: String, call: (Array<Any?>) -> Unit) {
        for (i in types.indices) {
            if (types[i].isPrimitive) continue
            val args = Array(types.size) { defaultFor(types[it]) }
            args[i] = null
            val c = cause { call(args) }
            assertTrue(c is IllegalArgumentException, "$what param $i: $c")
            assertFalse(c is NullPointerException, "$what param $i raw NPE")
        }
    }

    @Test
    fun everyPublicReferenceParameterIsGuarded() {
        var checked = 0
        val gateMethods: List<Method> = LogicGates::class.java.declaredMethods.filter {
            Modifier.isPublic(it.modifiers) && !it.isSynthetic && it.parameterTypes.any { t -> !t.isPrimitive }
        }
        for (m in gateMethods) {
            check(m.parameterTypes, m.name) { m.invoke(LogicGates, *it) }
            checked++
        }
        assertTrue(gateMethods.map { it.name }.containsAll(listOf("AND_N", "OR_N", "XOR_N")))
        for (k in listOf(InvalidBitException::class.java, InvalidShapeException::class.java)) {
            for (ctor: Constructor<*> in k.constructors) {
                if (ctor.parameterTypes.none { !it.isPrimitive }) continue
                check(ctor.parameterTypes, k.simpleName) { ctor.newInstance(*it) }
                checked++
            }
        }
        assertEquals(5, checked)
    }

    @Test
    fun constructorsStillWorkWithValidArguments() {
        assertEquals("p", InvalidBitException("p", 3).parameter)
        assertEquals("p", InvalidShapeException("p", "m").parameter)
    }
}
