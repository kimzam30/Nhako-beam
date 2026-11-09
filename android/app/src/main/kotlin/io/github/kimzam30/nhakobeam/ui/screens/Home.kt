package io.github.kimzam30.nhakobeam.ui.screens

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.LinearOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
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
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.model.Connection
import io.github.kimzam30.nhakobeam.model.DiscoveryState
import io.github.kimzam30.nhakobeam.model.ServerProblem
import io.github.kimzam30.nhakobeam.platform.LocalNetwork
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.HomeStatus
import io.github.kimzam30.nhakobeam.state.Screen
import io.github.kimzam30.nhakobeam.state.Sheet
import io.github.kimzam30.nhakobeam.state.StatusTone
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.components.BannerTone
import io.github.kimzam30.nhakobeam.ui.components.HomePanel
import io.github.kimzam30.nhakobeam.ui.components.RoomNight
import io.github.kimzam30.nhakobeam.ui.components.RoomStage
import io.github.kimzam30.nhakobeam.ui.components.RoomTextShadow
import io.github.kimzam30.nhakobeam.ui.components.RoundAction
import io.github.kimzam30.nhakobeam.ui.components.rememberRoomData
import io.github.kimzam30.nhakobeam.ui.components.largeText
import io.github.kimzam30.nhakobeam.ui.components.Bolt
import io.github.kimzam30.nhakobeam.ui.components.BoltMode
import io.github.kimzam30.nhakobeam.ui.components.ButtonSize
import io.github.kimzam30.nhakobeam.ui.components.ButtonVariant
import io.github.kimzam30.nhakobeam.ui.components.FileStack
import io.github.kimzam30.nhakobeam.ui.components.NBanner
import io.github.kimzam30.nhakobeam.ui.components.NButton
import io.github.kimzam30.nhakobeam.ui.components.loopFloat
import io.github.kimzam30.nhakobeam.ui.components.NIconButton
import io.github.kimzam30.nhakobeam.ui.components.TransferCard
import io.github.kimzam30.nhakobeam.ui.formatBytes
import io.github.kimzam30.nhakobeam.ui.isLink
import io.github.kimzam30.nhakobeam.ui.textPreview
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.style

@Composable
fun HomeScreen(app: AppState) {
    val c = Nhako.colors
    val nothingNearby = app.discovery != DiscoveryState.Searching && app.reachable.isEmpty() && app.started
    // A send to several devices is one card (GroupCard), the rest one card each.
    val transfers = io.github.kimzam30.nhakobeam.state.activityItems(app.visibleTransfers, app.groups).take(2)

    fun pick(deviceId: String) {
        val d = app.device(deviceId) ?: return
        if (!d.reachable && !d.wakeable) return
        if (app.hasStaged) app.sendTo(deviceId) else {
            app.pickTarget = deviceId
            app.platform.pickFiles()
        }
    }

    val screen = LocalConfiguration.current
    val data = rememberRoomData()
    val layout = app.roomLayout(data, screen.screenWidthDp.toFloat(), screen.screenHeightDp.toFloat())
    var panelOpen by rememberSaveable { mutableStateOf(true) }
    // Files or text waiting for a device: the buttons come back, so the next step is in reach.
    LaunchedEffect(app.hasStaged) { if (app.hasStaged) panelOpen = true }

    Box(Modifier.fillMaxSize().background(RoomNight)) {
        RoomStage(app, layout, data, onPick = ::pick)
        Column(
            Modifier.fillMaxWidth().windowInsetsPadding(WindowInsets.statusBars).padding(start = NhakoSpace.s5, end = NhakoSpace.s4, top = NhakoSpace.s3),
            verticalArrangement = Arrangement.spacedBy(NhakoSpace.s3),
        ) {
            Row(verticalAlignment = Alignment.Top) {
                Column(Modifier.weight(1f).padding(top = NhakoSpace.s1)) {
                    StatusHeading(app.homeStatus)
                    BasicText(app.selfName.ifBlank { t("home.orb.you") }, Modifier.padding(start = 18.dp), style = NhakoType.callout.style(Color(0xFFC9D0DE)).copy(shadow = RoomTextShadow))
                }
                Row(horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s1)) {
                    RoundAction("devices", t("home.devices"), { app.sheet = Sheet.Devices }, size = 42.dp)
                    RoundAction("clock-counter-clockwise", t("history.title"), { app.navigate(Screen.History) }, size = 42.dp)
                    RoundAction("gear-six", t("settings.title"), { app.navigate(Screen.Settings) }, size = 42.dp)
                }
            }
            // Banners: what's wrong right now, with a way to fix it.
            LocalNetworkBanner(app)
            when (app.discovery) {
                DiscoveryState.Offline -> NBanner(t("home.banner.offline"), icon = "wifi-slash")
                DiscoveryState.Blocked -> NBanner(t("home.banner.discoveryBlocked"), actionLabel = t("home.banner.discoveryBlocked.fix"), onAction = { app.engine.rescan() })
                else -> if (!app.onLan) NBanner(t("home.banner.noLan"), icon = "cell-signal-full")
            }
            val share = app.linkShare
            if (share != null && app.sheet != Sheet.Link) {
                val asking = share.visitors.count { it.state == io.github.kimzam30.nhakobeam.model.VisitorState.Asking }
                NBanner(
                    if (asking > 0) "${t("link.active")} · ${t("link.active.asking", "n" to asking)}" else t("link.active"),
                    icon = "link-simple",
                    tone = BannerTone.Info,
                    actionLabel = t("text.show"),
                    onAction = { app.sheet = Sheet.Link },
                )
            }
            val update = app.update
            if (update != null && app.updateSeen != update.version) {
                NBanner(
                    t("update.available", "version" to update.version),
                    icon = "sparkle",
                    tone = BannerTone.Info,
                    actionLabel = t("update.download"),
                    onAction = { app.openUpdate() },
                    otherLabel = t("update.later"),
                    onOther = { app.hideUpdate() },
                )
            }
            if (app.serverProblem != null && app.connection is Connection.Own) {
                val token = app.serverProblem == ServerProblem.TokenRejected
                NBanner(
                    t(if (token) "home.banner.server.tokenRejected" else "home.banner.server.unreachable"),
                    icon = if (token) "warning-circle" else "hard-drives",
                    tone = BannerTone.Warning,
                    actionLabel = t("home.banner.server.fix"),
                    onAction = { app.navigate(Screen.Settings) },
                )
            }

            AnimatedVisibility(app.hasStaged, enter = slideInVertically(NhakoMotion.bouncy.spec()) { -it / 3 } + fadeIn(), exit = fadeOut()) {
                Row(
                    Modifier.fillMaxWidth().shadow(3.dp, RoundedCornerShape(NhakoRadius.lg)).clip(RoundedCornerShape(NhakoRadius.lg)).background(c.surface)
                        .padding(start = NhakoSpace.s4, end = NhakoSpace.s2, top = NhakoSpace.s3, bottom = NhakoSpace.s3),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    val text = app.stagedText
                    FileStack(app.staged, 44.dp, text = text)
                    Spacer(Modifier.width(NhakoSpace.s4))
                    Column(Modifier.weight(1f)) {
                        BasicText(text?.let { textPreview(it) } ?: app.stagedSummary, style = NhakoType.headline.style(c.textPrimary), maxLines = 1, overflow = TextOverflow.Ellipsis)
                        val what = text?.let { t(if (isLink(it)) "staged.link" else "staged.text") } ?: formatBytes(app.staged.sumOf { it.size })
                        BasicText("$what · ${t("home.staged.hint")}", style = NhakoType.footnote.style(c.textSecondary), maxLines = 1, overflow = TextOverflow.Ellipsis)
                    }
                    NButton(t("send.clear"), { app.clearStaged() }, variant = ButtonVariant.Plain, size = ButtonSize.Sm)
                }
            }

        }
        HomePanel(app, layout.rest, panelOpen, { panelOpen = it }, onPick = ::pick, Modifier.align(Alignment.BottomCenter)) {
            if (nothingNearby && transfers.isEmpty()) {
                // On mobile data nearby can't work, so say why and offer the way that does.
                val noLan = !app.onLan && app.discovery != DiscoveryState.Offline
                Column(
                    Modifier.fillMaxWidth().clip(RoundedCornerShape(NhakoRadius.lg)).background(c.surface).padding(NhakoSpace.s4),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    BasicText(t(if (noLan) "home.empty.noLan.title" else "home.empty.title"), style = NhakoType.headline.style(c.textPrimary))
                    BasicText(t(if (noLan) "home.empty.noLan.body" else "home.empty.body"), style = NhakoType.callout.style(c.textSecondary).copy(textAlign = TextAlign.Center))
                    if (noLan) {
                        Spacer(Modifier.height(NhakoSpace.s2))
                        NButton(t("home.empty.noLan.action"), { app.sheet = Sheet.ShowCode }, variant = ButtonVariant.Secondary, size = ButtonSize.Sm, icon = "qr-code")
                    }
                }
            }
            for (item in transfers) {
                androidx.compose.runtime.key(item.key) {
                    AnimatedVisibility(true, enter = scaleIn(NhakoMotion.smooth.spec(), initialScale = NhakoMotion.Scale.enterFrom) + fadeIn()) {
                        when (item) {
                            is io.github.kimzam30.nhakobeam.state.ActivityItem.Group -> io.github.kimzam30.nhakobeam.ui.components.GroupCard(app, item.id, item.transfers)
                            is io.github.kimzam30.nhakobeam.state.ActivityItem.One -> TransferCard(app, item.transfer, onDetails = { app.sheet = Sheet.TransferDetails(item.transfer.id) })
                        }
                    }
                }
            }
        }
    }
}

/** Android 17 refuses the local network until it's allowed, so nearby devices can't be found. Asks
 *  once by itself (an update from an older version, or Not now in onboarding); after that the
 *  banner asks, and once the prompt is turned down it opens the app's page in Settings instead. */
@Composable
private fun LocalNetworkBanner(app: AppState) {
    val context = LocalContext.current
    var turnedDown by remember { mutableStateOf(false) }
    val ask = rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) {
        val allowed = LocalNetwork.allowed(context)
        turnedDown = !allowed
        app.localNetworkChanged(allowed)
    }
    LaunchedEffect(app.localNetwork) {
        if (!app.localNetwork && !app.askedLocalNetwork) {
            app.askedLocalNetwork = true
            ask.launch(LocalNetwork.permissions)
        }
    }
    if (!app.localNetwork) {
        NBanner(
            t("home.banner.localNetwork"),
            icon = "broadcast",
            actionLabel = t("home.banner.localNetwork.fix"),
            onAction = { if (turnedDown) LocalNetwork.openSettings(context) else ask.launch(LocalNetwork.permissions) },
        )
    }
}

/** What the app is doing now: a dot, then the line, which rolls up to a new one when it changes.
 *  The percent ticks on its own. The desktop twin is Home.svelte's heading. */
@Composable
private fun StatusHeading(status: HomeStatus) {
    val c = Nhako.colors
    val reduced = Nhako.reducedMotion
    // It sits on the night room whatever the theme, so it's always light.
    val style = NhakoType.title2.style(Color.White).copy(shadow = RoomTextShadow)
    val line = with(LocalDensity.current) { NhakoType.title2.lineHeight.toDp() }
    Row(Modifier.testTag("home-status"), verticalAlignment = Alignment.Top, horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s2)) {
        Box(Modifier.height(line), contentAlignment = Alignment.Center) { StatusDot(status.tone, reduced) }
        AnimatedContent(
            status,
            Modifier.weight(1f, fill = false).semantics { liveRegion = LiveRegionMode.Polite },
            contentKey = { it.key },
            transitionSpec = {
                if (reduced) fadeIn(tween(NhakoMotion.Duration.fast)) togetherWith fadeOut(tween(NhakoMotion.Duration.fast))
                else (slideInVertically(NhakoMotion.smooth.spec()) { it / 2 } + fadeIn(tween(NhakoMotion.Duration.base))) togetherWith
                    (slideOutVertically(tween(NhakoMotion.Duration.fast)) { -it / 2 } + fadeOut(tween(NhakoMotion.Duration.fast))) using
                    SizeTransform(clip = true)
            },
            label = "status",
        ) { s ->
            BasicText(s.text, style = style, maxLines = 2, overflow = TextOverflow.Ellipsis)
        }
        status.percent?.let { BasicText("$it%", Modifier.clearAndSetSemantics {}, style = style.copy(color = Color(0xFF9ACBFF))) }
    }
}

/** Steady when ready, a sonar ring while looking, a heartbeat while working. */
@Composable
private fun StatusDot(tone: StatusTone, reduced: Boolean) {
    val c = Nhako.colors
    val color by animateColorAsState(
        when (tone) {
            StatusTone.Ready -> c.success
            StatusTone.Searching, StatusTone.Busy -> c.accent
            StatusTone.Warn -> c.warning
        },
        tween(NhakoMotion.Duration.base),
        label = "dot",
    )
    val ring = tone == StatusTone.Searching && !reduced
    val pulsing = tone == StatusTone.Busy && !reduced
    val sonar by loopFloat(ring, 0f, 1f, infiniteRepeatable(tween(NhakoMotion.Duration.searchingPulse, easing = LinearOutSlowInEasing)), "sonar")
    val beat by loopFloat(pulsing, 1f, 1.3f, infiniteRepeatable(tween(440, easing = FastOutSlowInEasing), RepeatMode.Reverse), "beat")
    Box(
        Modifier.size(10.dp).drawBehind {
            if (ring) drawCircle(color.copy(alpha = 0.6f * (1f - sonar)), radius = size.minDimension / 2 * (1f + 1.6f * sonar))
            drawCircle(color, radius = size.minDimension / 2 * beat)
        },
    )
}

