// The radar on Home: this device in the middle, others on two rings, a sonar while searching.
// The Compose twin of desktop/src/components/Radar.svelte and DeviceOrb.svelte.
package io.github.kimzam30.nhakobeam.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.requiredWidth
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.model.Device
import io.github.kimzam30.nhakobeam.model.DeviceKind
import io.github.kimzam30.nhakobeam.model.DiscoveryState
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.percent
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.PhIcon
import io.github.kimzam30.nhakobeam.ui.theme.style
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.min
import kotlin.math.sin
import kotlinx.coroutines.delay

fun kindIcon(kind: DeviceKind) = when (kind) {
    DeviceKind.Laptop -> "laptop"
    DeviceKind.Desktop -> "desktop-tower"
    DeviceKind.Phone -> "device-mobile"
    DeviceKind.Tablet -> "device-tablet"
}

/** Where each device sits: six on the inner ring (first one top left), the rest on the outer one. */
fun radarSlots(count: Int, size: Float): List<Pair<Float, Float>> {
    val inner = min(count, 6)
    val outer = max(0, count - 6)
    val r1 = size * 0.36f
    val r2 = size * 0.47f
    val slots = mutableListOf<Pair<Float, Float>>()
    for (i in 0 until inner) {
        val a = -PI * 0.75 + i * 2 * PI / max(inner, 1)
        slots += (cos(a) * r1).toFloat() to (sin(a) * r1).toFloat()
    }
    for (i in 0 until outer) {
        val a = -PI * 0.5 + PI / max(outer, 1) / 2 + i * 2 * PI / max(outer, 1)
        slots += (cos(a) * r2).toFloat() to (sin(a) * r2).toFloat()
    }
    return slots
}

/**
 * One device as a round orb, with its name under it. [waiting] pulses a ring, [progress] (0..1)
 * draws a progress ring, [pulse] bumps play the bouncy "it landed" scale.
 */
@Composable
fun DeviceOrb(
    device: Device?,
    size: Dp,
    modifier: Modifier = Modifier,
    self: Boolean = false,
    selfName: String = "",
    showName: Boolean = true,
    waiting: Boolean = false,
    progress: Float? = null,
    pulse: Int = 0,
    onClick: (() -> Unit)? = null,
) {
    val c = Nhako.colors
    val reduced = Nhako.reducedMotion
    val offline = !self && device != null && !device.reachable
    val name = if (self) selfName.ifBlank { t("home.orb.you") } else device?.name.orEmpty()
    val icon = if (self) "device-mobile" else kindIcon(device?.kind ?: DeviceKind.Laptop)

    // Bouncy pulse when a transfer to or from this device starts or lands.
    val bump = remember { Animatable(1f) }
    LaunchedEffect(pulse) {
        if (pulse > 0 && !reduced) {
            bump.snapTo(1.12f)
            bump.animateTo(1f, NhakoMotion.bouncy.spec())
        }
    }
    val waitPulse = rememberInfiniteTransition(label = "wait").animateFloat(0f, 1f, infiniteRepeatable(tween(NhakoMotion.Duration.searchingPulse, easing = LinearEasing)), label = "w")
    val shownProgress by animateFloatAsState(progress ?: 0f, NhakoMotion.smooth.spec(), label = "progress")
    val interaction = remember { MutableInteractionSource() }
    val status = when {
        self -> t("home.orb.you")
        offline -> t("home.orb.offline")
        progress != null -> "${(progress * 100).toInt()} percent"
        waiting -> "waiting"
        else -> "ready"
    }

    Column(
        modifier
            .alpha(if (offline) 0.5f else 1f)
            .semantics { contentDescription = "$name, $status" }
            .then(if (onClick != null && !offline) Modifier.clickable(interaction, indication = null, role = Role.Button, onClick = onClick) else Modifier),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(Modifier.size(size + 16.dp).graphicsLayer { scaleX = bump.value; scaleY = bump.value }.pressScale(interaction, 0.94f), contentAlignment = Alignment.Center) {
            Canvas(Modifier.size(size + 16.dp)) {
                val r = size.toPx() / 2 + 5.dp.toPx()
                if (waiting && !reduced) {
                    val p = waitPulse.value
                    drawCircle(c.accent.copy(alpha = 0.5f * (1 - p)), radius = r + p * 6.dp.toPx(), style = Stroke(2.dp.toPx()))
                }
                if (progress != null) {
                    drawCircle(c.beamTrack, radius = r, style = Stroke(3.dp.toPx()))
                    drawArc(
                        c.accent, -90f, 360f * shownProgress, false,
                        topLeft = androidx.compose.ui.geometry.Offset(center.x - r, center.y - r),
                        size = androidx.compose.ui.geometry.Size(r * 2, r * 2),
                        style = Stroke(3.dp.toPx(), cap = StrokeCap.Round),
                    )
                }
            }
            Box(
                Modifier.size(size).shadow(if (self) 10.dp else 4.dp, CircleShape, ambientColor = c.accent, spotColor = if (self) c.accent else c.textPrimary)
                    .clip(CircleShape).background(if (self) c.accent else c.surface),
                contentAlignment = Alignment.Center,
            ) {
                PhIcon(icon, size = size * 0.42f, tint = if (self) c.onAccent else c.accent, weight = if (self) IconWeight.Fill else IconWeight.Duotone)
            }
            if (device?.trusted == true && !self) {
                Box(
                    Modifier.align(Alignment.Center).offset(x = size * 0.36f, y = size * 0.36f).size(18.dp).clip(CircleShape).background(c.surface),
                    contentAlignment = Alignment.Center,
                ) {
                    PhIcon("shield-check", size = 13.dp, tint = c.success, weight = IconWeight.Fill)
                }
            }
        }
        if (showName) {
            BasicText(
                name,
                Modifier.requiredWidth(size + 56.dp).padding(top = 2.dp),
                style = NhakoType.footnote.style(c.textPrimary).copy(textAlign = TextAlign.Center, fontWeight = androidx.compose.ui.text.font.FontWeight(500)),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            if (offline) {
                BasicText(t("home.orb.offline"), style = NhakoType.caption.style(c.textTertiary))
            }
        }
    }
}

/** The radar: guide rings, a sonar while searching, this device in the middle, the rest around it. */
@Composable
fun Radar(app: AppState, size: Dp, onPick: (String) -> Unit, onMore: () -> Unit, modifier: Modifier = Modifier) {
    val c = Nhako.colors
    val reduced = Nhako.reducedMotion
    val shown = app.radarDevices.take(8)
    val extra = max(0, app.radarDevices.size - 8)
    val searching = app.discovery == DiscoveryState.Searching
    val orbSize = if (size < 380.dp) 52.dp else 64.dp
    val selfSize = if (size < 380.dp) 72.dp else 96.dp
    val sweep = NhakoMotion.Duration.radarSweep * if (searching) 1 else 2
    val sonar = rememberInfiniteTransition(label = "sonar")
    val s1 = sonar.animateFloat(0f, 1f, infiniteRepeatable(tween(sweep, easing = NhakoMotion.Easing.enter)), label = "s1")
    val s2 = sonar.animateFloat(0f, 1f, infiniteRepeatable(tween(sweep, delayMillis = 0, easing = NhakoMotion.Easing.enter), initialStartOffset = androidx.compose.animation.core.StartOffset(sweep / 2)), label = "s2")

    Box(modifier.size(size), contentAlignment = Alignment.Center) {
        Canvas(Modifier.size(size)) {
            val r = this.size.minDimension / 2
            drawCircle(c.separator, radius = r * 0.66f, style = Stroke(1.dp.toPx()))
            drawCircle(c.separator.copy(alpha = c.separator.alpha * 0.6f), radius = r, style = Stroke(1.dp.toPx()))
            if ((searching || app.devices.isEmpty()) && !reduced) {
                for (p in listOf(s1.value, s2.value)) {
                    val scale = 0.9f + 2.0f * p
                    drawCircle(c.accent.copy(alpha = 0.55f * (1 - p)), radius = r * 0.34f * scale, style = Stroke(1.5.dp.toPx()))
                }
            }
        }
        DeviceOrb(null, selfSize, self = true, selfName = app.selfName, showName = false)

        val slots = radarSlots(shown.size, size.value)
        shown.forEachIndexed { i, d ->
            val (x, y) = slots.getOrElse(i) { 0f to 0f }
            val tr = app.transferFor(d.id)
            androidx.compose.runtime.key(d.id) {
                val appear = remember { Animatable(if (reduced) 1f else 0f) }
                LaunchedEffect(Unit) {
                    if (!reduced) {
                        delay((min(i, 5) * NhakoMotion.staggerStepMs).toLong())
                        appear.animateTo(1f, NhakoMotion.bouncy.spec())
                    }
                }
                Box(
                    Modifier.offset(x = x.dp, y = y.dp + 12.dp).graphicsLayer {
                        val s = 0.6f + 0.4f * appear.value
                        scaleX = s
                        scaleY = s
                        alpha = 0.4f + 0.6f * appear.value.coerceIn(0f, 1f)
                    },
                ) {
                    DeviceOrb(
                        d, orbSize,
                        waiting = tr?.status == TransferStatus.Waiting,
                        progress = tr?.takeIf { it.status != TransferStatus.Waiting }?.let { percent(it.doneBytes, it.totalBytes) / 100f },
                        pulse = app.pulse[d.id] ?: 0,
                        onClick = { onPick(d.id) },
                    )
                }
            }
        }

        if (extra > 0) {
            BasicText(
                t("home.moreDevices", "count" to extra),
                Modifier.align(Alignment.BottomCenter).clip(RoundedCornerShape(NhakoRadius.full)).background(c.fill)
                    .clickable(role = Role.Button, onClick = onMore).padding(horizontal = NhakoSpace.s4, vertical = NhakoSpace.s2),
                style = NhakoType.footnote.style(c.textPrimary),
            )
        }
        if (searching) {
            BasicText(t("home.searching"), Modifier.offset(y = selfSize / 2 + 28.dp), style = NhakoType.footnote.style(c.textSecondary))
        }
    }
}
