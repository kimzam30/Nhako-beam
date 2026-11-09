package io.github.kimzam30.nhakobeam.state

import io.github.kimzam30.nhakobeam.model.Transfer
import io.github.kimzam30.nhakobeam.model.TransferStatus

// A send to several devices shows as one card on Home. The desktop has the same rules in
// src/lib/state/groups.ts; change both together.

/** Up to this many devices in one send. */
const val MAX_DEVICES = 8

/** One card on Home: a transfer on its own, or a send to several devices. */
sealed interface ActivityItem {
    val key: String
    data class One(val transfer: Transfer) : ActivityItem { override val key get() = transfer.id }
    data class Group(val id: String, val transfers: List<Transfer>) : ActivityItem { override val key get() = id }
}

/** The transfers as Home shows them, in order. A group sits where its first transfer would. A
 *  group down to one shown transfer (the others dismissed) is just that transfer again. */
fun activityItems(transfers: List<Transfer>, groups: Map<String, List<String>>): List<ActivityItem> {
    val groupOf = HashMap<String, String>()
    for ((g, ids) in groups) for (id in ids) groupOf[id] = g
    val seen = HashSet<String>()
    val items = mutableListOf<ActivityItem>()
    for (tr in transfers) {
        val g = groupOf[tr.id]
        if (g == null) {
            items += ActivityItem.One(tr)
            continue
        }
        if (!seen.add(g)) continue
        val members = transfers.filter { groupOf[it.id] == g }
        items += if (members.size > 1) ActivityItem.Group(g, members) else ActivityItem.One(tr)
    }
    return items
}

/** The group's progress: all bytes over all devices, so a fast device doesn't hide a slow one. */
fun groupProgress(transfers: List<Transfer>): Float {
    val total = transfers.sumOf { it.totalBytes }
    if (total <= 0) return if (transfers.all { it.status == TransferStatus.Done }) 1f else 0f
    return transfers.sumOf { if (it.status == TransferStatus.Done) it.totalBytes else it.doneBytes }.toFloat() / total
}
