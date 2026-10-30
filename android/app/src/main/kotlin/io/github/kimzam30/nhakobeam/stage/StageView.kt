package io.github.kimzam30.nhakobeam.stage

import kotlin.math.hypot
import kotlin.math.tan

/**
 * The stage's camera, as plain math: where a point lands on screen. The Kotlin twin of
 * desktop/src/lib/stage/view.ts (the same as three.js's PerspectiveCamera with lookAt), so each
 * device's button sits exactly on its drawn device.
 */
class StageCamera(val eye: FloatArray, val target: FloatArray, val fovDeg: Float) {
    private val f = norm(sub(target, eye))
    private val r = norm(cross(f, floatArrayOf(0f, 1f, 0f)))
    private val u = cross(r, f)
    private val k = 1f / tan(Math.toRadians(fovDeg / 2.0)).toFloat()

    /** Screen x, y in a [width] x [height] box from its top left, and the distance from the camera. */
    fun project(x: Float, y: Float, z: Float, width: Float, height: Float): Projected {
        val dx = x - eye[0]
        val dy = y - eye[1]
        val dz = z - eye[2]
        val cx = dx * r[0] + dy * r[1] + dz * r[2]
        val cy = dx * u[0] + dy * u[1] + dz * u[2]
        val cz = dx * f[0] + dy * f[1] + dz * f[2]
        val aspect = width / height
        val nx = cx * k / aspect / cz
        val ny = cy * k / cz
        return Projected((nx + 1) / 2 * width, (1 - ny) / 2 * height, cz)
    }

    /** Pixels per unit at [depth]. */
    fun scaleAt(depth: Float, height: Float) = k * height / 2 / depth

    /** Which way the camera looks (for lighting and which faces show). */
    val forward: FloatArray get() = f

    companion object {
        fun home(s: MascotData.StageSpec) = StageCamera(s.eye, s.target, s.fovDeg)

        /** A radar slot (pixels from the middle, in a radar [size] across) as a spot on the floor. */
        fun slotOnFloor(x: Float, y: Float, size: Float, s: MascotData.StageSpec): FloatArray {
            val k = s.slotReach * 2 / size
            return floatArrayOf(x * k, 0f, y * k)
        }
    }
}

data class Projected(val x: Float, val y: Float, val depth: Float)

private fun sub(a: FloatArray, b: FloatArray) = floatArrayOf(a[0] - b[0], a[1] - b[1], a[2] - b[2])
private fun cross(a: FloatArray, b: FloatArray) = floatArrayOf(a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])
private fun norm(a: FloatArray): FloatArray {
    val l = hypot(hypot(a[0], a[1]), a[2]).takeIf { it > 0f } ?: 1f
    return floatArrayOf(a[0] / l, a[1] / l, a[2] / l)
}
