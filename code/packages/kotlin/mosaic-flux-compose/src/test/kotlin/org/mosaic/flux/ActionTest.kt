package org.mosaic.flux

import kotlin.test.Test
import kotlin.test.assertEquals

private data class ActionTestS(val count: Int = 0)

private class ActionTestInc : MosaicAction<ActionTestS> {
    override fun apply(state: ActionTestS): ActionTestS = state.copy(count = state.count + 1)
}

private data class ActionTestAdd(val amount: Int) : MosaicAction<ActionTestS> {
    override fun apply(state: ActionTestS): ActionTestS = state.copy(count = state.count + amount)
}

class ActionTest {
    @Test
    fun applyReturnsNextStateWithoutMutatingInput() {
        val initial = ActionTestS(count = 5)
        val next = ActionTestInc().apply(initial)
        assertEquals(6, next.count)
        assertEquals(5, initial.count)  // Kotlin data class copy semantics
    }

    @Test
    fun payloadAccessible() {
        val action = ActionTestAdd(amount = 7)
        assertEquals(7, action.amount)
        assertEquals(10, action.apply(ActionTestS(count = 3)).count)
    }

    @Test
    fun deterministic() {
        val state = ActionTestS(count = 0)
        val action = ActionTestAdd(amount = 5)
        assertEquals(action.apply(state), action.apply(state))
    }
}
