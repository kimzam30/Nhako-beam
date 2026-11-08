// The radar on Home: this device in the middle, others on two rings, a sonar while searching.
// The Compose twin of desktop/src/components/Radar.svelte and DeviceOrb.svelte.
package io.github.kimzam30.nhakobeam.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
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
import androidx.compose.runtime.setValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.design.NhakoBrand
import io.github.kimzam30.nhakobeam.design.NhakoHaptic
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.model.Device
import io.github.kimzam30.nhakobeam.model.DeviceKind
import io.github.kimzam30.nhakobeam.model.DiscoveryState
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.stage.MascotAssets
import io.github.kimzam30.nhakobeam.stage.MascotModel
import io.github.kimzam30.nhakobeam.stage.MoodInput
import io.github.kimzam30.nhakobeam.stage.StageCamera
import io.github.kimzam30.nhakobeam.stage.StageCanvas
import io.github.kimzam30.nhakobeam.stage.StageScene
import io.github.kimzam30.nhakobeam.stage.mascotMood
import io.github.kimzam30.nhakobeam.stage.nextChange
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.percent
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.PhIcon
import io.github.kimzam30.nhakobeam.ui.theme.style
import io.github.kimzam30.nhakobeam.ui.withTap
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
    /** A long press: the device's own sheet. */
    onLongClick: (() -> Unit)? = null,
    /** The 3D stage draws this device under it: the circle and its icon go clear. */
    ghost: Boolean = false,
) {
    val c = Nhako.colors
    val reduced = Nhako.reducedMotion
    /** A phone Android put to sleep: it can still be sent to, it's woken up first. */
    val asleep = !self && device != null && !device.reachable && device.wakeable
    val offline = !self && device != null && !device.reachable && !device.wakeable
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
    val waitPulse = loopFloat(waiting && !reduced, 0f, 1f, infiniteRepeatable(tween(NhakoMotion.Duration.searchingPulse, easing = LinearEasing)), "wait")
    val shownProgress by animateFloatAsState(progress ?: 0f, NhakoMotion.smooth.spec(), label = "progress")
    val interaction = remember { MutableInteractionSource() }
    val status = when {
        self -> t("home.orb.you")
        offline -> t("home.orb.offline")
        asleep -> t("home.orb.asleep")
        progress != null -> "${(progress * 100).toInt()} percent"
        waiting -> "waiting"
        else -> "ready"
    }

    Column(
        modifier
            .alpha(if (offline) 0.5f else if (asleep) 0.8f else 1f)
            .semantics { contentDescription = "$name, $status" }
            .then(
                if (onClick != null && !offline) {
                    val tap = withTap(onClick)
                    val view = LocalView.current
                    Modifier.combinedClickable(
                        interaction, indication = null, role = Role.Button, onClick = tap,
                        onLongClick = onLongClick?.let { long -> { NhakoHaptic.DragStart.perform(view); long() } },
                    )
                } else Modifier,
            ),
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
                    // A bolt rides the head of the ring.
                    if (shownProgress in 0.01f..0.995f) {
                        val a = (-90f + 360f * shownProgress) * (PI.toFloat() / 180f)
                        drawBolt(androidx.compose.ui.geometry.Offset(center.x + cos(a) * r, center.y + sin(a) * r), 12.dp.toPx(), rotation = 360f * shownProgress)
                    }
                }
            }
            if (!ghost) {
                Box(
                    Modifier.size(size).shadow(if (self) 10.dp else 4.dp, CircleShape, ambientColor = c.accent, spotColor = if (self) c.accent else c.textPrimary)
                        .clip(CircleShape).background(if (self) c.accent else c.surface),
                    contentAlignment = Alignment.Center,
                ) {
                    PhIcon(icon, size = size * 0.42f, tint = if (self) c.onAccent else c.accent, weight = if (self) IconWeight.Fill else IconWeight.Duotone)
                }
            } else Box(Modifier.size(size))
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
            if (offline || asleep) {
                BasicText(t(if (offline) "home.orb.offline" else "home.orb.asleep"), style = NhakoType.caption.style(c.textTertiary))
            }
        }
    }
}
