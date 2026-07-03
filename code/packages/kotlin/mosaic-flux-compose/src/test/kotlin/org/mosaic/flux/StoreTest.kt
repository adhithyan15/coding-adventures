package org.mosaic.flux

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

private data class StS(val count: Int = 0, val label: String = "")

private class StIncrement : MosaicAction<StS> {
    override fun apply(state: StS): StS = state.copy(count = state.count + 1)
}

private data class StSetLabel(val label: String) : MosaicAction<StS> {
    override fun apply(state: StS): StS = state.copy(label = label)
}

private class StNoOp : MosaicAction<StS> {
    override fun apply(state: StS): StS = state
}

class StoreTest {
    @Test
    fun startsAtInitialState() {
        val store = MosaicStore(StS())
        assertEquals(StS(), store.state)
    }

    @Test
    fun dispatchAppliesAction() {
        val store = MosaicStore(StS())
        store.dispatch(StIncrement())
        assertEquals(1, store.state.count)
    }

    @Test
    fun payloadedActionWorks() {
        val store = MosaicStore(StS())
        store.dispatch(StSetLabel("hi"))
        assertEquals("hi", store.state.label)
    }

    @Test
    fun selectReturnsProjection() {
        val store = MosaicStore(StS(count = 5))
        assertEquals(5, store.select { it.count })
    }

    @Test
    fun subscribeFiresOnChangedSlice() {
        val store = MosaicStore(StS())
        val received = mutableListOf<Int>()
        store.subscribe({ it.count }, { a, b -> a == b }) { received.add(it) }
        store.dispatch(StIncrement())
        assertEquals(listOf(1), received)
    }

    @Test
    fun subscribeDoesNotFireOnUnrelatedChange() {
        val store = MosaicStore(StS())
        val received = mutableListOf<Int>()
        store.subscribe({ it.count }, { a, b -> a == b }) { received.add(it) }
        store.dispatch(StSetLabel("ignore"))
        assertTrue(received.isEmpty())
    }

    @Test
    fun unsubscribeStopsNotifications() {
        val store = MosaicStore(StS())
        val received = mutableListOf<Int>()
        val unsub = store.subscribe({ it.count }, { a, b -> a == b }) { received.add(it) }
        store.dispatch(StIncrement())
        unsub()
        store.dispatch(StIncrement())
        assertEquals(listOf(1), received)
    }

    @Test
    fun middlewareSeesTriple() {
        val seen = mutableListOf<Triple<String, Int, Int>>()
        val m: Middleware<StS> = { action, prev, next ->
            seen.add(Triple(action::class.simpleName ?: "?", prev.count, next.count))
        }
        val store = MosaicStore(StS(), listOf(m))
        store.dispatch(StIncrement())
        assertEquals(1, seen.size)
        assertEquals(0, seen[0].second)
        assertEquals(1, seen[0].third)
    }

    @Test
    fun noOpDispatchSkipsSubscriberButRunsMiddleware() {
        var subscriberCalls = 0
        var middlewareCalls = 0
        val store = MosaicStore(
            StS(),
            listOf({ _, _, _ -> middlewareCalls++ }),
        )
        store.subscribe({ it.count }, { a, b -> a == b }) { subscriberCalls++ }
        store.dispatch(StNoOp())
        assertEquals(0, subscriberCalls)
        assertEquals(1, middlewareCalls)
    }

    @Test
    fun stateFlowExposesCurrentState() {
        val store = MosaicStore(StS(count = 42))
        assertEquals(42, store.stateFlow.value.count)
        store.dispatch(StIncrement())
        assertEquals(43, store.stateFlow.value.count)
    }

    @Test
    fun customEqualityRespected() {
        val store = MosaicStore(StS())
        val received = mutableListOf<Int>()
        store.subscribe({ it.count }, { _, _ -> true }) { received.add(it) }
        store.dispatch(StIncrement())
        assertTrue(received.isEmpty())
    }
}
