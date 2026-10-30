package io.github.kimzam30.nhakobeam.stage

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.os.Build
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow
import kotlin.math.sin
import kotlin.math.sqrt

/**
 * The 3D stage, drawn by the app itself: the voxel bolt on its floor and, on Home, your devices
 * around it, ripples while it searches, an arc to each device that turns up, and sparks while
 * files move. The Kotlin twin of desktop/src/lib/stage/scene.ts (which uses three.js); here each
 * frame the faces are placed, lit by a sky fill and one key light, sorted back to front and drawn
 * with one `drawVertices`. No 3D library, so the APK stays the same size and the Robolectric
 * screenshots show exactly what a phone does. motion-spec.md section 4.18.
 */
class StageScene(
    private val model: MascotModel,
    private val camera: StageCamera,
    /** Home: the floor with its guide rings. Elsewhere a soft shadow only. */
    private val home: Boolean,
    private val mascotScale: Float,
    /** Two bolts leaning in (pairing). */
    private val pair: Boolean = false,
    /** Drop in from above and land (the launch intro). */
    private val intro: Boolean = false,
) {
    class Device(val id: String, val kind: String, val at: FloatArray, val reachable: Boolean, val lifted: Boolean)

    /** The theme's colors (ARGB). */
    var floorColor = 0xffeceef2.toInt()
    var ringColor = 0x1a121418
    var accent = 0xff1a6fe0.toInt()

    var reduced = false
    var mood = MascotState.Idle
        set(v) {
            if (field == v && moodSet) return
            field = v
            moodSet = true
            if (!pair) motions.forEach { it.set(v, clock) }
        }
    private var moodSet = false
    var look: Look? = null
    /** Sparks run between the bolt and this device while files move. */
    var transferDevice: String? = null

    private val data = model.data
    private val motions = List(if (pair) 2 else 1) { MascotMotion(data) }
    private var clock = 0L
    private var born = -1L
    private var last = -1L

    // ---------- Devices ----------

    private class Shown(var d: Device, var pop: Float, var popV: Float, var lift: Float, var liftV: Float, var leaving: Boolean)
    private val devices = LinkedHashMap<String, Shown>()

    fun setDevices(list: List<Device>) {
        val ids = list.map { it.id }.toSet()
        for ((id, s) in devices) if (id !in ids) s.leaving = true
        for (d in list) {
            val s = devices[d.id]
            // Devices already there when the stage first shows stand there; later ones pop in.
            if (s == null || s.leaving) devices[d.id] = Shown(d, if (reduced || born < 0) 1f else 0f, 0f, 0f, 0f, false)
            else s.d = d
        }
    }

    // ---------- Sparks and arcs ----------

    private class Spark(var born: Long, var life: Float, val from: FloatArray, val to: FloatArray, var lift: Float, var vel: FloatArray?)
    private val sparks = ArrayList<Spark>()
    private class Arc(val born: Long, val points: List<FloatArray>)
    private val arcs = ArrayList<Arc>()

    fun arc(id: String) {
        val s = devices[id] ?: return
        if (reduced) return
        val from = boltTop()
        val to = floatArrayOf(s.d.at[0], 0.5f, s.d.at[2])
        val pts = (0..9).map { i ->
            val a = i / 9f
            val p = floatArrayOf(from[0] + (to[0] - from[0]) * a, from[1] + (to[1] - from[1]) * a + sin(a * PI.toFloat()) * 0.6f, from[2] + (to[2] - from[2]) * a)
            if (i in 1..8) {
                p[0] += (Math.random().toFloat() - 0.5f) * 0.35f
                p[1] += (Math.random().toFloat() - 0.5f) * 0.25f
            }
            p
        }
        arcs += Arc(clock, pts)
    }

    /** The top of the bolt's head, where arcs and sparks leave from. */
    private fun boltTop() = floatArrayOf(0f, 3 * mascotScale * 0.98f, 0f)

    /** Something still moves (so keep drawing frames). */
    var moving = true
        private set

    /** Only a breath or a blink: 30 frames a second look the same, for half the work. */
    val calm: Boolean get() = (mood == MascotState.Idle || mood == MascotState.Asleep || mood == MascotState.Offline) && sparks.isEmpty() && arcs.isEmpty()

    // ---------- Drawing ----------

    private val tri = TriangleBuffer()
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private var bitmap: Bitmap? = null

    /** Draws the frame at [now] (ms) into [canvas], [width] x [height] pixels. */
    fun draw(canvas: Canvas, width: Float, height: Float, now: Long) {
        if (born < 0) {
            born = now
            motions.forEach { it.set(if (pair) MascotState.Happy else mood, now) }
        }
        clock = now
        val dt = if (last < 0) 0.016f else min(0.05f, (now - last) / 1000f)
        last = now
        var busy = !reduced

        floor(canvas, width, height)
        tri.clear()

        motions.forEachIndexed { i, motion ->
            val p = if (reduced) motion.still(look) else motion.step(now, dt, if (pair) Look(if (i == 0) 0.8f else -0.8f, 0f) else look)
            val x = if (pair) (if (i == 0) -1.15f else 1.15f) else 0f
            var lift = 0f
            var squash = 1f
            if (intro && !reduced) {
                val t = (now - born) / 1000f
                if (t < 0.42f) lift = (1 - (t / 0.42f).pow(2)) * 3.2f
                else if (t < 0.64f) squash = 1 - sin((t - 0.42f) / 0.22f * PI.toFloat()) * 0.18f
            }
            if (pair) p.rz += if (i == 0) -0.08f else 0.08f
            shadow(canvas, width, height, x, 0f, 0.95f * mascotScale * (1 - min(0.5f, (p.y * mascotScale + lift) / 4f)))
            mascot(p, x, lift, squash, width, height)
            if (!reduced) {
                val target = transferDevice?.let { devices[it] }
                if (p.sparks != null && target != null) {
                    val at = floatArrayOf(target.d.at[0], 0.45f, target.d.at[2])
                    val from = boltTop()
                    sparks += if (p.sparks == "out") Spark(now, 0.85f, from, at, 0.7f + Math.random().toFloat() * 0.4f, null)
                    else Spark(now, 0.85f, at, from, 0.7f + Math.random().toFloat() * 0.4f, null)
                }
                if (p.burst) {
                    val c = boltTop()
                    for (k in 0 until 18) {
                        val a = k / 18f * 2 * PI.toFloat()
                        sparks += Spark(now, 0.8f, c, c, 0f, floatArrayOf(cos(a) * 2.2f, sin(a) * 2.2f + 1, (Math.random().toFloat() - 0.5f) * 1.4f))
                    }
                }
            }
            light = p.light
        }

        val gone = ArrayList<String>()
        for ((id, s) in devices) {
            val popTo = if (s.leaving) 0f else 1f
            val liftTo = if (s.d.lifted && !s.leaving) 0.18f else 0f
            if (reduced) {
                s.pop = popTo
                s.lift = liftTo
            } else {
                val a = 500 * (popTo - s.pop) - 13 * s.popV
                s.popV += a * dt
                s.pop += s.popV * dt
                val b = 260 * (liftTo - s.lift) - 22 * s.liftV
                s.liftV += b * dt
                s.lift += s.liftV * dt
                if (abs(popTo - s.pop) > 0.001f || abs(liftTo - s.lift) > 0.001f) busy = true
            }
            if (s.leaving && s.pop < 0.02f) gone += id
            else {
                shadow(canvas, width, height, s.d.at[0], s.d.at[2], 0.55f * s.pop.coerceAtLeast(0f))
                device(s, width, height)
            }
        }
        gone.forEach { devices.remove(it) }

        tri.sortAndDraw(canvas, paint)

        // Sparks: small yellow voxels.
        val sparkPaint = paint.apply { style = Paint.Style.FILL; shader = null }
        val it = sparks.iterator()
        while (it.hasNext()) {
            val s = it.next()
            val a = (now - s.born) / 1000f / s.life
            if (a >= 1f) {
                it.remove()
                continue
            }
            busy = true
            val pos = if (s.vel != null) {
                val tt = (now - s.born) / 1000f
                floatArrayOf(s.from[0] + s.vel!![0] * tt, s.from[1] + s.vel!![1] * tt - 4.5f * tt * tt, s.from[2] + s.vel!![2] * tt)
            } else {
                val e = if (a < 0.5f) 4 * a * a * a else 1 - (-2 * a + 2).pow(3) / 2
                floatArrayOf(s.from[0] + (s.to[0] - s.from[0]) * e, s.from[1] + (s.to[1] - s.from[1]) * e + sin(a * PI.toFloat()) * s.lift, s.from[2] + (s.to[2] - s.from[2]) * e)
            }
            val pr = camera.project(pos[0], pos[1], pos[2], width, height)
            val r = camera.scaleAt(pr.depth, height) * 0.05f
            sparkPaint.color = 0xffffd23f.toInt()
            sparkPaint.alpha = ((if (a > 0.7f) (1 - a) / 0.3f else 1f) * 255).toInt()
            canvas.save()
            canvas.rotate(a * 300, pr.x, pr.y)
            canvas.drawRect(pr.x - r, pr.y - r, pr.x + r, pr.y + r, sparkPaint)
            canvas.restore()
        }
        sparkPaint.alpha = 255

        // Arcs: a jagged yellow line with a navy shadow, struck in and then faded.
        val ai = arcs.iterator()
        while (ai.hasNext()) {
            val arc = ai.next()
            val a = (now - arc.born) / 420f
            if (a >= 1f) {
                ai.remove()
                continue
            }
            busy = true
            val shown = (arc.points.size * min(1f, a * 3)).toInt().coerceAtLeast(2)
            val path = Path()
            arc.points.take(shown).forEachIndexed { k, pt ->
                val pr = camera.project(pt[0], pt[1], pt[2], width, height)
                if (k == 0) path.moveTo(pr.x, pr.y) else path.lineTo(pr.x, pr.y)
            }
            val alpha = if (a < 0.5f) 1f else 1 - (a - 0.5f) * 2
            val line = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE; strokeCap = Paint.Cap.ROUND; strokeJoin = Paint.Join.ROUND; strokeWidth = height * 0.008f }
            line.color = 0xff1a6fe0.toInt(); line.alpha = (alpha * 255).toInt()
            canvas.save(); canvas.translate(1.5f, 2f); canvas.drawPath(path, line); canvas.restore()
            line.color = 0xffffd23f.toInt(); line.alpha = (alpha * 255).toInt()
            canvas.drawPath(path, line)
        }

        moving = busy || ripplesOn()
    }

    /** Draws into a bitmap first where the canvas can't draw triangles itself (Android 9 and older, on the GPU). */
    fun drawSafely(canvas: Canvas, width: Float, height: Float, now: Long) {
        if (Build.VERSION.SDK_INT >= 29 || !canvas.isHardwareAccelerated) return draw(canvas, width, height, now)
        val w = width.toInt().coerceAtLeast(1)
        val h = height.toInt().coerceAtLeast(1)
        val bmp = bitmap?.takeIf { it.width == w && it.height == h } ?: Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888).also { bitmap = it }
        bmp.eraseColor(0)
        draw(Canvas(bmp), width, height, now)
        canvas.drawBitmap(bmp, 0f, 0f, null)
    }

    private fun ripplesOn() = home && !reduced && devices.isEmpty() && (mood == MascotState.Looking || mood == MascotState.Asleep)

    private fun floor(canvas: Canvas, width: Float, height: Float) {
        if (!home) return
        val r = data.stage.floorRadius
        paint.style = Paint.Style.FILL
        paint.color = floorColor
        canvas.drawPath(circlePath(r, width, height), paint)
        paint.style = Paint.Style.STROKE
        paint.strokeWidth = height * 0.0035f
        paint.color = ringColor
        for (f in listOf(0.66f, 1f)) canvas.drawPath(circlePath(data.stage.slotReach * f, width, height), paint)
        if (ripplesOn()) {
            val period = if (mood == MascotState.Looking) 2.4f else 3.8f
            for (i in 0..1) {
                val a = (((clock - born) / 1000f / period + i / 2f) % 1f + 1f) % 1f
                paint.color = accent
                paint.alpha = (0.5f * (1 - a) * 255).toInt()
                canvas.drawPath(circlePath(0.7f + a * (data.stage.slotReach + 0.3f), width, height), paint)
            }
            paint.alpha = 255
        }
        paint.style = Paint.Style.FILL
    }

    private fun circlePath(r: Float, width: Float, height: Float) = Path().apply {
        for (a in 0..72) {
            val t = a / 72f * 2 * PI.toFloat()
            val p = camera.project(cos(t) * r, 0f, sin(t) * r, width, height)
            if (a == 0) moveTo(p.x, p.y) else lineTo(p.x, p.y)
        }
        close()
    }

    /** A soft dark spot on the floor under something standing at x, z. */
    private fun shadow(canvas: Canvas, width: Float, height: Float, x: Float, z: Float, radius: Float) {
        if (radius <= 0f) return
        val c = camera.project(x, 0f, z, width, height)
        val e = camera.project(x + radius, 0f, z, width, height)
        val f = camera.project(x, 0f, z + radius, width, height)
        val rx = abs(e.x - c.x)
        val ry = abs(f.y - c.y).coerceAtLeast(rx * 0.3f)
        paint.style = Paint.Style.FILL
        for ((k, alpha) in listOf(1.0f to 0.07f, 0.7f to 0.07f, 0.45f to 0.06f)) {
            paint.color = 0xff000000.toInt()
            paint.alpha = (alpha * 255).toInt()
            canvas.drawOval(RectF(c.x - rx * k, c.y - ry * k, c.x + rx * k, c.y + ry * k), paint)
        }
        paint.alpha = 255
    }

    // ---------- Faces ----------

    private var light = 1f
    private val lightDir = normalize(floatArrayOf(-4f, 8f, 6f))

    /** A face's color under the stage's light: a sky fill (brighter facing up) and the key light. */
    private fun shade(r: Float, g: Float, b: Float, nx: Float, ny: Float, nz: Float, alpha: Float): Int {
        val k = (0.62f + 0.18f * ny + 0.62f * max(0f, nx * lightDir[0] + ny * lightDir[1] + nz * lightDir[2])) * light
        fun srgb(v: Float): Int {
            val c = (v * k).coerceIn(0f, 1f)
            val s = if (c <= 0.0031308f) c * 12.92f else 1.055f * c.pow(1 / 2.4f) - 0.055f
            return (s * 255 + 0.5f).toInt().coerceIn(0, 255)
        }
        return ((alpha * 255).toInt() shl 24) or (srgb(r) shl 16) or (srgb(g) shl 8) or srgb(b)
    }

    private fun mascot(p: Pose, x: Float, lift: Float, squash: Float, width: Float, height: Float) {
        val shown = data.faces[p.face].orEmpty().toSet()
        val faceParts = setOf("eyeL", "eyeR", "mouth", "eyeXL", "eyeXR", "eyeHappyL", "eyeHappyR", "mouthOpen")
        val s = mascotScale
        val sx = s * p.sx * (if (squash < 1f) 1 + (1 - squash) / 2 else 1f)
        val sy = s * p.sy * squash
        val rot = rotation(p.rx, p.ry, p.rz)
        val ox = x
        val oy = p.y * s + lift
        for (part in model.parts) {
            val name = part.name
            var visible = !part.hidden
            var scaleX = 1f
            var scaleY = 1f
            var dx = 0f
            var dy = 0f
            var alpha = 1f
            when {
                name in faceParts -> visible = name in shown
                name == "crack" -> visible = p.crack
                name == "zzz" -> {
                    visible = p.zzzRise != null
                    dy = (p.zzzRise ?: 0f) * 6
                    alpha = p.zzzOpacity
                }
                name.startsWith("sparkle") -> {
                    val v = p.sparkles.getOrElse(name.removePrefix("sparkle").toInt() - 1) { 1f }
                    visible = v > 0f
                    scaleX = v
                    scaleY = v
                }
            }
            if (name == "eyeL" || name == "eyeR") {
                scaleY = p.eyeOpen
                dx = p.eyeShift
            }
            if (!visible || alpha <= 0.01f) continue
            val pos = part.positions
            val n = part.normals
            val col = part.colors
            val idx = part.indices
            var t = 0
            while (t < idx.size) {
                val i0 = idx[t].toInt()
                // The face's normal, turned with the bolt; faces turned away aren't drawn.
                val nn = rot.apply(n[i0 * 3], n[i0 * 3 + 1], n[i0 * 3 + 2])
                val first = world(pos, i0, part.origin, scaleX, scaleY, dx, dy, sx, sy, rot, ox, oy)
                if (facesAway(nn, first)) {
                    t += 3
                    continue
                }
                val color = shade(col[i0 * 3], col[i0 * 3 + 1], col[i0 * 3 + 2], nn[0], nn[1], nn[2], alpha)
                tri.begin(color)
                for (c in 0 until 3) {
                    val vi = idx[t + c].toInt()
                    val w = if (c == 0) first else world(pos, vi, part.origin, scaleX, scaleY, dx, dy, sx, sy, rot, ox, oy)
                    val pr = camera.project(w[0], w[1], w[2], width, height)
                    tri.vertex(pr.x, pr.y, pr.depth)
                }
                t += 3
            }
        }
    }

    /** A model corner in the world: the part's own scale and shift, the root (1/16, lifted 1.5), the bolt's pose. */
    private fun world(pos: FloatArray, i: Int, origin: FloatArray, scaleX: Float, scaleY: Float, dx: Float, dy: Float, sx: Float, sy: Float, rot: Rotation, ox: Float, oy: Float): FloatArray {
        val vx = (pos[i * 3] * scaleX + origin[0] + dx) * model.rootScale
        val vy = (pos[i * 3 + 1] * scaleY + origin[1] + dy) * model.rootScale + model.rootLift
        val vz = (pos[i * 3 + 2] * scaleX + origin[2]) * model.rootScale
        val r = rot.apply(vx * sx, vy * sy, vz * sx)
        return floatArrayOf(r[0] + ox, r[1] + oy, r[2])
    }

    private fun facesAway(n: FloatArray, p: FloatArray): Boolean {
        val e = camera.eye
        return n[0] * (e[0] - p[0]) + n[1] * (e[1] - p[1]) + n[2] * (e[2] - p[2]) <= 0f
    }

    private fun device(s: Shown, width: Float, height: Float) {
        val d = s.d
        val k = (1f / 16f) * data.stage.deviceScale * s.pop.coerceAtLeast(0.001f)
        // Each device faces you (the camera), so its screen shows.
        val ry = atan2(camera.eye[0] - d.at[0], camera.eye[2] - d.at[2])
        val turn = rotation(0f, ry, 0f)
        val colors = data.deviceColors
        val alpha = if (d.reachable) 1f else 0.55f
        for (box in data.devices[d.kind].orEmpty()) {
            val base = when (box.role) {
                "screen" -> if (d.reachable) colors.getValue("screen") else colors.getValue("screenAway")
                else -> colors.getValue(box.role)
            }
            val tilt = rotation(-box.tiltDeg * PI.toFloat() / 180f, 0f, 0f)
            val (r, g, b) = linearOf(base)
            val glow = if (box.role == "screen" && d.reachable) 1.35f else 1f
            for (f in BOX_FACES) {
                val n = turn.apply(tilt.apply(f.n[0], f.n[1], f.n[2]))
                val corners = f.corners.map { c ->
                    val local = tilt.apply(c[0] * box.size[0] / 2, c[1] * box.size[1] / 2, c[2] * box.size[2] / 2)
                    val w = turn.apply((local[0] + box.at[0]) * k, (local[1] + box.at[1]) * k, (local[2] + box.at[2]) * k)
                    floatArrayOf(w[0] + d.at[0], w[1] + s.lift, w[2] + d.at[2])
                }
                if (facesAway(n, corners[0])) continue
                val color = shade(r * glow, g * glow, b * glow, n[0], n[1], n[2], alpha)
                val projected = corners.map { camera.project(it[0], it[1], it[2], width, height) }
                // Both halves of a face sort as one, by its middle, and a screen sits just in front
                // of the case it's on, so no half of it slips behind.
                val depth = projected.sumOf { it.depth.toDouble() }.toFloat() / 4 - if (box.role == "screen") 0.05f else 0f
                for (tr in listOf(intArrayOf(0, 1, 2), intArrayOf(0, 2, 3))) {
                    tri.begin(color, depth)
                    for (c in tr) tri.vertex(projected[c].x, projected[c].y, 0f)
                }
            }
        }
    }

    private class Face(val n: FloatArray, val corners: List<FloatArray>)

    private companion object {
        val BOX_FACES = listOf(
            Face(floatArrayOf(0f, 0f, 1f), listOf(floatArrayOf(-1f, -1f, 1f), floatArrayOf(1f, -1f, 1f), floatArrayOf(1f, 1f, 1f), floatArrayOf(-1f, 1f, 1f))),
            Face(floatArrayOf(0f, 0f, -1f), listOf(floatArrayOf(1f, -1f, -1f), floatArrayOf(-1f, -1f, -1f), floatArrayOf(-1f, 1f, -1f), floatArrayOf(1f, 1f, -1f))),
            Face(floatArrayOf(1f, 0f, 0f), listOf(floatArrayOf(1f, -1f, 1f), floatArrayOf(1f, -1f, -1f), floatArrayOf(1f, 1f, -1f), floatArrayOf(1f, 1f, 1f))),
            Face(floatArrayOf(-1f, 0f, 0f), listOf(floatArrayOf(-1f, -1f, -1f), floatArrayOf(-1f, -1f, 1f), floatArrayOf(-1f, 1f, 1f), floatArrayOf(-1f, 1f, -1f))),
            Face(floatArrayOf(0f, 1f, 0f), listOf(floatArrayOf(-1f, 1f, 1f), floatArrayOf(1f, 1f, 1f), floatArrayOf(1f, 1f, -1f), floatArrayOf(-1f, 1f, -1f))),
            Face(floatArrayOf(0f, -1f, 0f), listOf(floatArrayOf(-1f, -1f, -1f), floatArrayOf(1f, -1f, -1f), floatArrayOf(1f, -1f, 1f), floatArrayOf(-1f, -1f, 1f))),
        )

        fun linearOf(argb: Int): Triple<Float, Float, Float> {
            fun l(c: Int): Float {
                val v = c / 255f
                return if (v <= 0.04045f) v / 12.92f else ((v + 0.055f) / 1.055f).pow(2.4f)
            }
            return Triple(l((argb shr 16) and 0xff), l((argb shr 8) and 0xff), l(argb and 0xff))
        }
    }
}

/** A rotation as three.js applies an Euler (x, y, z): v' = Rx · Ry · Rz · v. */
class Rotation(private val m: FloatArray) {
    fun apply(x: Float, y: Float, z: Float) = floatArrayOf(
        m[0] * x + m[1] * y + m[2] * z,
        m[3] * x + m[4] * y + m[5] * z,
        m[6] * x + m[7] * y + m[8] * z,
    )
    fun apply(v: FloatArray) = apply(v[0], v[1], v[2])
}

fun rotation(rx: Float, ry: Float, rz: Float): Rotation {
    val a = cos(rx); val b = sin(rx)
    val c = cos(ry); val d = sin(ry)
    val e = cos(rz); val f = sin(rz)
    return Rotation(
        floatArrayOf(
            c * e, -c * f, d,
            a * f + b * e * d, a * e - b * f * d, -b * c,
            b * f - a * e * d, b * e + a * f * d, a * c,
        ),
    )
}

private fun normalize(v: FloatArray): FloatArray {
    val l = sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2])
    return floatArrayOf(v[0] / l, v[1] / l, v[2] / l)
}

/** Triangles for one frame: screen corners, a depth and a color each; drawn far to near. */
private class TriangleBuffer {
    private var xy = FloatArray(6 * 4096)
    private var depth = FloatArray(4096)
    private var color = IntArray(4096)
    private var count = 0
    private var corner = 0

    fun clear() {
        count = 0
        corner = 0
    }

    /** Starts a triangle of color [c]; with [at], it sorts at that depth rather than its corners'. */
    fun begin(c: Int, at: Float? = null) {
        if (count == depth.size) {
            xy = xy.copyOf(xy.size * 2)
            depth = depth.copyOf(depth.size * 2)
            color = color.copyOf(color.size * 2)
        }
        color[count] = c
        depth[count] = at ?: 0f
        fixed = at != null
        corner = 0
    }

    private var fixed = false

    fun vertex(x: Float, y: Float, d: Float) {
        xy[count * 6 + corner * 2] = x
        xy[count * 6 + corner * 2 + 1] = y
        if (!fixed) depth[count] += d / 3
        if (++corner == 3) count++
    }

    private var order = IntArray(0)
    private var outXy = FloatArray(0)
    private var outColor = IntArray(0)

    fun sortAndDraw(canvas: Canvas, paint: Paint) {
        if (count == 0) return
        if (order.size < count) order = IntArray(count * 2)
        for (i in 0 until count) order[i] = i
        val d = depth
        val sorted = order.copyOf(count).sortedByDescending { d[it] }
        if (outXy.size < count * 6) {
            outXy = FloatArray(count * 6 * 2)
            outColor = IntArray(count * 3 * 2)
        }
        sorted.forEachIndexed { k, i ->
            System.arraycopy(xy, i * 6, outXy, k * 6, 6)
            outColor[k * 3] = color[i]
            outColor[k * 3 + 1] = color[i]
            outColor[k * 3 + 2] = color[i]
        }
        paint.shader = null
        paint.style = Paint.Style.FILL
        paint.color = 0xffffffff.toInt()
        canvas.drawVertices(Canvas.VertexMode.TRIANGLES, count * 6, outXy, 0, null, 0, outColor, 0, null, 0, 0, paint)
    }
}
