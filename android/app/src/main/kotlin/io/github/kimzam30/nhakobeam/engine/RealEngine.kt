package io.github.kimzam30.nhakobeam.engine

import android.content.ContentResolver
import android.content.Context
import android.net.Uri
import android.os.Environment
import android.os.ParcelFileDescriptor
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import io.github.kimzam30.nhakobeam.BuildConfig
import io.github.kimzam30.nhakobeam.core.EventListener
import io.github.kimzam30.nhakobeam.core.FileOpener
import io.github.kimzam30.nhakobeam.core.NhakoEngine
import io.github.kimzam30.nhakobeam.core.NhakoException
import io.github.kimzam30.nhakobeam.core.SendFile
import io.github.kimzam30.nhakobeam.model.Connection
import io.github.kimzam30.nhakobeam.model.ContractJson
import io.github.kimzam30.nhakobeam.model.EngineEvent
import io.github.kimzam30.nhakobeam.model.FileItem
import io.github.kimzam30.nhakobeam.model.LinkShare
import io.github.kimzam30.nhakobeam.model.PairingCode
import io.github.kimzam30.nhakobeam.model.RedeemResult
import io.github.kimzam30.nhakobeam.model.SelfInfo
import io.github.kimzam30.nhakobeam.model.ServerCheck
import io.github.kimzam30.nhakobeam.model.Snapshot
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.model.WhoCanSend
import java.io.File
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext

/**
 * The Rust engine (ffi/ through UniFFI). Its calls block, so they run on [Dispatchers.IO]; its
 * events arrive on an engine thread and go out through [events].
 *
 * Files shared from other apps are `content://` URIs, not paths. Sending opens each one and hands
 * the engine `/proc/self/fd/N` with the real name; the handles stay open until the transfer is
 * finished (a failed one may be resumed, so it keeps them).
 */
class RealEngine(private val context: Context, private val scope: CoroutineScope) : Engine {
    private val _events = MutableSharedFlow<EngineEvent>(extraBufferCapacity = 512)
    override val events: SharedFlow<EngineEvent> = _events

    private val native: NhakoEngine by lazy {
        NhakoEngine(context.filesDir.resolve("engine").path, defaultSaveDir(), object : EventListener {
            override fun onEvent(json: String) = onEngineEvent(json)
        }).also { engine ->
            // F-Droid gets its updates from F-Droid: no check against GitHub there.
            engine.setFromStore(BuildConfig.FROM_STORE)
            // Files in a picked folder are opened one at a time as each is sent, never all at once.
            engine.setFileOpener(object : FileOpener {
                override fun open(uri: String): Int {
                    if (uri !in folderFiles) return -1
                    return runCatching { context.contentResolver.openFileDescriptor(Uri.parse(uri), "r")?.detachFd() }.getOrNull() ?: -1
                }
            })
        }
    }

    /** FileItem id -> a file inside a picked folder (a document content:// URI), and its folder. */
    private val folderSources = ConcurrentHashMap<String, Pair<Uri, String>>()
    /** The document URIs the engine may open: only files you picked a folder for. */
    private val folderFiles: MutableSet<String> = ConcurrentHashMap.newKeySet()

    /** FileItem id -> the URI it came from. */
    private val sources = ConcurrentHashMap<String, Uri>()
    /** Transfer id -> handles kept open while it runs. */
    private val openFiles = ConcurrentHashMap<String, List<ParcelFileDescriptor>>()
    /** Copies of shared files that came as a stream, not a file (see [send]); deleted when done. */
    private val copies = ConcurrentHashMap<String, File>()
    private val seq = AtomicLong()

    private fun onEngineEvent(json: String) {
        val event = runCatching { ContractJson.decodeFromString<EngineEvent>(json) }.getOrNull() ?: return
        if (event is EngineEvent.TransferChanged) {
            val s = event.transfer.status
            if (s == TransferStatus.Done || s == TransferStatus.Cancelled || s == TransferStatus.Declined) {
                openFiles.remove(event.transfer.id)?.forEach { runCatching { it.close() } }
                copies.remove(event.transfer.id)?.deleteRecursively()
            }
        }
        // A link stopped (Stop, or its time ran out): its files can close.
        if (event is EngineEvent.LinkShareChanged && event.share == null) closeLinkFiles()
        // This runs on the engine's own events thread, so when the app falls behind (a burst of
        // progress) it can wait here rather than drop an event, such as a transfer finishing.
        if (!_events.tryEmit(event)) runBlocking { _events.emit(event) }
    }

    /** Runs a blocking engine call off the main thread, turning its errors into [EngineException]. */
    private suspend fun <T> call(block: NhakoEngine.() -> T): T = withContext(Dispatchers.IO) {
        try {
            native.block()
        } catch (e: NhakoException.Failed) {
            throw EngineException(e.reason)
        }
    }

    /** For calls the app doesn't wait on; problems show up as events, or are logged. */
    private fun fire(block: NhakoEngine.() -> Unit) {
        scope.launch(Dispatchers.IO) {
            try {
                native.block()
            } catch (e: NhakoException.Failed) {
                android.util.Log.w("nhako", e.reason)
            }
        }
    }

    override suspend fun start(name: String?): Snapshot =
        ContractJson.decodeFromString(call { start(name) })

    override fun stop() = fire { stop() }

    override fun rescan() = fire { rescan() }

    override fun setForeground(on: Boolean) = fire { setForeground(on) }

    override suspend fun stage(uris: List<String>): Staged = withContext(Dispatchers.IO) {
        var skipped = 0
        val files = uris.mapNotNull { raw ->
            val uri = Uri.parse(raw)
            val item = runCatching { describe(uri) }.getOrNull()
            if (item == null) skipped++ else sources[item.id] = uri
            item
        }
        Staged(files, skipped)
    }

    private fun describe(uri: Uri): FileItem? {
        // Only content:// from a file picker or another app's share. A file:// address could point
        // anywhere this app can read, its own private files included, so it's left out (skipped).
        if (uri.scheme != ContentResolver.SCHEME_CONTENT) return null
        val resolver = context.contentResolver
        var name: String? = null
        var size: Long? = null
        resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)?.use { c ->
            if (c.moveToFirst()) {
                name = c.getString(0)
                size = if (c.isNull(1)) null else c.getLong(1)
            }
        }
        // Folders and things we can't read are left out.
        val bytes = size ?: resolver.openFileDescriptor(uri, "r")?.use { it.statSize }?.takeIf { it >= 0 } ?: return null
        return FileItem(
            id = "u${seq.incrementAndGet()}",
            name = name ?: uri.lastPathSegment ?: "file",
            size = bytes,
            mime = resolver.getType(uri) ?: "application/octet-stream",
        )
    }

    /** Everything inside a folder picked with the system picker, keeping its shape. Nothing is
     *  opened yet; the engine opens each file when it's sent. */
    override suspend fun stageFolder(treeUri: String): Staged = withContext(Dispatchers.IO) {
        val tree = Uri.parse(treeUri)
        if (tree.scheme != ContentResolver.SCHEME_CONTENT) return@withContext Staged(emptyList(), 1)
        val resolver = context.contentResolver
        val rootId = DocumentsContract.getTreeDocumentId(tree)
        val rootName = resolver.query(DocumentsContract.buildDocumentUriUsingTree(tree, rootId), arrayOf(DocumentsContract.Document.COLUMN_DISPLAY_NAME), null, null, null)
            ?.use { c -> if (c.moveToFirst()) c.getString(0) else null } ?: "Folder"
        val out = mutableListOf<FileItem>()
        val cols = arrayOf(
            DocumentsContract.Document.COLUMN_DOCUMENT_ID,
            DocumentsContract.Document.COLUMN_DISPLAY_NAME,
            DocumentsContract.Document.COLUMN_MIME_TYPE,
            DocumentsContract.Document.COLUMN_SIZE,
        )
        val pending = ArrayDeque(listOf(rootId to rootName))
        while (pending.isNotEmpty()) {
            val (docId, folder) = pending.removeFirst()
            val rows = mutableListOf<Array<String?>>()
            resolver.query(DocumentsContract.buildChildDocumentsUriUsingTree(tree, docId), cols, null, null, null)?.use { c ->
                while (c.moveToNext()) rows += arrayOf(c.getString(0), c.getString(1), c.getString(2), if (c.isNull(3)) null else c.getLong(3).toString())
            }
            for ((id, name, mime, size) in rows.sortedBy { it[1].orEmpty() }) {
                if (id == null || name == null) continue
                if (mime == DocumentsContract.Document.MIME_TYPE_DIR) {
                    pending += id to "$folder/$name"
                    continue
                }
                if (name in CLUTTER) continue
                if (out.size >= FILES_MAX) throw EngineException("That folder has more than 100,000 files. Send it in parts.")
                val uri = DocumentsContract.buildDocumentUriUsingTree(tree, id)
                val item = FileItem("t${seq.incrementAndGet()}", name, size?.toLongOrNull() ?: 0, mime ?: "application/octet-stream", folder)
                folderSources[item.id] = uri to folder
                folderFiles += uri.toString()
                out += item
            }
        }
        Staged(out, if (out.isEmpty()) 1 else 0)
    }

    /**
     * The engine reads each file through the descriptor Android opened for us (`/proc/self/fd/<n>`,
     * see core/src/files.rs `open_to_send`); it stays open until the transfer ends. A few apps share
     * a stream rather than a file, which can't be resumed, so those are copied to the cache first.
     */
    override suspend fun send(deviceId: String, files: List<FileItem>): String = withContext(Dispatchers.IO) {
        val handles = mutableListOf<ParcelFileDescriptor>()
        val copyDir = File(context.cacheDir, "outgoing/${seq.incrementAndGet()}")
        try {
            val id = call { send(deviceId, toSend(files, handles, copyDir)) }
            openFiles[id] = handles
            if (copyDir.exists()) copies[id] = copyDir
            id
        } catch (e: Exception) {
            handles.forEach { runCatching { it.close() } }
            copyDir.deleteRecursively()
            throw if (e is EngineException) e else EngineException(e.message ?: "Couldn't send.")
        }
    }

    /** The engine's view of [files], opening each one (into [handles]) or copying it (into [copyDir]). */
    private fun toSend(files: List<FileItem>, handles: MutableList<ParcelFileDescriptor>, copyDir: File): List<SendFile> = files.map { item ->
        folderSources[item.id]?.let { (uri, folder) -> return@map SendFile(path = uri.toString(), name = item.name, folder = folder) }
        val uri = sources[item.id] ?: throw EngineException("${item.name} can't be found any more. Add it again.")
        val pfd = context.contentResolver.openFileDescriptor(uri, "r")
            ?: throw EngineException("Couldn't open ${item.name}.")
        if (pfd.statSize >= 0) {
            handles += pfd
            SendFile(path = "/proc/self/fd/${pfd.fd}", name = item.name, folder = null)
        } else {
            pfd.close()
            val copy = File(copyDir, "${handles.size}-${item.name.replace('/', '_')}").also { it.parentFile?.mkdirs() }
            context.contentResolver.openInputStream(uri)?.use { input -> copy.outputStream().use { input.copyTo(it) } }
                ?: throw EngineException("Couldn't open ${item.name}.")
            SendFile(path = copy.path, name = item.name, folder = null)
        }
    }

    /** The shared link's open files, kept until it stops. */
    private var linkHandles: Pair<List<ParcelFileDescriptor>, File>? = null

    private fun closeLinkFiles() {
        val (handles, dir) = linkHandles ?: return
        linkHandles = null
        handles.forEach { runCatching { it.close() } }
        dir.deleteRecursively()
    }

    override suspend fun shareLink(files: List<FileItem>, text: String?): LinkShare = withContext(Dispatchers.IO) {
        val handles = mutableListOf<ParcelFileDescriptor>()
        val copyDir = File(context.cacheDir, "outgoing/link-${seq.incrementAndGet()}")
        try {
            val share: LinkShare = ContractJson.decodeFromString(call { shareLink(toSend(files, handles, copyDir), text) })
            closeLinkFiles()
            linkHandles = handles to copyDir
            share
        } catch (e: Exception) {
            handles.forEach { runCatching { it.close() } }
            copyDir.deleteRecursively()
            throw if (e is EngineException) e else EngineException(e.message ?: "Couldn't share.")
        }
    }

    override fun stopLink() = fire { stopLink() }
    override fun answerVisitor(visitorId: String, allow: Boolean) = fire { answerVisitor(visitorId, allow) }

    override suspend fun getUpdateCheck(): Boolean? = call { getUpdateCheck() }
    override suspend fun setUpdateCheck(on: Boolean) = call { setUpdateCheck(on) }

    override suspend fun sendText(deviceId: String, text: String): String = call { sendText(deviceId, text) }

    override suspend fun askTrust(deviceId: String) = call { askTrust(deviceId) }
    override fun cancelTrust(deviceId: String) = fire { cancelTrust(deviceId) }
    override fun answerTrust(deviceId: String, yes: Boolean) = fire { answerTrust(deviceId, yes) }

    override suspend fun accept(requestId: String, trust: Boolean, autoAccept: Boolean): String? =
        call { accept(requestId, trust, autoAccept) }

    override fun decline(requestId: String) = fire { decline(requestId) }
    override fun pause(transferId: String) = fire { pause(transferId) }
    override fun resume(transferId: String) = fire { resume(transferId) }
    override fun cancel(transferId: String) = fire { cancel(transferId) }
    override fun retry(transferId: String) = fire { retry(transferId) }

    override suspend fun createCode(): PairingCode = ContractJson.decodeFromString(call { createCode() })
    override fun cancelCode() = fire { cancelCode() }
    override suspend fun redeemCode(words: List<String>): RedeemResult = ContractJson.decodeFromString(call { redeemCode(words) })

    override fun setTrusted(deviceId: String, trusted: Boolean) = fire { setTrusted(deviceId, trusted) }
    override fun setAutoAccept(deviceId: String, on: Boolean) = fire { setAutoAccept(deviceId, on) }
    override suspend fun renameSelf(name: String) = call { renameSelf(name) }
    override suspend fun getSelf(): SelfInfo = ContractJson.decodeFromString(call { getSelf() })

    override suspend fun getConnection(): Connection = ContractJson.decodeFromString(call { getConnection() })
    override suspend fun setConnection(c: Connection) = call { setConnection(ContractJson.encodeToString(Connection.serializer(), c)) }
    override suspend fun checkServer(c: Connection.Own): ServerCheck =
        ContractJson.decodeFromString(call { checkServer(ContractJson.encodeToString(Connection.Own.serializer(), c)) })

    override suspend fun savedFiles(transferId: String): List<String> = call { savedFiles(transferId) }

    override suspend fun getLanDiscovery(): Boolean = call { getLanDiscovery() }
    override suspend fun setLanDiscovery(on: Boolean) = call { setLanDiscovery(on) }
    override suspend fun getWhoCanSend(): WhoCanSend = WhoCanSend.fromWire(call { getWhoCanSend() })
    override suspend fun setWhoCanSend(who: WhoCanSend) = call { setWhoCanSend(who.wire) }
    override fun clearHistory() = fire { clearHistory() }
    override suspend fun diagnostics(): String = withContext(Dispatchers.IO) {
        native.diagnostics("android ${BuildConfig.VERSION_NAME} (Android ${android.os.Build.VERSION.RELEASE})")
    }

    override fun setWakeToken(token: String?) = fire { setWakeToken(token) }

    companion object {
        /** The engine's own limit on one transfer (core FILES_MAX). */
        const val FILES_MAX = 100_000
        /** Files the system leaves in folders, never worth sending (core files::walk_folder). */
        private val CLUTTER = setOf(".DS_Store", "Thumbs.db", "desktop.ini")

        /** Download/Nhako Beam: where people look for files, and visible in the Files app. */
        fun defaultSaveDir(): String =
            File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS), "Nhako Beam").path
    }
}
