package io.github.kimzam30.nhakobeam.engine

import android.content.Context
import android.net.Uri
import android.os.Environment
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import io.github.kimzam30.nhakobeam.core.EventListener
import io.github.kimzam30.nhakobeam.core.NhakoEngine
import io.github.kimzam30.nhakobeam.core.NhakoException
import io.github.kimzam30.nhakobeam.core.SendFile
import io.github.kimzam30.nhakobeam.model.Connection
import io.github.kimzam30.nhakobeam.model.ContractJson
import io.github.kimzam30.nhakobeam.model.EngineEvent
import io.github.kimzam30.nhakobeam.model.FileItem
import io.github.kimzam30.nhakobeam.model.PairingCode
import io.github.kimzam30.nhakobeam.model.RedeemResult
import io.github.kimzam30.nhakobeam.model.SelfInfo
import io.github.kimzam30.nhakobeam.model.ServerCheck
import io.github.kimzam30.nhakobeam.model.Snapshot
import io.github.kimzam30.nhakobeam.model.TransferStatus
import java.io.File
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.launch
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
        })
    }

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
        _events.tryEmit(event)
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
        val resolver = context.contentResolver
        var name: String? = null
        var size: Long? = null
        resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)?.use { c ->
            if (c.moveToFirst()) {
                name = c.getString(0)
                size = if (c.isNull(1)) null else c.getLong(1)
            }
        }
        if (uri.scheme == "file") {
            val f = File(uri.path ?: return null)
            name = f.name
            size = f.length()
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

    /**
     * The engine reads each file through the descriptor Android opened for us (`/proc/self/fd/<n>`,
     * see core/src/files.rs `open_to_send`); it stays open until the transfer ends. A few apps share
     * a stream rather than a file, which can't be resumed, so those are copied to the cache first.
     */
    override suspend fun send(deviceId: String, files: List<FileItem>): String = withContext(Dispatchers.IO) {
        val handles = mutableListOf<ParcelFileDescriptor>()
        val copyDir = File(context.cacheDir, "outgoing/${seq.incrementAndGet()}")
        try {
            val list = files.map { item ->
                val uri = sources[item.id] ?: throw EngineException("${item.name} can't be found any more. Add it again.")
                val pfd = context.contentResolver.openFileDescriptor(uri, "r")
                    ?: throw EngineException("Couldn't open ${item.name}.")
                if (pfd.statSize >= 0) {
                    handles += pfd
                    SendFile(path = "/proc/self/fd/${pfd.fd}", name = item.name)
                } else {
                    pfd.close()
                    val copy = File(copyDir, "${handles.size}-${item.name.replace('/', '_')}").also { it.parentFile?.mkdirs() }
                    context.contentResolver.openInputStream(uri)?.use { input -> copy.outputStream().use { input.copyTo(it) } }
                        ?: throw EngineException("Couldn't open ${item.name}.")
                    SendFile(path = copy.path, name = item.name)
                }
            }
            val id = call { send(deviceId, list) }
            openFiles[id] = handles
            if (copyDir.exists()) copies[id] = copyDir
            id
        } catch (e: Exception) {
            handles.forEach { runCatching { it.close() } }
            copyDir.deleteRecursively()
            throw if (e is EngineException) e else EngineException(e.message ?: "Couldn't send.")
        }
    }

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
    override fun clearHistory() = fire { clearHistory() }

    override fun setWakeToken(token: String?) = fire { setWakeToken(token) }
    override val wakeAvailable: Boolean by lazy { runCatching { native.wakeAvailable() }.getOrDefault(false) }

    companion object {
        /** Download/Nhako Beam: where people look for files, and visible in the Files app. */
        fun defaultSaveDir(): String =
            File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS), "Nhako Beam").path
    }
}
