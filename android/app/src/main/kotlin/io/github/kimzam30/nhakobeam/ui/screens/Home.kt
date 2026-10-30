package io.github.kimzam30.nhakobeam.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.slideInVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
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
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
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
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.Screen
import io.github.kimzam30.nhakobeam.state.Sheet
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.components.BannerTone
import io.github.kimzam30.nhakobeam.ui.components.largeText
import io.github.kimzam30.nhakobeam.ui.components.Bolt
import io.github.kimzam30.nhakobeam.ui.components.BoltMode
import io.github.kimzam30.nhakobeam.ui.components.ButtonSize
import io.github.kimzam30.nhakobeam.ui.components.ButtonVariant
import io.github.kimzam30.nhakobeam.ui.components.FileStack
import io.github.kimzam30.nhakobeam.ui.components.NBanner
import io.github.kimzam30.nhakobeam.ui.components.NButton
import io.github.kimzam30.nhakobeam.ui.components.NIconButton
import io.github.kimzam30.nhakobeam.ui.components.Radar
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
    val transfers = app.visibleTransfers.take(2)

    fun pick(deviceId: String) {
        val d = app.device(deviceId) ?: return
        if (!d.reachable && !d.wakeable) return
        if (app.hasStaged) app.sendTo(deviceId) else {
            app.pickTarget = deviceId
            app.platform.pickFiles()
        }
    }

    Column(
        Modifier.fillMaxSize().background(c.canvas).windowInsetsPadding(WindowInsets.statusBars).windowInsetsPadding(WindowInsets.navigationBars)
            .padding(horizontal = NhakoSpace.s5).padding(top = NhakoSpace.s5, bottom = NhakoSpace.s4),
        verticalArrangement = Arrangement.spacedBy(NhakoSpace.s4),
    ) {
        Row(verticalAlignment = Alignment.Top) {
            Column(Modifier.weight(1f)) {
                BasicText(t("home.title"), style = NhakoType.largeTitle.style(c.textPrimary))
                BasicText(app.selfName.ifBlank { t("home.orb.you") }, style = NhakoType.callout.style(c.textSecondary))
            }
            NIconButton("clock-counter-clockwise", t("history.title"), { app.navigate(Screen.History) }, size = 24.dp)
            NIconButton("gear-six", t("settings.title"), { app.navigate(Screen.Settings) }, size = 24.dp)
        }

        // Banners: what's wrong right now, with a way to fix it.
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

        BoxWithConstraints(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
            val size = minOf(maxWidth - 16.dp, maxHeight - 24.dp, 420.dp).coerceAtLeast(220.dp)
            Radar(app, size, onPick = ::pick, onMore = { app.sheet = Sheet.Send })
        }

        if (nothingNearby && transfers.isEmpty()) {
            // On mobile data nearby can't work, so say why and offer the way that does.
            val noLan = !app.onLan && app.discovery != DiscoveryState.Offline
            Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
                BasicText(t(if (noLan) "home.empty.noLan.title" else "home.empty.title"), style = NhakoType.headline.style(c.textPrimary))
                BasicText(t(if (noLan) "home.empty.noLan.body" else "home.empty.body"), Modifier.padding(horizontal = NhakoSpace.s5), style = NhakoType.callout.style(c.textSecondary).copy(textAlign = TextAlign.Center))
                if (noLan) {
                    Spacer(Modifier.height(NhakoSpace.s2))
                    NButton(t("home.empty.noLan.action"), { app.sheet = Sheet.ShowCode }, variant = ButtonVariant.Secondary, size = ButtonSize.Sm, icon = "qr-code")
                }
            }
        }

        for (tr in transfers) {
            androidx.compose.runtime.key(tr.id) {
                AnimatedVisibility(true, enter = scaleIn(NhakoMotion.smooth.spec(), initialScale = NhakoMotion.Scale.enterFrom) + fadeIn()) {
                    TransferCard(app, tr, onDetails = { app.sheet = Sheet.TransferDetails(tr.id) })
                }
            }
        }

        NButton(
            if (app.hasStaged) t("send.choose.title") + "…" else t("home.action.sendFiles"),
            { if (app.hasStaged) app.sheet = Sheet.Send else app.platform.pickFiles() },
            full = true, size = ButtonSize.Lg, icon = "paper-plane-tilt", iconWeight = IconWeight.Fill,
        )
        if (!app.hasStaged) Row(Modifier.offset(y = (-4).dp)) {
            NButton(t("home.action.folder"), { app.platform.pickFolder() }, Modifier.weight(1f), variant = ButtonVariant.Plain, size = ButtonSize.Sm, icon = "folder-simple", full = true, padding = NhakoSpace.s2)
            NButton(t("home.action.text"), { app.sheet = Sheet.Compose() }, Modifier.weight(1f), variant = ButtonVariant.Plain, size = ButtonSize.Sm, icon = "text-aa", full = true, padding = NhakoSpace.s2)
            NButton(t("home.action.clipboard"), { app.stageClipboard() }, Modifier.weight(1f), variant = ButtonVariant.Plain, size = ButtonSize.Sm, icon = "clipboard-text", full = true, padding = NhakoSpace.s2)
        }
        // Side by side, or one above the other when the system font is large.
        if (largeText()) {
            NButton(t("home.action.sendCode"), { app.sheet = Sheet.ShowCode }, variant = ButtonVariant.Secondary, icon = "qr-code", full = true, padding = NhakoSpace.s4)
            NButton(t("home.action.receiveCode"), { app.sheet = Sheet.Scan }, variant = ButtonVariant.Secondary, icon = "download-simple", full = true, padding = NhakoSpace.s4)
        } else {
            Row(horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
                NButton(t("home.action.sendCode"), { app.sheet = Sheet.ShowCode }, Modifier.weight(1f), variant = ButtonVariant.Secondary, icon = "qr-code", full = true, padding = NhakoSpace.s4)
                NButton(t("home.action.receiveCode"), { app.sheet = Sheet.Scan }, Modifier.weight(1f), variant = ButtonVariant.Secondary, icon = "download-simple", full = true, padding = NhakoSpace.s4)
            }
        }
    }
}
