// Things that sit over the screen: the incoming request card, bottom sheets and toasts. Each one
// follows the finger and leaves faster than it arrives (docs/design/motion-spec.md).
package io.github.kimzam30.nhakobeam.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.design.NhakoGesture
import io.github.kimzam30.nhakobeam.design.NhakoHaptic
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.model.FileItem
import io.github.kimzam30.nhakobeam.model.IncomingRequest
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.Toast
import io.github.kimzam30.nhakobeam.state.ToastTone
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.fileKindName
import io.github.kimzam30.nhakobeam.ui.formatBytes
import io.github.kimzam30.nhakobeam.ui.summarizeFiles
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.PhIcon
import io.github.kimzam30.nhakobeam.ui.theme.style
import kotlin.math.abs
import kotlin.math.sign
import kotlinx.coroutines.launch

fun fileIcon(f: FileItem) = when (fileKindName(f)) {
    "photo" -> "image"
    "video" -> "film-strip"
    "song" -> "music-note"
    "document" -> "file-text"
    "archive" -> "file-zip"
    else -> "file"
}

/** A small fanned stack of the files, with a count badge. */
@Composable
fun FileStack(files: List<FileItem>, size: Dp = 44.dp, modifier: Modifier = Modifier) {
    val c = Nhako.colors
    Box(modifier.size(size + 10.dp), contentAlignment = Alignment.Center) {
        val shown = files.take(3)
        shown.forEachIndexed { i, f ->
            val back = shown.size - 1 - i
            Box(
                Modifier.size(size).graphicsLayer { rotationZ = back * -6f; translationX = back * -3.dp.toPx() }
                    .shadow(2.dp, RoundedCornerShape(NhakoRadius.sm)).clip(RoundedCornerShape(NhakoRadius.sm)).background(c.surface),
                contentAlignment = Alignment.Center,
            ) {
                if (i == shown.lastIndex) PhIcon(fileIcon(f), size = size * 0.45f, tint = c.accent)
            }
        }
        if (files.size > 1) {
            Box(Modifier.align(Alignment.TopEnd).size(20.dp).clip(CircleShape).background(c.accent), contentAlignment = Alignment.Center) {
                BasicText("${files.size}", style = NhakoType.caption.style(c.onAccent).copy(fontWeight = FontWeight(700)))
            }
        }
    }
}

/**
 * The incoming request. Swipe right to accept, left to decline, or use the buttons. Follows the
 * finger, tilts up to 6 degrees, buzzes when crossing the commit point, and flies out carrying
 * the release speed (or springs back).
 */
@Composable
fun IncomingCard(app: AppState, request: IncomingRequest, modifier: Modifier = Modifier) {
    val c = Nhako.colors
    val view = LocalView.current
    val density = LocalDensity.current
    val reduced = Nhako.reducedMotion
    val scope = rememberCoroutineScope()
    val device = app.device(request.deviceId)
    val name = device?.name ?: "Someone nearby"
    val noSpace = app.freeSpace in 1 until request.totalBytes
    var always by remember(request.id) { mutableStateOf(false) }
    val x = remember(request.id) { Animatable(0f) }
    val enter = remember(request.id) { Animatable(if (reduced) 1f else 0f) }
    var leaving by remember(request.id) { mutableStateOf(false) }
    var widthPx by remember { mutableFloatStateOf(1f) }
    LaunchedEffect(request.id) { if (!reduced) enter.animateTo(1f, NhakoMotion.smooth.spec()) }

    fun commit(accept: Boolean, velocity: Float = 0f) {
        if (leaving) return
        leaving = true
        (if (accept) NhakoHaptic.Accept else NhakoHaptic.Decline).perform(view)
        scope.launch {
            if (!reduced) x.animateTo((if (accept) 1 else -1) * widthPx * 1.4f, NhakoMotion.settle.spec(), initialVelocity = velocity)
            if (accept) app.accept(request.id, autoAccept = always) else app.decline(request.id)
        }
    }

    val progress = (abs(x.value) / (widthPx * NhakoGesture.swipeCommitFraction)).coerceIn(0f, 1f)
    Box(
        modifier
            .fillMaxWidth()
            .graphicsLayer {
                translationX = x.value
                translationY = (1 - enter.value) * 80.dp.toPx()
                rotationZ = (x.value / widthPx).coerceIn(-1f, 1f) * NhakoGesture.swipeMaxRotationDeg
                transformOrigin = androidx.compose.ui.graphics.TransformOrigin(0.5f, 1f)
                alpha = enter.value.coerceIn(0f, 1f)
            }
            .shadow(16.dp, RoundedCornerShape(NhakoRadius.xl))
            .clip(RoundedCornerShape(NhakoRadius.xl))
            .background(c.surfaceRaised)
            .pointerInput(request.id, noSpace) {
                if (noSpace) return@pointerInput
                widthPx = size.width.toFloat()
                val tracker = VelocityTracker()
                var past = false
                detectHorizontalDragGestures(
                    onDragStart = { tracker.resetTracking() },
                    onDragEnd = {
                        val v = tracker.calculateVelocity().x
                        val vDp = v / density.density
                        val far = abs(x.value) > widthPx * NhakoGesture.swipeCommitFraction
                        val flung = abs(vDp) > NhakoGesture.swipeCommitVelocity && sign(v) == sign(x.value)
                        if (far || flung) commit(x.value > 0, v) else scope.launch { x.animateTo(0f, NhakoMotion.settle.spec(), initialVelocity = v) }
                        past = false
                    },
                    onDragCancel = { scope.launch { x.animateTo(0f, NhakoMotion.settle.spec()) } },
                ) { change, dx ->
                    tracker.addPosition(change.uptimeMillis, change.position)
                    scope.launch { x.snapTo(x.value + dx) }
                    val nowPast = abs(x.value + dx) > widthPx * NhakoGesture.swipeCommitFraction
                    if (nowPast != past) {
                        (if (nowPast) NhakoHaptic.ThresholdActivate else NhakoHaptic.ThresholdDeactivate).perform(view)
                        past = nowPast
                    }
                }
            }
            .padding(NhakoSpace.s6),
    ) {
        Column(verticalArrangement = Arrangement.spacedBy(NhakoSpace.s4)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                DeviceOrb(device, 48.dp, showName = false)
                Spacer(Modifier.width(NhakoSpace.s4))
                Column(Modifier.weight(1f)) {
                    BasicText(t("incoming.title", "device" to name), style = NhakoType.footnote.style(c.textSecondary), maxLines = 1, overflow = TextOverflow.Ellipsis)
                    BasicText(summarizeFiles(request.files), style = NhakoType.title2.style(c.textPrimary), maxLines = 2, overflow = TextOverflow.Ellipsis)
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        BasicText(formatBytes(request.totalBytes), style = NhakoType.footnote.style(c.textSecondary))
                        device?.route?.let {
                            Spacer(Modifier.width(NhakoSpace.s3))
                            RouteBadge(it)
                        }
                    }
                }
                FileStack(request.files, 40.dp)
            }
            if (noSpace) {
                NBanner(t("incoming.noSpace", "needed" to formatBytes(request.totalBytes - app.freeSpace)), tone = BannerTone.Danger, icon = "warning-circle")
            }
            NToggle(always, t("incoming.alwaysAccept", "device" to name), { always = it })
            Row(horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
                NButton(t("incoming.decline"), { commit(false) }, Modifier.weight(1f), variant = ButtonVariant.Secondary, size = ButtonSize.Lg, full = true)
                NButton(t("incoming.accept"), { commit(true) }, Modifier.weight(1f), size = ButtonSize.Lg, full = true, enabled = !noSpace)
            }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.Center, verticalAlignment = Alignment.CenterVertically) {
                PhIcon("hand-swipe-right", size = 14.dp, tint = c.textTertiary)
                Spacer(Modifier.width(NhakoSpace.s2))
                BasicText(t("incoming.hint.gesture"), style = NhakoType.footnote.style(c.textTertiary))
            }
        }
        // Edge labels that grow as the card nears the commit point.
        BasicText(
            t("incoming.accept"),
            Modifier.align(Alignment.TopStart).alpha(if (x.value > 0) progress else 0f).clip(RoundedCornerShape(NhakoRadius.xs)).background(c.successSubtle).padding(horizontal = 8.dp, vertical = 2.dp),
            style = NhakoType.caption.style(c.success),
        )
        BasicText(
            t("incoming.decline"),
            Modifier.align(Alignment.TopEnd).alpha(if (x.value < 0) progress else 0f).clip(RoundedCornerShape(NhakoRadius.xs)).background(c.dangerSubtle).padding(horizontal = 8.dp, vertical = 2.dp),
            style = NhakoType.caption.style(c.danger),
        )
    }
}

/** The incoming request on top of everything, over a dimmed screen. */
@Composable
fun IncomingLayer(app: AppState) {
    val request = app.requests.firstOrNull() ?: return
    val c = Nhako.colors
    val dim by animateFloatAsState(1f, tween(NhakoMotion.Duration.base), label = "dim")
    Box(Modifier.fillMaxSize().background(c.scrim.copy(alpha = c.scrim.alpha * 0.6f * dim)).clickable(remember { MutableInteractionSource() }, indication = null) {}) {
        androidx.compose.runtime.key(request.id) {
            IncomingCard(app, request, Modifier.align(Alignment.BottomCenter).windowInsetsPadding(WindowInsets.navigationBars).padding(NhakoSpace.s5).widthIn(max = 560.dp))
        }
    }
}

/**
 * A bottom sheet: rises on the smooth spring over a scrim, closes by dragging it down (past 30% or
 * a flick) or tapping outside. [visible] false plays the exit, then [content] is dropped.
 */
@Composable
fun NSheet(
    visible: Boolean,
    title: String,
    onClose: () -> Unit,
    footer: (@Composable ColumnScope.() -> Unit)? = null,
    content: @Composable ColumnScope.() -> Unit,
) {
    val c = Nhako.colors
    val reduced = Nhako.reducedMotion
    val scope = rememberCoroutineScope()
    val offset = remember { Animatable(1f) }
    var heightPx by remember { mutableFloatStateOf(1f) }
    LaunchedEffect(visible) {
        if (visible) {
            if (reduced) offset.snapTo(0f) else offset.animateTo(0f, NhakoMotion.smooth.spec())
        } else {
            offset.animateTo(1f, tween(if (reduced) 0 else NhakoMotion.Duration.slow, easing = NhakoMotion.Easing.exit))
        }
    }
    if (!visible && offset.value >= 1f) return
    Box(Modifier.fillMaxSize()) {
        Box(
            Modifier.fillMaxSize().alpha((1 - offset.value).coerceIn(0f, 1f)).background(c.scrim)
                .clickable(remember { MutableInteractionSource() }, indication = null, onClick = onClose),
        )
        Column(
            Modifier.align(Alignment.BottomCenter).widthIn(max = 640.dp).fillMaxWidth()
                .graphicsLayer {
                    heightPx = size.height
                    translationY = offset.value * size.height
                }
                .clip(RoundedCornerShape(topStart = NhakoRadius.xl, topEnd = NhakoRadius.xl))
                .background(c.surfaceRaised)
                .windowInsetsPadding(WindowInsets.navigationBars),
        ) {
            // The handle: drag down to close.
            Box(
                Modifier.fillMaxWidth().height(28.dp).pointerInput(Unit) {
                    val tracker = VelocityTracker()
                    detectVerticalDragGestures(
                        onDragStart = { tracker.resetTracking() },
                        onDragEnd = {
                            val v = tracker.calculateVelocity().y / density
                            if (offset.value > NhakoGesture.sheetDismissFraction || v > NhakoGesture.sheetDismissVelocity) onClose()
                            else scope.launch { offset.animateTo(0f, NhakoMotion.settle.spec()) }
                        },
                    ) { change, dy ->
                        tracker.addPosition(change.uptimeMillis, change.position)
                        scope.launch { offset.snapTo((offset.value + dy / heightPx).coerceAtLeast(-0.03f)) }
                    }
                },
                contentAlignment = Alignment.Center,
            ) {
                Box(Modifier.size(36.dp, 5.dp).clip(CircleShape).background(c.fillPressed))
            }
            Row(Modifier.padding(start = NhakoSpace.s6, end = NhakoSpace.s3), verticalAlignment = Alignment.CenterVertically) {
                BasicText(title, Modifier.weight(1f), style = NhakoType.title2.style(c.textPrimary))
                NIconButton("x", t("common.close"), onClose)
            }
            Column(
                Modifier.weight(1f, fill = false).verticalScroll(rememberScrollState()).padding(horizontal = NhakoSpace.s6).padding(bottom = NhakoSpace.s5),
                verticalArrangement = Arrangement.spacedBy(NhakoSpace.s4),
                content = content,
            )
            if (footer != null) {
                Separator()
                Column(Modifier.padding(NhakoSpace.s5), verticalArrangement = Arrangement.spacedBy(NhakoSpace.s3), content = footer)
            }
        }
    }
}

/** Toasts at the bottom: rise on the smooth spring, leave quicker, swipe sideways to dismiss. */
@Composable
fun ToastHost(app: AppState, modifier: Modifier = Modifier) {
    val shown = remember { mutableStateListOf<Toast>() }
    val leaving = remember { mutableStateListOf<Long>() }
    // Keep toasts that just left on screen long enough to play their exit.
    LaunchedEffect(app.toasts) {
        for (t in app.toasts) if (shown.none { it.id == t.id }) shown += t
        for (t in shown.toList()) if (app.toasts.none { it.id == t.id } && t.id !in leaving) leaving += t.id
    }
    Column(modifier.fillMaxWidth().padding(horizontal = NhakoSpace.s5), verticalArrangement = Arrangement.spacedBy(NhakoSpace.s3), horizontalAlignment = Alignment.CenterHorizontally) {
        for (toast in shown) {
            androidx.compose.runtime.key(toast.id) {
                ToastView(toast, leaving = toast.id in leaving, onGone = { shown.removeAll { it.id == toast.id }; leaving.remove(toast.id) }, onDismiss = { app.dismissToast(toast.id) })
            }
        }
    }
}

@Composable
private fun ToastView(toast: Toast, leaving: Boolean, onGone: () -> Unit, onDismiss: () -> Unit) {
    val c = Nhako.colors
    val reduced = Nhako.reducedMotion
    val scope = rememberCoroutineScope()
    val enter = remember { Animatable(if (reduced) 1f else 0f) }
    val x = remember { Animatable(0f) }
    LaunchedEffect(leaving) {
        if (!leaving) {
            if (!reduced) enter.animateTo(1f, NhakoMotion.smooth.spec())
        } else {
            enter.animateTo(0f, tween(if (reduced) 0 else NhakoMotion.Duration.fast, easing = NhakoMotion.Easing.exit))
            onGone()
        }
    }
    val (icon, tint) = when (toast.tone) {
        ToastTone.Success -> "check-circle" to c.success
        ToastTone.Warning -> "warning-circle" to c.warning
        ToastTone.Danger -> "x-circle" to c.danger
        ToastTone.Neutral -> "info" to c.textSecondary
    }
    Row(
        Modifier.widthIn(max = 480.dp).fillMaxWidth()
            .graphicsLayer {
                translationY = (1 - enter.value) * 24.dp.toPx()
                translationX = x.value
                alpha = enter.value.coerceIn(0f, 1f) * (1 - abs(x.value) / (size.width.coerceAtLeast(1f)))
            }
            .pointerInput(toast.id) {
                detectHorizontalDragGestures(onDragEnd = {
                    if (abs(x.value) > size.width * 0.35f) onDismiss() else scope.launch { x.animateTo(0f, NhakoMotion.settle.spec()) }
                }) { _, dx -> scope.launch { x.snapTo(x.value + dx) } }
            }
            .shadow(8.dp, RoundedCornerShape(NhakoRadius.full)).clip(RoundedCornerShape(NhakoRadius.full)).background(c.surfaceRaised)
            .heightIn(min = 52.dp).padding(horizontal = NhakoSpace.s5, vertical = NhakoSpace.s3),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        PhIcon(icon, size = 20.dp, tint = tint, weight = IconWeight.Fill)
        Spacer(Modifier.width(NhakoSpace.s3))
        BasicText(toast.text, Modifier.weight(1f), style = NhakoType.callout.style(c.textPrimary))
        if (toast.actionLabel != null && toast.action != null) {
            BasicText(toast.actionLabel, Modifier.clickable(role = Role.Button) { toast.action.invoke(); onDismiss() }.padding(NhakoSpace.s2), style = NhakoType.callout.style(c.accentText).copy(fontWeight = FontWeight(600)))
        }
    }
}
