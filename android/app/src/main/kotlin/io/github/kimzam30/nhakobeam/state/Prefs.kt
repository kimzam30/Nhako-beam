package io.github.kimzam30.nhakobeam.state

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

enum class ThemePref { System, Light, Dark }
enum class MotionPref { System, Full, Reduced }
enum class SoundPref { Loud, Soft, Off }

/** Settings that only the app needs (the engine keeps its own): onboarding done, theme, motion, sounds. */
interface Prefs {
    var onboarded: Boolean
    var theme: ThemePref
    var motion: MotionPref
    /** Soft by default. */
    var sounds: SoundPref
}

/** Saved in SharedPreferences; the Compose state makes theme and motion switch live. */
class SharedPrefs(context: Context) : Prefs {
    private val sp = context.getSharedPreferences("nhako", Context.MODE_PRIVATE)
    private var themeState by mutableStateOf(runCatching { ThemePref.valueOf(sp.getString("theme", null) ?: "System") }.getOrDefault(ThemePref.System))
    private var motionState by mutableStateOf(runCatching { MotionPref.valueOf(sp.getString("motion", null) ?: "System") }.getOrDefault(MotionPref.System))
    // "sounds" was an on/off switch before Loud, Soft and Off.
    private var soundsState by mutableStateOf(
        runCatching { SoundPref.valueOf(sp.getString("soundLevel", null) ?: "") }.getOrNull()
            ?: if (sp.getBoolean("sounds", true)) SoundPref.Soft else SoundPref.Off,
    )

    override var onboarded: Boolean
        get() = sp.getBoolean("onboarded", false)
        set(v) = sp.edit().putBoolean("onboarded", v).apply()
    override var theme: ThemePref
        get() = themeState
        set(v) {
            themeState = v
            sp.edit().putString("theme", v.name).apply()
        }
    override var motion: MotionPref
        get() = motionState
        set(v) {
            motionState = v
            sp.edit().putString("motion", v.name).apply()
        }
    override var sounds: SoundPref
        get() = soundsState
        set(v) {
            soundsState = v
            sp.edit().putString("soundLevel", v.name).apply()
        }
}

/** For previews and tests. */
class MemoryPrefs(onboarded: Boolean = true) : Prefs {
    override var onboarded by mutableStateOf(onboarded)
    override var theme by mutableStateOf(ThemePref.System)
    override var motion by mutableStateOf(MotionPref.System)
    override var sounds by mutableStateOf(SoundPref.Soft)
}
