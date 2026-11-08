// Home's room and the panel of buttons over it. The Compose twin of desktop/src/components/Room.svelte.
// The room is pre-rendered art (design/room/build.mjs): a picture, the person in layers that move on
// their own, a picture per device. Only moving things redraw: dust, rain, steam, a breath, a head.
package io.github.kimzam30.nhakobeam.ui.components

import android.graphics.BitmapFactory
import android.util.Log
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.graphics.GraphicsLayerScope
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.Shadow
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.layout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.kimzam30.nhakobeam.design.NhakoHaptic
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.model.Device
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.stage.RoomData
import io.github.kimzam30.nhakobeam.stage.RoomLayout
import io.github.kimzam30.nhakobeam.stage.RoomRect
import io.github.kimzam30.nhakobeam.stage.RoomSpot
import io.github.kimzam30.nhakobeam.stage.RoomTagAt
import io.github.kimzam30.nhakobeam.stage.deviceArt
import io.github.kimzam30.nhakobeam.stage.fitRoom
import io.github.kimzam30.nhakobeam.stage.roomTags
import io.github.kimzam30.nhakobeam.ui.LocalSound
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.RoomPerson
import io.github.kimzam30.nhakobeam.state.Sheet
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.percent
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.PhIcon
import io.github.kimzam30.nhakobeam.ui.theme.style
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

val RoomNight = Color(0xFF060A1E)
val RoomTextShadow = Shadow(Color.Black.copy(alpha = 0.8f), Offset(0f, 1f), 6f)

private var roomDataCache: RoomData? = null

@Composable
fun rememberRoomData(): RoomData {
    val context = LocalContext.current
    return remember {
        roomDataCache ?: RoomData.parse(context.assets.open("room.json").use { it.reader().readText() }).also { roomDataCache = it }
    }
}

/** The room on this screen, in pixels: world units to pixels. */
private class RoomPixels(val k: Float, val ox: Float, val oy: Float) {
    fun x(w: Float) = ox + w * k
    fun y(w: Float) = oy + w * k
}

/** Pictures load off the main thread, so the first frame of Home never waits for them. */
@Composable
private fun roomImage(name: String?): ImageBitmap? {
    val context = LocalContext.current
    val image by produceState<ImageBitmap?>(null, name) {
        value = null
        if (name != null) value = withContext(Dispatchers.IO) {
            runCatching { context.assets.open("$name.webp").use { BitmapFactory.decodeStream(it) }?.asImageBitmap() }
                .onFailure { Log.w("nhako", "The room picture $name couldn't load: ${it.message}") }.getOrNull()
        }
    }
    return image
}

/** One picture placed in the room's coordinates. */
@Composable
private fun Art(image: ImageBitmap?, f: RoomPixels, r: RoomRect, h: Float = r.h, layer: GraphicsLayerScope.() -> Unit = {}) {
    if (image == null) return
    // Measured at its true size even where it runs past the screen: a plain size() would be squeezed
    // to fit, and the furniture would no longer line up with the devices standing on it.
    Image(
        image, null,
        Modifier.layout { m, _ ->
            val p = m.measure(Constraints.fixed((r.w * f.k).roundToInt(), (h * f.k).roundToInt()))
            layout(0, 0) { p.place(f.x(r.x).roundToInt(), f.y(r.y).roundToInt()) }
        }.graphicsLayer(layer),
        contentScale = ContentScale.FillBounds, filterQuality = FilterQuality.Medium,
    )
}

@Composable
fun RoomStage(app: AppState, layout: RoomLayout, data: RoomData, onPick: (String) -> Unit) {
    val reduced = Nhako.reducedMotion
    val who = if (app.prefs.roomPerson == RoomPerson.Girl) "girl" else "boy"
    val moving = app.activeTransfers.firstOrNull { it.status == TransferStatus.Transferring || it.status == TransferStatus.Connecting || it.status == TransferStatus.Reconnecting }

    // A second clock for what never stops. Tests don't run endless animations, so they draw it once.
    var clock by remember { mutableFloatStateOf(0f) }
    // Sleeping in a quiet room is slow: fewer frames while nothing moves, for the battery.
    val busy = moving != null
    // After a quiet while (AppState.still) the room holds still too, until something happens.
    val still = Nhako.still
    LaunchedEffect(reduced, busy, still) {
        if (reduced || still) return@LaunchedEffect
        while (true) {
            androidx.compose.animation.core.withInfiniteAnimationFrameMillis { ms -> clock = ms / 1000f }
            delay(if (busy) 32L else 66L)
        }
    }
    val awake by animateFloatAsState(if (moving != null) 1f else 0f, if (reduced) tween(1) else spring(dampingRatio = 0.72f, stiffness = 90f), label = "awake")
    val lit by animateFloatAsState(if (moving != null) 1f else 0f, if (reduced) tween(1) else spring(dampingRatio = 1f, stiffness = 120f), label = "lit")
    // Like a sheet on iOS: whatever opens over the room pushes it back a little.
    val recede by animateFloatAsState(if (app.sheet != null && !reduced) 0.955f else 1f, NhakoMotion.smooth.spec(), label = "recede")

    val parts = listOf("under", "torso", "head", "over", "chair").map { roomImage("$who-$it") }
    val bolt = roomImage("bolt-lit")
    val front = roomImage("front-$who")

    BoxWithConstraints(Modifier.fillMaxSize()) {
        val density = LocalDensity.current.density
        val screenWidth = maxWidth.value
        val fitted = fitRoom(data, screenWidth, maxHeight.value)
        val f = RoomPixels(fitted.k * density, fitted.ox * density, fitted.oy * density)
        val room = roomImage("room-$who-${fitted.view}")
        val appear by animateFloatAsState(if (room != null) 1f else 0f, tween(if (reduced) 1 else 420), label = "appear")
        Box(Modifier.fillMaxSize().background(RoomNight).graphicsLayer { alpha = appear; scaleX = recede; scaleY = recede }) {
            Art(room, f, data.views.getValue(fitted.view))
            // Rain falls behind the window's frame, curtains and sill, so it only shows through the glass.
            RoomRain(data, f, { clock }, reduced)
            Art(front, f, data.front)
            // Devices stand on furniture behind the person, so they're drawn first, farthest first.
            for (spot in layout.room.sortedByDescending { it.z }) key(spot.device.id) { RoomDevice(app, spot, f, data, onPick) }
            val p = data.person
            val origin = TransformOrigin(0.5f, 1f)
            val breathNow = { if (reduced) 0f else (sin(clock * 2f * PI.toFloat() / 4.2f) + 1f) / 2f * (1f - awake) }
            Art(parts[0], f, p)
            val body: GraphicsLayerScope.() -> Unit = {
                transformOrigin = origin
                scaleY = 1f + 0.016f * breathNow()
                translationY = -10f * awake * f.k
            }
            Art(parts[1], f, p, layer = body)
            Art(parts[2], f, p) {
                transformOrigin = TransformOrigin((745f - p.x) / p.w, (561f - p.y) / p.h)
                // 4 above the shoulders and hood at most, as desktop: more showed the boy's neck far too long.
                translationY = (-5f * breathNow() - 14f * awake) * f.k
                rotationZ = -3f * awake
            }
            Art(parts[3], f, p, layer = body)
            Art(parts[4], f, p)
            val b = data.boltLit
            val u = data.bolt.size / b.unit
            Art(bolt, f, RoomRect(data.bolt.x + b.x * u, data.bolt.y + b.y * u, b.w * u)) {
                alpha = lit
                transformOrigin = TransformOrigin(0.42f, 0.7f)
                val pulse = 1f + 0.05f * lit * (if (reduced) 0f else (sin(clock * 5.7f) + 1f) / 2f)
                scaleX = pulse
                scaleY = pulse
            }
            val target = moving?.let { m -> layout.room.firstOrNull { it.device.id == m.deviceId } }
            RoomMotion(data, f, { clock }, { awake }, { lit }, target, reduced)
            // Names last, so no device ever covers one.
            val tags = roomTags(data, layout.room, fitted, screenWidth)
            for (spot in layout.room) key(spot.device.id) { RoomTag(app, spot, tags.first { it.id == spot.device.id }, data) }
        }
    }
}

/** Rain on the window. It's clipped to the glass, and the window's front layer covers everything else there. */
@Composable
private fun RoomRain(data: RoomData, f: RoomPixels, clock: () -> Float, reduced: Boolean) {
    if (reduced) return
    val rain = remember { val q = java.util.Random(9); List(26) { floatArrayOf(q.nextFloat(), q.nextFloat(), 0.8f + q.nextFloat() * 0.6f) } }
    Canvas(Modifier.fillMaxSize()) {
        val t = clock()
        val k = f.k
        val w = data.window
        clipRect(f.x(w.x), f.y(w.y), f.x(w.x + w.w), f.y(w.y + w.h)) {
            for (d in rain) {
                val a = ((t / d[2] + d[1]) % 1f)
                val x = f.x(w.x + d[0] * w.w - a * 8f)
                val y = f.y(w.y + a * w.h)
                drawLine(Color(0xFF8FB2F0).copy(alpha = 0.55f), Offset(x, y), Offset(x - 3f * k, y + 15f * k), 1f * k)
            }
        }
    }
}

/** Dust in the lamp light, steam off the mug, a sleeper's z, the thread of light. */
@Composable
private fun RoomMotion(data: RoomData, f: RoomPixels, clock: () -> Float, awake: () -> Float, lit: () -> Float, target: RoomSpot?, reduced: Boolean) {
    val dust = remember { val q = java.util.Random(77); List(26) { floatArrayOf(q.nextFloat(), q.nextFloat(), 7f + q.nextFloat() * 8f, q.nextFloat() * 12f) } }
    Canvas(Modifier.fillMaxSize()) {
        val t = clock()
        val k = f.k
        if (!reduced) {
            val (apexA, apexB, footB, footA) = data.cone
            for (d in dust) {
                val a = ((t + d[3]) / d[2]) % 1f
                val along = d[1]
                val lx = apexA[0] + (footA[0] - apexA[0]) * along
                val rx = apexB[0] + (footB[0] - apexB[0]) * along
                val y = apexA[1] + (footA[1] - apexA[1]) * along - a * 70f
                val fade = if (a < 0.25f) a / 0.25f else if (a > 0.75f) (1f - a) / 0.25f else 1f
                drawCircle(Color(0xFFFFE9AD).copy(alpha = 0.8f * fade), 1.1f * k, Offset(f.x(lx + (rx - lx) * d[0] + a * 14f), f.y(y)))
            }
            for (i in 0 until 3) {
                val a = ((t / 3.6f) + i / 3f) % 1f
                val path = Path().apply {
                    val x = f.x(data.steam.x + i * 6f)
                    val y = f.y(data.steam.y - a * 30f)
                    moveTo(x, y)
                    quadraticBezierTo(x - 6f * k, y - 10f * k, x, y - 20f * k)
                    quadraticBezierTo(x + 6f * k, y - 30f * k, x, y - 40f * k)
                }
                drawPath(path, Color(0xFFFFF3D2).copy(alpha = 0.5f * sin(a * PI.toFloat())), style = Stroke(1.8f * k, cap = StrokeCap.Round))
            }
            val sleepy = 1f - awake()
            if (sleepy > 0.02f) for (i in 0 until 3) {
                val a = (t / 4f + i / 3f) % 1f
                val s = (6f + 7f * a) * k
                val x = f.x(data.z.x + a * 22f)
                val y = f.y(data.z.y - a * 42f)
                drawPath(
                    Path().apply { moveTo(x, y); lineTo(x + s, y); lineTo(x, y + s); lineTo(x + s, y + s) },
                    Color(0xFFDBE7FF).copy(alpha = sleepy * 0.9f * sin(a * PI.toFloat())), style = Stroke(1.6f * k, cap = StrokeCap.Round),
                )
            }
        }
        // While files move, a thread of light from the trophy to the device.
        val l = lit()
        if (target != null && l > 0.01f) {
            val a = Offset(f.x(data.bolt.threadX), f.y(data.bolt.threadY))
            val e = Offset(f.x((target.at.box.x0 + target.at.box.x1) / 2), f.y((target.at.box.y0 + target.at.box.y1) / 2))
            val path = Path().apply { moveTo(a.x, a.y); quadraticBezierTo((a.x + e.x) / 2, min(a.y, e.y) - 90f * k, e.x, e.y) }
            val effect = PathEffect.dashPathEffect(floatArrayOf(3f * k, 15f * k), if (reduced) 0f else -t * 26f * k)
            drawPath(path, Color(0xFFFFD23F).copy(alpha = 0.3f * l), style = Stroke(8f * k, cap = StrokeCap.Round, pathEffect = effect))
            drawPath(path, Color(0xFFFFF1A8).copy(alpha = l), style = Stroke(3f * k, cap = StrokeCap.Round, pathEffect = effect))
        }
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun RoomDevice(app: AppState, spot: RoomSpot, f: RoomPixels, data: RoomData, onPick: (String) -> Unit) {
    val d = spot.device
    val view = LocalView.current
    val density = LocalDensity.current
    val reduced = Nhako.reducedMotion
    val image = roomImage(deviceArt(spot, data))
    val art = spot.at.art
    val box = spot.at.box
    val usable = d.reachable || d.wakeable
    val tr = app.transferFor(d.id)
    // Each device springs in when it's put in the room, and hops when you press it.
    val pop = remember { Animatable(if (reduced) 1f else 0.82f) }
    LaunchedEffect(Unit) { pop.animateTo(1f, spring(dampingRatio = 0.55f, stiffness = 340f)) }
    val interaction = remember { MutableInteractionSource() }
    val pressed = remember { mutableStateOf(false) }
    LaunchedEffect(interaction) {
        interaction.interactions.collect { pressed.value = it is androidx.compose.foundation.interaction.PressInteraction.Press }
    }
    val hop by animateFloatAsState(if (pressed.value && !reduced) 1f else 0f, spring(dampingRatio = 0.5f, stiffness = 500f), label = "hop")
    Art(image, f, art) {
        alpha = if (image != null) pop.value.coerceIn(0f, 1f) else 0f
        transformOrigin = TransformOrigin((spot.at.x - art.x) / art.w, (spot.at.y - art.y) / art.h)
        scaleX = pop.value
        scaleY = pop.value
        translationY = -9f * hop * f.k
    }
    // The real button: as big as the drawn device, for touch and screen readers.
    val w = with(density) { ((box.x1 - box.x0) * f.k).toDp() }.coerceAtLeast(64.dp)
    val h = with(density) { ((box.y1 - box.y0) * f.k).toDp() }.coerceAtLeast(64.dp)
    val status = if (!usable) t("home.orb.offline") else if (!d.reachable) t("home.orb.asleep") else t("devices.ready")
    Box(
        Modifier
            .offset { IntOffset((f.x((box.x0 + box.x1) / 2) - w.toPx() / 2).roundToInt(), (f.y((box.y0 + box.y1) / 2) - h.toPx() / 2).roundToInt()) }
            .size(w, h)
            .testTag("orb-${d.id}")
            .semantics { contentDescription = "${d.name}, $status" }
            .combinedClickable(interaction, null, enabled = usable, role = Role.Button, onLongClick = {
                NhakoHaptic.DragStart.perform(view)
                app.sheet = Sheet.DeviceSheet(d.id)
            }, onClick = { onPick(d.id) }),
    )
}

/** The name, on a small solid tag, at its place. */
@Composable
private fun RoomTag(app: AppState, spot: RoomSpot, at: RoomTagAt, data: RoomData) {
    val d = spot.device
    val usable = d.reachable || d.wakeable
    val tr = app.transferFor(d.id)
    val label = tr?.takeIf { it.status != TransferStatus.Waiting }?.let { "${d.name} ${percent(it.doneBytes, it.totalBytes)}%" } ?: d.name
    Row(
        Modifier
            .layout { m, _ ->
                val p = m.measure(Constraints(maxWidth = data.tag.w.dp.roundToPx()))
                val centre = (at.x + data.tag.w / 2).dp.toPx()
                layout(p.width, p.height) { p.place((centre - p.width / 2).roundToInt(), at.y.dp.roundToPx()) }
            }
            .clip(CircleShape).background(Color(0xFF10132A)).border(0.7.dp, Color(0xFF6E7BA3), CircleShape)
            .padding(horizontal = 10.dp, vertical = 3.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (!usable) { Box(Modifier.size(7.dp).clip(CircleShape).background(Color(0xFF818A9E))); Spacer(Modifier.width(6.dp)) }
        BasicText(label, Modifier.widthIn(max = data.tag.w.dp), style = NhakoType.caption.style(if (usable) Color(0xFFF2F5FF) else Color(0xFFAAB2C7)), maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}

/** A round button like the folder one in the panel. */
@Composable
fun RoundAction(icon: String, label: String, onClick: () -> Unit, modifier: Modifier = Modifier, size: Dp = 48.dp, enabled: Boolean = true) {
    val c = Nhako.colors
    val interaction = remember { MutableInteractionSource() }
    val play = LocalSound.current
    Box(
        modifier.size(size).pressScale(interaction, 0.9f).shadow(5.dp, CircleShape).clip(CircleShape).background(c.surfaceRaised)
            .clickable(interaction, null, enabled = enabled, role = Role.Button, onClickLabel = label) { play(io.github.kimzam30.nhakobeam.design.NhakoSound.Tap); onClick() }
            .semantics { contentDescription = label },
        contentAlignment = Alignment.Center,
    ) { PhIcon(icon, size = 22.dp, tint = c.textPrimary) }
}

/**
 * The buttons over the room. It follows the finger exactly, throws with momentum (where the throw would
 * land decides where it stops), rubber-bands past its ends, and a touch catches it mid-flight. Swiped
 * away, the handle stays, with every device that isn't in the room as a bubble.
 */
@Composable
fun HomePanel(app: AppState, others: List<Device>, open: Boolean, setOpen: (Boolean) -> Unit, onPick: (String) -> Unit, modifier: Modifier = Modifier, above: @Composable ColumnScope.() -> Unit) {
    val density = LocalDensity.current
    val view = LocalView.current
    val reduced = Nhako.reducedMotion
    val scope = rememberCoroutineScope()
    var buttonsH by remember { mutableIntStateOf(0) }
    val rowH = with(density) { 78.dp.toPx() }
    val pad = with(density) { 12.dp.toPx() }
    val hasOthers = others.isNotEmpty()
    val travel = if (buttonsH == 0) 0f else if (hasOthers) buttonsH - rowH else buttonsH + pad
    val y = remember { Animatable(0f) }
    val sheetSpring = spring<Float>(dampingRatio = 0.84f, stiffness = 224f)
    val settle: (Boolean, Float) -> Unit = { want, v ->
        if (want != open) (if (want) NhakoHaptic.ThresholdActivate else NhakoHaptic.ThresholdDeactivate).perform(view)
        setOpen(want)
        scope.launch { val goal = if (want) 0f else travel; if (reduced) y.snapTo(goal) else y.animateTo(goal, sheetSpring, v) }
    }
    // Opening from elsewhere (files were staged), or the travel changing as bubbles come and go.
    LaunchedEffect(open, travel) { if (!y.isRunning) { val goal = if (open) 0f else travel; if (reduced || buttonsH == 0) y.snapTo(goal) else y.animateTo(goal, sheetSpring) } }
    val p: () -> Float = { if (travel <= 0f) 1f else 1f - (y.value / travel).coerceIn(0f, 1f) }
    Column(
        modifier.widthIn(max = 520.dp).fillMaxWidth().windowInsetsPadding(WindowInsets.navigationBars).padding(horizontal = NhakoSpace.s4).padding(bottom = 12.dp)
            // Measured outside the moving part: a finger's travel is against the screen, not the panel.
            .pointerInput(travel, open) {
                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
                    val caught = y.isRunning
                    if (caught) scope.launch { y.stop() }
                    val tracker = VelocityTracker().apply { addPosition(down.uptimeMillis, down.position) }
                    // Watched before the buttons see it: a clear vertical move becomes a drag (and
                    // cancels their press), a tap stays a tap, sideways stays with the bubble row.
                    val slop = viewConfiguration.touchSlop
                    var dx = 0f
                    var dy = 0f
                    var dragging = false
                    var raw = y.value
                    while (true) {
                        val change = awaitPointerEvent(PointerEventPass.Initial).changes.firstOrNull { it.id == down.id } ?: break
                        if (!change.pressed) break
                        val step = change.position - change.previousPosition
                        tracker.addPosition(change.uptimeMillis, change.position)
                        if (!dragging) {
                            dx += step.x
                            dy += step.y
                            if (abs(dx) > slop && abs(dx) > abs(dy)) break
                            if (abs(dy) > slop) dragging = true else continue
                        }
                        raw += step.y
                        change.consume()
                        scope.launch { y.snapTo(rubberPanel(raw, travel)) }
                    }
                    if (dragging) {
                        val v = tracker.calculateVelocity().y
                        settle(y.value + v * 0.18f <= travel / 2f, v)
                    } else if (caught) settle(open, 0f)
                }
            }
            .offset { IntOffset(0, y.value.roundToInt()) },
        verticalArrangement = Arrangement.spacedBy(NhakoSpace.s2),
    ) {
        above()
        // The handle. Tap it too, or use a screen reader's action.
        Box(
            Modifier.fillMaxWidth().height(28.dp).testTag("panel-handle").clickable(role = Role.Button, onClickLabel = t("home.panel"), interactionSource = remember { MutableInteractionSource() }, indication = null) { settle(!open, 0f) }
                .semantics { stateDescription = if (open) "open" else "closed" },
            contentAlignment = Alignment.Center,
        ) { Box(Modifier.size(44.dp, 5.dp).clip(CircleShape).background(Color.White.copy(alpha = 0.66f))) }
        Box {
            Column(
                Modifier.fillMaxWidth().onSizeChanged { buttonsH = it.height }.graphicsLayer { alpha = fadeStep(p(), hasOthers, 0); },
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) { PanelButtons(app, open, p, hasOthers) }
            // Only there once the buttons have moved away: otherwise it would sit over them and take their taps.
            val showBubbles by remember(travel) { derivedStateOf { y.value > 1f } }
            if (hasOthers && showBubbles) Bubbles(app, others, onPick, p, open)
        }
    }
}

private fun rubberPanel(raw: Float, travel: Float): Float {
    fun band(x: Float, d: Float) = (1f - 1f / (x * 0.55f / d + 1f)) * d
    return when {
        raw < 0f -> -band(-raw, 240f)
        raw > travel -> travel + band(raw - travel, 150f)
        else -> raw
    }
}

/** Where one thing is along the panel's travel: they arrive one after another, even when dragged slowly. */
private fun fadeStep(p: Float, hasOthers: Boolean, i: Int): Float {
    val base = if (hasOthers) ((p - 0.35f) / 0.65f).coerceIn(0f, 1f) else p
    val k = ((base - i * 0.055f) / (1f - 0.22f)).coerceIn(0f, 1f)
    return k * k * (3f - 2f * k)
}

@Composable
private fun PanelButtons(app: AppState, open: Boolean, p: () -> Float, hasOthers: Boolean) {
    val c = Nhako.colors
    val onLayer: (Int) -> Modifier = { i ->
        Modifier.graphicsLayer {
            val e = fadeStep(p(), hasOthers, i)
            alpha = e
            translationY = (1f - e) * 16.dp.toPx()
            val s = 0.82f + 0.18f * e
            scaleX = s
            scaleY = s
        }
    }
    Row(horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.CenterVertically) {
        NButton(
            if (app.hasStaged) t("send.choose.title") + "…" else t("home.action.sendFiles"),
            { if (app.hasStaged) app.sheet = Sheet.Send else app.platform.pickFiles() },
            Modifier.weight(1f).then(onLayer(4)), full = true, size = ButtonSize.Lg, icon = "paper-plane-tilt", iconWeight = IconWeight.Fill, enabled = open,
        )
        if (!app.hasStaged) {
            RoundAction("folder-simple", t("home.action.folder"), { app.platform.pickFolder() }, onLayer(3), enabled = open)
            RoundAction("text-aa", t("home.action.text"), { app.sheet = Sheet.Compose() }, onLayer(2), enabled = open)
            RoundAction("clipboard-text", t("home.action.clipboard"), { app.stageClipboard() }, onLayer(1), enabled = open)
        }
    }
    val your: @Composable (Modifier) -> Unit = { m -> NButton(t("home.action.yourCode"), { app.sheet = Sheet.ShowCode }, m, variant = ButtonVariant.Secondary, icon = "qr-code", full = true, padding = NhakoSpace.s4, enabled = open) }
    val scan: @Composable (Modifier) -> Unit = { m -> NButton(t("home.action.scanCode"), { app.sheet = Sheet.Scan }, m, variant = ButtonVariant.Secondary, icon = "download-simple", full = true, padding = NhakoSpace.s4, enabled = open) }
    // Side by side, or one above the other when the system font is large.
    if (largeText()) { your(onLayer(0)); scan(onLayer(0)) } else Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        your(Modifier.weight(1f).then(onLayer(0)))
        scan(Modifier.weight(1f).then(onLayer(0)))
    }
}

@Composable
private fun Bubbles(app: AppState, others: List<Device>, onPick: (String) -> Unit, p: () -> Float, open: Boolean) {
    val c = Nhako.colors
    val sending = app.activeTransfers.map { it.deviceId }
    LazyRow(
        Modifier.fillMaxWidth().graphicsLayer { alpha = (0.35f - p()).coerceAtLeast(0f) / 0.35f },
        contentPadding = PaddingValues(horizontal = 2.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        userScrollEnabled = !open,
    ) {
        items(others, key = { it.id }) { d ->
            val usable = d.reachable || d.wakeable
            val on = d.id in sending
            val interaction = remember { MutableInteractionSource() }
            Column(Modifier.width(70.dp).clickable(interaction, null, enabled = usable && !open, role = Role.Button) { onPick(d.id) }.semantics { contentDescription = d.name + if (usable) "" else ", " + t("home.orb.offline") }, horizontalAlignment = Alignment.CenterHorizontally) {
                Box(
                    Modifier.size(48.dp).pressScale(interaction, 0.9f).shadow(5.dp, CircleShape).clip(CircleShape).background(if (on) c.accent else c.surfaceRaised),
                    contentAlignment = Alignment.Center,
                ) { PhIcon(kindIcon(d.kind), size = 23.dp, tint = if (on) c.onAccent else if (usable) c.textPrimary else c.textTertiary) }
                Spacer(Modifier.height(4.dp))
                BasicText(d.name, style = NhakoType.caption.style(if (usable) Color.White else Color(0xFFAAB2C7)).copy(shadow = RoomTextShadow), maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
    }
}
