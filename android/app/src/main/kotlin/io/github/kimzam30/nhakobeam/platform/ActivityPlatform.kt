package io.github.kimzam30.nhakobeam.platform

import android.annotation.SuppressLint
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Environment
import android.os.PowerManager
import android.provider.DocumentsContract
import android.provider.Settings
import android.webkit.MimeTypeMap
import androidx.activity.ComponentActivity
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.FileProvider
import io.github.kimzam30.nhakobeam.BuildConfig
import io.github.kimzam30.nhakobeam.design.NhakoHaptic
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.PlatformHooks
import io.github.kimzam30.nhakobeam.state.ToastTone
import io.github.kimzam30.nhakobeam.ui.Copy.t
import java.io.File

/** What the app can do while a screen is showing: pick files, buzz, open and share what came in. */
class ActivityPlatform(private val activity: ComponentActivity, private val app: AppState) : PlatformHooks {
    // Registered when the activity is created, as Android requires.
    private val picker = activity.registerForActivityResult(ActivityResultContracts.GetMultipleContents()) { uris ->
        if (uris.isEmpty()) app.pickTarget = null else app.stageUris(uris.map { it.toString() })
    }

    private val folderPicker = activity.registerForActivityResult(ActivityResultContracts.OpenDocumentTree()) { tree ->
        if (tree == null) app.pickTarget = null else app.stageFolder(tree.toString())
    }

    override fun pickFolder() {
        try {
            folderPicker.launch(null)
        } catch (e: ActivityNotFoundException) {
            app.pickTarget = null
            app.toast(t("send.failed"), ToastTone.Danger)
        }
    }

    private val clipboard get() = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager

    // Android only lets the app on screen read the clipboard, which is when this is asked.
    override fun readClipboard(): String =
        runCatching { clipboard.primaryClip?.takeIf { it.itemCount > 0 }?.getItemAt(0)?.coerceToText(activity)?.toString() }.getOrNull().orEmpty()

    override fun writeClipboard(text: String): Boolean =
        runCatching { clipboard.setPrimaryClip(ClipData.newPlainText("Nhako Beam", text)) }.isSuccess

    override fun openLink(url: String) = start(Intent(Intent.ACTION_VIEW, Uri.parse(url)).addCategory(Intent.CATEGORY_BROWSABLE))

    override fun backgroundAllowed(): Boolean =
        (activity.getSystemService(Context.POWER_SERVICE) as? PowerManager)?.isIgnoringBatteryOptimizations(activity.packageName) ?: true

    // Phones that close apps to save battery (Samsung, Xiaomi and others) leave alone the ones
    // the owner allowed here. If the prompt is missing, the list of apps opens instead.
    @SuppressLint("BatteryLife")
    override fun allowBackground() {
        val ask = Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS, Uri.parse("package:${activity.packageName}"))
        try {
            activity.startActivity(ask)
        } catch (e: ActivityNotFoundException) {
            start(Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS))
        }
    }

    override fun pickFiles() {
        try {
            picker.launch("*/*")
        } catch (e: ActivityNotFoundException) {
            app.pickTarget = null
            app.toast(t("send.failed"), ToastTone.Danger)
        }
    }

    override fun haptic(h: NhakoHaptic) = h.perform(activity.window.decorView)

    override fun openFiles(paths: List<String>) {
        val files = paths.map(::File)
        // Several files: show the folder in the Files app if it can, else open the first one.
        if (files.size > 1 && openFolder(files.first().parentFile)) return
        val file = files.first()
        val uri = uriFor(file) ?: return failed()
        val view = Intent(Intent.ACTION_VIEW).setDataAndType(uri, mimeOf(file)).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        start(view)
    }

    override fun shareFiles(paths: List<String>) {
        val files = paths.map(::File)
        val uris = files.mapNotNull(::uriFor)
        if (uris.isEmpty()) return failed()
        val types = files.map(::mimeOf).distinct()
        val type = types.singleOrNull() ?: types.map { it.substringBefore('/') }.distinct().singleOrNull()?.let { "$it/*" } ?: "*/*"
        val send = if (uris.size == 1) {
            Intent(Intent.ACTION_SEND).putExtra(Intent.EXTRA_STREAM, uris[0])
        } else {
            Intent(Intent.ACTION_SEND_MULTIPLE).putParcelableArrayListExtra(Intent.EXTRA_STREAM, ArrayList(uris))
        }
        send.setType(type).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        send.clipData = ClipData.newRawUri(null, uris[0]).apply { uris.drop(1).forEach { addItem(ClipData.Item(it)) } }
        start(Intent.createChooser(send, null))
    }

    private fun openFolder(dir: File?): Boolean {
        val root = Environment.getExternalStorageDirectory().path
        val path = dir?.path?.takeIf { it.startsWith("$root/") }?.removePrefix("$root/") ?: return false
        val uri = DocumentsContract.buildDocumentUri("com.android.externalstorage.documents", "primary:$path")
        val intent = Intent(Intent.ACTION_VIEW).setDataAndType(uri, DocumentsContract.Document.MIME_TYPE_DIR)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        return try {
            activity.startActivity(intent)
            true
        } catch (e: ActivityNotFoundException) {
            false
        } catch (e: SecurityException) {
            false
        }
    }

    private fun uriFor(file: File): Uri? =
        runCatching { FileProvider.getUriForFile(activity, AUTHORITY, file) }.getOrNull()

    private fun mimeOf(file: File): String =
        MimeTypeMap.getSingleton().getMimeTypeFromExtension(file.extension.lowercase()) ?: "application/octet-stream"

    private fun start(intent: Intent) {
        try {
            activity.startActivity(intent)
        } catch (e: ActivityNotFoundException) {
            failed()
        }
    }

    private fun failed() = app.toast(t("transfer.openFailed"), ToastTone.Danger)

    companion object {
        const val AUTHORITY = "${BuildConfig.APPLICATION_ID}.files"
    }
}
