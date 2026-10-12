package io.github.kimzam30.nhakobeam.platform

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioManager
import android.media.SoundPool
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import io.github.kimzam30.nhakobeam.design.NhakoSound
import io.github.kimzam30.nhakobeam.state.Prefs
import io.github.kimzam30.nhakobeam.state.SoundPref
import io.github.kimzam30.nhakobeam.state.SoundPlayer

/**
 * Plays the app's sounds (design/sounds/) through SoundPool, as interface sounds, at the level
 * chosen in Settings (Loud, or Soft at [SOFT] of it). Quiet when Sounds is Off, when the phone is
 * on silent or vibrate, and when the app is out of sight (notifications carry their own sound
 * then, see [Notifier]).
 */
class Sounds(context: Context, private val prefs: Prefs, private val inSight: () -> Boolean) {
    private val audio = context.getSystemService(AudioManager::class.java)
    private val handler = Handler(Looper.getMainLooper())
    private val pool = SoundPool.Builder()
        .setMaxStreams(6)
        .setAudioAttributes(
            AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_ASSISTANCE_SONIFICATION)
                .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                .build(),
        )
        .build()
    private val ids = NhakoSound.entries.associateWith { pool.load(context, it.res, 1) }
    private val loaded = mutableSetOf<Int>()
    /** A sound asked for before its file had loaded (the launch intro, right at start). */
    private var early: Pair<Int, Long>? = null

    init {
        pool.setOnLoadCompleteListener { _, id, status ->
            if (status != 0) return@setOnLoadCompleteListener
            loaded += id
            val e = early
            if (e?.first == id && SystemClock.uptimeMillis() - e.second < 600) start(id)
            if (e?.first == id) early = null
        }
    }

    private val ringerOn get() = audio?.ringerMode == AudioManager.RINGER_MODE_NORMAL

    val player = SoundPlayer(
        out = { sound -> ids[sound]?.let { if (it in loaded) start(it) else early = it to SystemClock.uptimeMillis() } },
        enabled = { prefs.sounds != SoundPref.Off && inSight() && ringerOn },
        post = { handler.post(it) },
        unpost = { handler.removeCallbacks(it) },
    )

    fun play(sound: NhakoSound) = player.play(sound)

    private fun start(id: Int) {
        val v = if (prefs.sounds == SoundPref.Loud) 1f else SOFT
        pool.play(id, v, v, 1, 0, 1f)
    }

    companion object {
        /** Soft plays everything at this share of Loud (the same as desktop/src/lib/sounds.svelte.ts). */
        const val SOFT = 0.4f
    }
}
