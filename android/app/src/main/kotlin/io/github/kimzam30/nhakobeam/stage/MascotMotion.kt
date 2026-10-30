package io.github.kimzam30.nhakobeam.stage

import io.github.kimzam30.nhakobeam.design.NhakoMotion
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.exp
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow
import kotlin.math.sin

/**
 * The mascot's pose for one frame: lift, turn, lean, squash, and what the face shows. The Kotlin
 * twin of desktop/src/lib/stage/pose.ts, so both apps move it the same way.
 */
data class Pose(
    var y: Float = 0f,
    var rx: Float = 0f,
    var ry: Float = 0f,
    var rz: Float = 0f,
    var sx: Float = 1f,
    var sy: Float = 1f,
    var face: String = "eyes",
    var eyeOpen: Float = 1f,
    var eyeShift: Float = 0f,
    var crack: Boolean = false,
    /** The Zs: how far they've risen (0 to 1) and how visible; null when hidden. */
    var zzzRise: Float? = null,
    var zzzOpacity: Float = 0f,
    var sparkles: FloatArray = floatArrayOf(1f, 1f, 1f),
    var light: Float = 1f,
    /** Sparks this frame: "out" (to the device) or "in". */
    var sparks: String? = null,
    var burst: Boolean = false,
)

/** Where the bolt looks, from -1 (left, up) to 1 (right, down). */
data class Look(val x: Float, val y: Float)

/** Keeps what carries from frame to frame (the next blink, the smoothed turn). */
class MascotMotion(private val data: MascotData, private val random: () -> Float = { Math.random().toFloat() }) {
    var state = MascotState.Idle
        private set
    private var since = 0L
    private var nextBlink = 0L
    private var yaw = 0f
    private var pitch = 0f
    private var burstDone = false
    private var sparkDebt = 0f

    private fun spec() = data.states.getValue(state.id)
    private fun deg(d: Float) = d * PI.toFloat() / 180f

    /** Starts [s] at [now] (ms). The same state again changes nothing. */
    fun set(s: MascotState, now: Long) {
        if (s == state && since != 0L) return
        state = s
        since = now
        burstDone = false
        nextBlink = now + 1200
    }

    /** The still pose for reduced motion. */
    fun still(look: Look? = null): Pose {
        val s = spec()
        val p = base(s.face)
        val side = if (look != null && look.x < 0) -1f else 1f
        s.pose["turnDeg"]?.let { p.ry = deg(it) * side }
        s.pose["leanDeg"]?.let { p.rz = deg(it) }
        s.pose["tiltDeg"]?.let { p.rz = deg(it) }
        if (state == MascotState.Looking) p.eyeShift = (s.motion["eyeShift"] ?: 0f) * side
        p.crack = (s.pose["crack"] ?: 0f) > 0f
        if ((s.pose["zzz"] ?: 0f) > 0f) {
            p.zzzRise = 0.3f
            p.zzzOpacity = 1f
        }
        p.sparkles = sparklesFor(state)
        p.light = s.pose["dim"] ?: 1f
        return p
    }

    /** The pose at [now] (ms), [dt] seconds after the last one, turning towards [look]. */
    fun step(now: Long, dt: Float, look: Look?): Pose {
        val s = spec()
        val m = s.motion
        val t = (now - since) / 1000f
        val p = base(s.face)
        p.sparkles = sparklesFor(state)
        p.light = s.pose["dim"] ?: 1f
        var yawTo = 0f
        var pitchTo = 0f
        val timing = NhakoMotion.Mascot

        fun breathe(ms: Float) {
            val b = sin(t * 1000 * 2 * PI.toFloat() / ms)
            p.sx = 1 - b * 0.008f
            p.sy = 1 + b * 0.016f
        }
        fun blink() {
            if (now > nextBlink) nextBlink = now + timing.blinkMinMs + (random() * (timing.blinkMaxMs - timing.blinkMinMs)).toLong()
            val into = timing.blinkMs - (nextBlink - now)
            if (into in 1 until timing.blinkMs) p.eyeOpen = 1 - sin(into.toFloat() / timing.blinkMs * PI.toFloat()) * 0.88f
        }
        fun follow(strength: Float) {
            if (look == null) return
            yawTo += look.x * deg(timing.followDeg) * strength
            pitchTo += look.y * 0.12f * strength
        }
        fun twinkle(ms: Float) {
            p.sparkles = FloatArray(3) { i -> 0.75f + 0.35f * sin(t * 1000 * 2 * PI.toFloat() / ms + i * 2.1f) }
        }
        fun sparksAt(perSecond: Float, dir: String) {
            sparkDebt += perSecond * dt
            if (sparkDebt >= 1f) {
                sparkDebt -= 1f
                p.sparks = dir
            }
        }

        when (state) {
            MascotState.Idle -> {
                breathe(timing.breatheMs.toFloat())
                blink()
                yawTo += deg(m["swayDeg"] ?: 0f) * sin(t * 0.9f) * 0.5f
                follow(1f)
                twinkle(2200f)
            }
            MascotState.Looking -> {
                blink()
                val side = if (look != null && look.x < 0) -1f else 1f
                yawTo += deg(m["turnDeg"] ?: 0f) * side + sin(t * 0.8f) * 0.06f
                p.eyeShift = (m["eyeShift"] ?: 0f) * side
                follow(0.4f)
            }
            MascotState.Happy -> {
                val hopMs = m["hopMs"] ?: 520f
                if (t * 1000 < hopMs * (m["hops"] ?: 2f)) hop(p, (t * 1000 % hopMs) / hopMs, 0.45f, 0.78f)
                twinkle(m["twinkleMs"] ?: 900f)
                follow(0.6f)
            }
            MascotState.Sending, MascotState.Receiving -> {
                val bob = sin(t * 1000 * 2 * PI.toFloat() / (m["bobMs"] ?: 700f))
                p.y = abs(bob) * 0.05f
                p.rz = deg(m["leanDeg"] ?: 0f) * (0.85f + 0.15f * bob)
                blink()
                follow(0.7f)
                sparksAt(m["sparksPerSecond"] ?: 14f, if (state == MascotState.Sending) "out" else "in")
            }
            MascotState.Done -> {
                val a = t * 1000 / (m["jumpMs"] ?: 900f)
                if (a < 1) {
                    val up = sin(a * PI.toFloat())
                    p.y = up * 0.85f
                    yawTo += deg(m["spinDeg"] ?: 360f) * easeInOut(a)
                    p.sx = 1 - up * 0.03f
                    p.sy = 1 + up * 0.08f
                    if (a > 0.45f && !burstDone) {
                        burstDone = true
                        p.burst = true
                    }
                } else {
                    val land = max(0f, 1 - (a - 1) * 4)
                    p.sx = 1 + land * 0.06f
                    p.sy = 1 - land * 0.12f
                    if (land == 0f) breathe(timing.breatheMs.toFloat())
                }
                twinkle(700f)
            }
            MascotState.Failed -> {
                val a = t * 1000 / (m["wobbleMs"] ?: 1100f)
                val w = if (a < 1) exp(-a * 3.2f) * sin(a * PI.toFloat() * 6) else 0f
                p.rz = deg(-6f) + deg(m["wobbleDeg"] ?: 9f) * w
                p.y = -0.03f
            }
            MascotState.Asleep -> {
                breathe(m["breatheMs"] ?: 4600f)
                p.rz = deg(m["droopDeg"] ?: 7f)
                pitchTo += 0.08f
                val period = m["zzzMs"] ?: 2400f
                val a = (t * 1000 % period) / period
                p.zzzRise = a
                p.zzzOpacity = if (a < 0.15f) a / 0.15f else 1 - (a - 0.15f) / 0.85f
            }
            MascotState.Offline -> {
                breathe(m["breatheMs"] ?: 5200f)
                p.rz = deg(m["droopDeg"] ?: 4f)
                pitchTo += 0.12f
            }
            MascotState.Loading -> {
                yawTo += t * 1000 / (m["spinMs"] ?: 1400f) * PI.toFloat() * 2
                p.y = 0.08f + sin(t * 3) * 0.04f
            }
        }

        // Turns follow softly; the spins are exact.
        val k = 1 - exp(-dt * 7)
        val spinning = state == MascotState.Loading || (state == MascotState.Done && t * 1000 < (m["jumpMs"] ?: 900f))
        yaw = if (spinning) yawTo else yaw + (yawTo - yaw) * k
        pitch += (pitchTo - pitch) * k
        p.ry = yaw
        p.rx = pitch
        return p
    }

    private fun base(face: String) = Pose(face = face, eyeOpen = when (face) { "closed" -> 0.12f; "half" -> 0.5f; else -> 1f })

    private fun sparklesFor(s: MascotState): FloatArray {
        val shown = s != MascotState.Failed && s != MascotState.Offline && s != MascotState.Asleep
        return if (shown) floatArrayOf(1f, 1f, 1f) else floatArrayOf(0f, 0f, 0f)
    }

    private fun hop(p: Pose, a: Float, height: Float, phase: Float) {
        val up = sin(min(1f, a / phase) * PI.toFloat())
        p.y = up * height
        val land = if (a > phase) sin(min(1f, (a - phase) / 0.18f) * PI.toFloat()) else 0f
        val sy = 1 + up * 0.06f - land * 0.12f
        p.sy = sy
        p.sx = 1 + (1 - sy) * 0.5f
    }

    private fun easeInOut(t: Float) = if (t < 0.5f) 4 * t * t * t else 1 - (-2 * t + 2).pow(3) / 2
}
