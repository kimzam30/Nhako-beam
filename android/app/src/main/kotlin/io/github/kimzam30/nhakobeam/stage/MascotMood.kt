package io.github.kimzam30.nhakobeam.stage

import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.state.Moments

/** The mascot's states (design/mascot/build/mascot.json). */
enum class MascotState(val id: String) { Idle("idle"), Looking("looking"), Happy("happy"), Sending("sending"), Receiving("receiving"), Done("done"), Failed("failed"), Asleep("asleep"), Offline("offline"), Loading("loading") }

/**
 * Which state the mascot is in, from what the app knows. The Kotlin twin of
 * desktop/src/lib/stage/mood.ts; both are tested with the same cases.
 */
data class MoodInput(
    val now: Long,
    /** The engine has started (before that: loading). */
    val started: Boolean,
    val offline: Boolean,
    val searching: Boolean,
    /** Devices on the stage. */
    val devices: Int,
    val sending: Boolean,
    val receiving: Boolean,
    /** Something the mascot is looking at: a device being pressed or files dragged near one. */
    val looking: Boolean,
    val moments: Moments,
)

/** Moments come first while they last, then what's going on, then rest. */
fun mascotMood(i: MoodInput): MascotState {
    fun recent(at: Long, ms: Int) = at > 0 && i.now - at < ms
    val m = NhakoMotion.Mascot
    return when {
        !i.started -> MascotState.Loading
        i.offline -> MascotState.Offline
        recent(i.moments.failed, m.failedMs) -> MascotState.Failed
        recent(i.moments.done, m.doneMs) -> MascotState.Done
        i.sending -> MascotState.Sending
        i.receiving -> MascotState.Receiving
        recent(i.moments.found, m.foundMs) -> MascotState.Happy
        i.looking -> MascotState.Looking
        i.devices == 0 -> if (i.searching) MascotState.Looking else MascotState.Asleep
        else -> MascotState.Idle
    }
}

/** When the mood will next change by itself (a moment running out). */
fun nextChange(i: MoodInput): Long? {
    val m = NhakoMotion.Mascot
    return listOf(i.moments.failed + m.failedMs, i.moments.done + m.doneMs, i.moments.found + m.foundMs).filter { it > i.now }.minOrNull()
}
