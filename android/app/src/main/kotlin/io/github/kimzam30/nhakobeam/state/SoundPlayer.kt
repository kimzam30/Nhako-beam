package io.github.kimzam30.nhakobeam.state

import io.github.kimzam30.nhakobeam.design.NhakoSound

/**
 * Decides what plays: the Kotlin twin of desktop/src/lib/sounds.svelte.ts. A tap waits until the
 * click has been handled, and any other sound in that time replaces it, so a press never makes two
 * sounds. The same sound asked for twice in a row (a button and the code behind it) plays once.
 * Where the sound comes out is [out]; whether sounds are on at all is [enabled].
 */
class SoundPlayer(
    private val out: (NhakoSound) -> Unit,
    private val enabled: () -> Boolean,
    private val post: (Runnable) -> Unit,
    private val unpost: (Runnable) -> Unit,
    private val now: () -> Long = System::currentTimeMillis,
) {
    /** The last sounds played, newest first (for tests and the debug log). */
    var played = listOf<NhakoSound>()
        private set
    private var pendingTap: Runnable? = null
    private val last = HashMap<NhakoSound, Long>()

    fun play(sound: NhakoSound) {
        if (sound == NhakoSound.Tap) {
            if (pendingTap != null) return
            // Whether to play is decided when it plays: the press may be the one that changes the
            // level, and then it's heard at the new one (or not at all, for Off).
            val r = Runnable {
                pendingTap = null
                if (enabled()) emit(NhakoSound.Tap)
            }
            pendingTap = r
            post(r)
            return
        }
        if (!enabled()) return
        pendingTap?.let {
            unpost(it)
            pendingTap = null
        }
        val t = now()
        val before = last[sound]
        if (before != null && t - before < REPEAT_MS) return
        last[sound] = t
        emit(sound)
    }

    private fun emit(sound: NhakoSound) {
        played = (listOf(sound) + played).take(12)
        out(sound)
    }

    companion object {
        const val REPEAT_MS = 400L
    }
}
