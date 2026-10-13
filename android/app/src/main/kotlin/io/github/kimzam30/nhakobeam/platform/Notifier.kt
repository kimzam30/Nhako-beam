package io.github.kimzam30.nhakobeam.platform

import android.Manifest
import android.annotation.SuppressLint
import android.app.Notification
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.media.AudioAttributes
import android.net.Uri
import android.os.Build
import androidx.core.app.NotificationChannelCompat
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.core.net.toUri
import io.github.kimzam30.nhakobeam.MainActivity
import io.github.kimzam30.nhakobeam.R
import io.github.kimzam30.nhakobeam.design.NhakoSound
import io.github.kimzam30.nhakobeam.model.IncomingRequest
import io.github.kimzam30.nhakobeam.model.Transfer
import io.github.kimzam30.nhakobeam.model.TransferDirection
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.formatEta
import io.github.kimzam30.nhakobeam.ui.formatSpeed
import io.github.kimzam30.nhakobeam.ui.isLink
import io.github.kimzam30.nhakobeam.ui.percent
import io.github.kimzam30.nhakobeam.ui.singleFolder
import io.github.kimzam30.nhakobeam.ui.summarizeFiles
import io.github.kimzam30.nhakobeam.ui.textPreview

/**
 * The app's notifications: file requests (with Accept and Decline), the progress of running
 * transfers (shown by [TransferService]) and files received while the app was out of sight.
 */
class Notifier(private val context: Context) {
    private val manager = NotificationManagerCompat.from(context)

    init {
        // A channel's sound is fixed once it exists, so the channels with Nhako Beam's own sounds
        // have new ids and the one from before them goes.
        manager.deleteNotificationChannel(OLD_CHANNEL_REQUESTS)
        manager.createNotificationChannelsCompat(
            listOf(
                NotificationChannelCompat.Builder(CHANNEL_REQUESTS, NotificationManagerCompat.IMPORTANCE_HIGH)
                    .setName(t("notify.channel.requests")).setSound(soundUri(NhakoSound.Incoming), SOUND_ATTRIBUTES).build(),
                NotificationChannelCompat.Builder(CHANNEL_RECEIVED, NotificationManagerCompat.IMPORTANCE_DEFAULT)
                    .setName(t("notify.channel.received")).setSound(soundUri(NhakoSound.Done), SOUND_ATTRIBUTES).build(),
                NotificationChannelCompat.Builder(CHANNEL_TRANSFERS, NotificationManagerCompat.IMPORTANCE_LOW)
                    .setName(t("notify.channel.transfers")).setShowBadge(false).build(),
            ),
        )
    }

    private fun soundUri(sound: NhakoSound): Uri = "android.resource://${context.packageName}/${sound.res}".toUri()

    private val canPost: Boolean
        get() = manager.areNotificationsEnabled() &&
            (Build.VERSION.SDK_INT < 33 || ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED)

    @SuppressLint("MissingPermission")
    private fun post(id: Int, n: Notification) {
        if (canPost) manager.notify(id, n)
    }

    fun cancel(id: Int) = manager.cancel(id)

    /** Opens the app where it was. */
    private fun openApp(): PendingIntent = PendingIntent.getActivity(
        context, 0,
        Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )

    private fun action(action: String, id: String, label: String): NotificationCompat.Action {
        val intent = Intent(context, ActionReceiver::class.java).setAction(action).putExtra(ActionReceiver.EXTRA_ID, id)
        val pi = PendingIntent.getBroadcast(context, (action + id).hashCode(), intent, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        return NotificationCompat.Action.Builder(0, label, pi).build()
    }

    // ---------- Requests ----------

    fun showRequest(r: IncomingRequest, deviceName: String) {
        val n = NotificationCompat.Builder(context, CHANNEL_REQUESTS)
            .setSmallIcon(R.drawable.ic_notify)
            .setColor(ACCENT)
            .setContentTitle(t("notify.request.title", "device" to deviceName, "what" to what(r.files, r.text)))
            .setContentText(r.text?.let { textPreview(it, 80) } ?: t("notify.request.body"))
            .setCategory(NotificationCompat.CATEGORY_MESSAGE)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setContentIntent(openApp())
            .setAutoCancel(true)
            .addAction(action(ActionReceiver.DECLINE, r.id, t("incoming.decline")))
            .addAction(action(ActionReceiver.ACCEPT, r.id, t("incoming.accept")))
            .build()
        post(requestId(r.id), n)
    }

    /** "a link", "some text", "the folder Trip", or the files' summary. */
    private fun what(files: List<io.github.kimzam30.nhakobeam.model.FileItem>, text: String?): String = when {
        text != null && isLink(text) -> t("notify.request.link")
        text != null -> t("notify.request.text")
        else -> singleFolder(files)?.let { "the folder ${it.first}" } ?: summarizeFiles(files)
    }

    /** Text from one of your own devices went straight to the clipboard while the app was out of sight. */
    fun showCopied(tr: Transfer, deviceName: String) {
        val text = tr.text ?: return
        val n = NotificationCompat.Builder(context, CHANNEL_RECEIVED)
            .setSmallIcon(R.drawable.ic_notify)
            .setColor(ACCENT)
            .setContentTitle(t("notify.copied.title", "what" to if (isLink(text)) t("notify.request.link") else t("staged.text").lowercase(), "device" to deviceName))
            .setContentText(textPreview(text, 80))
            .setContentIntent(openApp())
            .setAutoCancel(true)
            .build()
        post(doneId(tr.id), n)
    }

    /** Another device asks to trust this phone: open the app to check the code and answer. */
    fun showTrustAsk(deviceId: String, deviceName: String) {
        val n = NotificationCompat.Builder(context, CHANNEL_REQUESTS)
            .setSmallIcon(R.drawable.ic_notify)
            .setColor(ACCENT)
            .setContentTitle(t("notify.trust.title", "device" to deviceName))
            .setContentText(t("notify.trust.body"))
            .setCategory(NotificationCompat.CATEGORY_MESSAGE)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setContentIntent(openApp())
            .setAutoCancel(true)
            .build()
        post(trustId(deviceId), n)
    }

    // ---------- Progress ----------

    /** The foreground notification: the newest running transfer, plus how many more are going. */
    fun progress(active: List<Transfer>, deviceName: (String) -> String): Notification {
        val tr = active.first()
        val name = deviceName(tr.deviceId)
        val pct = percent(tr.doneBytes, tr.totalBytes)
        val title = t(if (tr.direction == TransferDirection.Send) "transfer.sending" else "transfer.receiving", "device" to name)
        val text = when (tr.status) {
            TransferStatus.Paused -> "$pct% · ${t("notify.transfer.paused")}"
            TransferStatus.Transferring -> {
                val eta = formatEta(tr.totalBytes - tr.doneBytes, tr.speed)
                listOfNotNull("$pct%", formatSpeed(tr.speed).takeIf { tr.speed > 0 }, eta?.let { t("transfer.eta", "time" to it) }).joinToString(" · ")
            }
            TransferStatus.Waiting -> t("send.waiting", "device" to name)
            TransferStatus.Reconnecting -> t("transfer.reconnecting")
            else -> t("transfer.connecting")
        }
        val b = NotificationCompat.Builder(context, CHANNEL_TRANSFERS)
            .setSmallIcon(R.drawable.ic_notify)
            .setColor(ACCENT)
            .setContentTitle(title)
            .setContentText(text)
            .setSubText(if (active.size > 1) "+${active.size - 1}" else null)
            .setCategory(NotificationCompat.CATEGORY_PROGRESS)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setSilent(true)
            .setContentIntent(openApp())
            .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)
        when (tr.status) {
            TransferStatus.Transferring, TransferStatus.Paused -> b.setProgress(100, pct, false)
            else -> b.setProgress(0, 0, true)
        }
        when (tr.status) {
            TransferStatus.Transferring -> b.addAction(action(ActionReceiver.PAUSE, tr.id, t("transfer.pause")))
            TransferStatus.Paused -> b.addAction(action(ActionReceiver.RESUME, tr.id, t("transfer.resume")))
            else -> {}
        }
        b.addAction(action(ActionReceiver.CANCEL, tr.id, t("send.cancel")))
        return b.build()
    }

    fun updateProgress(n: Notification) = post(PROGRESS_ID, n)

    // ---------- Ready to receive ----------

    /** The foreground notification while nothing moves but the phone is kept ready: from the Quick
     *  Settings tile ([woken] false, a few minutes), or after another device woke it up. */
    fun ready(woken: Boolean, minutesLeft: Int): Notification =
        NotificationCompat.Builder(context, CHANNEL_TRANSFERS)
            .setSmallIcon(R.drawable.ic_notify)
            .setColor(ACCENT)
            .setContentTitle(t(if (woken) "notify.woken.title" else "notify.ready.title"))
            .setContentText(if (woken) t("notify.woken.body") else t("notify.ready.body", "minutes" to minutesLeft.coerceAtLeast(1)))
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setSilent(true)
            .setContentIntent(openApp())
            .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)
            .addAction(action(ActionReceiver.STOP_READY, "ready", t("notify.ready.stop")))
            .build()

    // ---------- Done ----------

    fun showReceived(tr: Transfer, deviceName: String) {
        val n = NotificationCompat.Builder(context, CHANNEL_RECEIVED)
            .setSmallIcon(R.drawable.ic_notify)
            .setColor(ACCENT)
            .setContentTitle(t("notify.done.title", "device" to deviceName, "what" to what(tr.files, null)))
            .setContentText(t("notify.done.body"))
            .setContentIntent(openApp())
            .setAutoCancel(true)
            .build()
        post(doneId(tr.id), n)
    }

    companion object {
        /** File requests, with the incoming chime (design/sounds/). */
        const val CHANNEL_REQUESTS = "requests.chime"
        /** "Got files", with the done chime. */
        const val CHANNEL_RECEIVED = "received"
        private const val OLD_CHANNEL_REQUESTS = "requests"
        private val SOUND_ATTRIBUTES = AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_NOTIFICATION)
            .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
            .build()
        const val CHANNEL_TRANSFERS = "transfers"
        const val PROGRESS_ID = 1
        private const val ACCENT = 0xFF1A6FE0.toInt()

        fun requestId(id: String) = 1000 + (id.hashCode() and 0xFFFFF)
        fun doneId(id: String) = 2_000_000 + (id.hashCode() and 0xFFFFF)
        fun trustId(deviceId: String) = 4_000_000 + (deviceId.hashCode() and 0xFFFFF)
    }
}
