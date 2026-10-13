//! The beacon: Nhako Beam's own "I'm here" message on the local network.
//!
//! mDNS shares UDP port 5353 with the system's own mDNS service. On Windows the DNS Client
//! service keeps that port to itself for IPv4, so mDNS there only runs over IPv6, which phones
//! and laptops on Wi-Fi rarely hear (found on a real Windows PC after Phase 9). The beacon runs
//! next to mDNS on a port only Nhako Beam uses, on every network adapter, by multicast and by
//! broadcast (some routers drop multicast between Wi-Fi and Ethernet).
//!
//! A beacon only says "device X takes connections on port P of the address this came from".
//! Nothing in it is trusted: the engine then connects to X, and iroh checks X's key.
//! The format is in `docs/protocol/nhako-1.md` ("The beacon").

use std::{
    collections::BTreeSet,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{Arc, Mutex},
    time::Instant,
};

use iroh::EndpointId;
use socket2::{Domain, Protocol, SockRef, Socket, Type};
use tokio::{net::UdpSocket, sync::Notify};
use tracing::{debug, info, warn};

/// The beacon's UDP port.
pub const PORT: u16 = 47820;
/// The beacon's multicast group (organization-local scope, so routers keep it inside the network).
pub const GROUP: Ipv4Addr = Ipv4Addr::new(239, 255, 78, 66);

const MAGIC: &[u8; 4] = b"NHKB";
const VERSION: u8 = 1;
/// Magic, version, flags, device id, port. Later versions may add fields after these.
const LEN: usize = 4 + 1 + 1 + 32 + 2;
/// Anything longer isn't ours.
const MAX_LEN: usize = 512;
const FLAG_ASK: u8 = 1;

/// One beacon message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Packet {
    pub id: EndpointId,
    /// The UDP port the device takes connections on (its iroh endpoint).
    pub port: u16,
    /// "Please announce yourselves": sent when a device starts, rescans or changes networks.
    pub ask: bool,
}

impl Packet {
    pub fn encode(&self) -> [u8; LEN] {
        let mut b = [0u8; LEN];
        b[..4].copy_from_slice(MAGIC);
        b[4] = VERSION;
        b[5] = if self.ask { FLAG_ASK } else { 0 };
        b[6..38].copy_from_slice(self.id.as_bytes());
        b[38..40].copy_from_slice(&self.port.to_be_bytes());
        b
    }

    pub fn decode(b: &[u8]) -> Option<Self> {
        if b.len() < LEN || b.len() > MAX_LEN || &b[..4] != MAGIC || b[4] < VERSION {
            return None;
        }
        let id = EndpointId::from_bytes(b[6..38].try_into().ok()?).ok()?;
        let port = u16::from_be_bytes([b[38], b[39]]);
        if port == 0 {
            return None;
        }
        Some(Self {
            id,
            port,
            ask: b[5] & FLAG_ASK != 0,
        })
    }
}

/// Where the beacon listens and sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub port: u16,
    /// `None`: the multicast group and the broadcast address of every network adapter.
    /// Tests list loopback addresses here instead.
    pub send_to: Option<Vec<SocketAddr>>,
}

impl Default for Config {
    fn default() -> Self {
        Self { port: PORT, send_to: None }
    }
}

/// A network adapter the beacon sends on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Adapter {
    pub name: String,
    pub addr: Ipv4Addr,
    pub prefix: u8,
}

impl Adapter {
    /// The subnet's broadcast address, if it has one.
    fn broadcast(&self) -> Option<Ipv4Addr> {
        if self.prefix >= 31 {
            return None;
        }
        let mask = u32::MAX << (32 - u32::from(self.prefix));
        Some(Ipv4Addr::from(u32::from(self.addr) | !mask))
    }
}

/// What the beacon has done so far, for Copy diagnostics.
#[derive(Debug, Clone, Default)]
pub struct Status {
    pub port: u16,
    pub adapters: Vec<Adapter>,
    pub sent: u64,
    pub send_failures: u64,
    pub last_send_error: Option<String>,
    pub received: u64,
    pub last_heard: Option<Instant>,
}

pub struct Beacon {
    socket: UdpSocket,
    cfg: Config,
    me: Packet,
    adapters: Mutex<BTreeSet<Adapter>>,
    status: Mutex<Status>,
    ask: Notify,
}

impl Beacon {
    /// Opens the beacon's socket for device `id`, whose endpoint takes connections on `port`.
    /// Must be called inside the tokio runtime.
    pub fn open(cfg: Config, id: EndpointId, port: u16) -> io::Result<Arc<Self>> {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        // More than one copy of Nhako Beam on a computer (the app and the `nhako` tool) can listen.
        socket.set_reuse_address(true)?;
        #[cfg(all(unix, not(any(target_os = "solaris", target_os = "illumos", target_os = "cygwin"))))]
        socket.set_reuse_port(true)?;
        socket.set_broadcast(true)?;
        socket.set_multicast_loop_v4(true)?;
        socket.set_multicast_ttl_v4(1)?;
        socket.set_nonblocking(true)?;
        socket.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, cfg.port).into())?;
        let socket = UdpSocket::from_std(socket.into())?;
        let beacon = Arc::new(Self {
            socket,
            me: Packet { id, port, ask: false },
            status: Mutex::new(Status {
                port: cfg.port,
                ..Default::default()
            }),
            cfg,
            adapters: Mutex::new(BTreeSet::new()),
            ask: Notify::new(),
        });
        beacon.refresh_adapters();
        Ok(beacon)
    }

    /// Asks the other devices to announce themselves (a few times, in case one is lost).
    pub fn ask(&self) {
        self.ask.notify_one();
    }

    /// Resolves when someone called [`Beacon::ask`].
    pub async fn asked(&self) {
        self.ask.notified().await
    }

    pub fn status(&self) -> Status {
        self.status.lock().expect("beacon status lock").clone()
    }

    /// The next beacon from another device: its message and where it came from.
    /// Skips anything that isn't a beacon, and this device's own.
    pub async fn recv(&self) -> io::Result<(Packet, SocketAddr)> {
        let mut buf = [0u8; MAX_LEN + 1];
        loop {
            let (n, from) = match self.socket.recv_from(&mut buf).await {
                Ok(r) => r,
                // Windows reports an earlier send that bounced (ICMP port unreachable) on the next receive.
                Err(e) if e.kind() == io::ErrorKind::ConnectionReset => continue,
                Err(e) => return Err(e),
            };
            let Some(p) = Packet::decode(&buf[..n]) else { continue };
            if p.id == self.me.id {
                continue;
            }
            let mut s = self.status.lock().expect("beacon status lock");
            s.received += 1;
            s.last_heard = Some(Instant::now());
            return Ok((p, from));
        }
    }

    /// Looks at the network adapters again. Returns true if they changed.
    pub fn refresh_adapters(&self) -> bool {
        if self.cfg.send_to.is_some() {
            return false;
        }
        let now = lan_adapters();
        let mut current = self.adapters.lock().expect("adapters lock");
        if *current == now {
            return false;
        }
        let sock = SockRef::from(&self.socket);
        for gone in current.difference(&now) {
            let _ = sock.leave_multicast_v4(&GROUP, &gone.addr);
        }
        for new in now.difference(&current) {
            if let Err(e) = sock.join_multicast_v4(&GROUP, &new.addr) {
                debug!(adapter = %new.name, "beacon couldn't join the group: {e}");
            }
        }
        info!(adapters = ?now.iter().map(|a| format!("{} {}/{}", a.name, a.addr, a.prefix)).collect::<Vec<_>>(), "beacon adapters");
        self.status.lock().expect("beacon status lock").adapters = now.iter().cloned().collect();
        *current = now;
        true
    }

    /// Tells every device on the network this one is here.
    pub async fn announce(&self, ask: bool) {
        let packet = Packet { ask, ..self.me }.encode();
        if let Some(list) = &self.cfg.send_to {
            for to in list {
                self.send(&packet, *to).await;
            }
            return;
        }
        let adapters: Vec<Adapter> = self.adapters.lock().expect("adapters lock").iter().cloned().collect();
        let group = SocketAddr::from((GROUP, self.cfg.port));
        if adapters.is_empty() {
            // Nothing listed (some phones hide their adapters): the system's default route.
            self.send(&packet, group).await;
            self.send(&packet, SocketAddr::from((Ipv4Addr::BROADCAST, self.cfg.port))).await;
            return;
        }
        let sock = SockRef::from(&self.socket);
        for a in &adapters {
            if sock.set_multicast_if_v4(&a.addr).is_ok() {
                self.send(&packet, group).await;
            }
            if let Some(b) = a.broadcast() {
                self.send(&packet, SocketAddr::from((b, self.cfg.port))).await;
            }
        }
    }

    /// Answers one device directly.
    pub async fn reply(&self, to: SocketAddr) {
        self.send(&self.me.encode(), to).await;
    }

    async fn send(&self, packet: &[u8], to: SocketAddr) {
        let r = self.socket.send_to(packet, to).await;
        let mut s = self.status.lock().expect("beacon status lock");
        match r {
            Ok(_) => s.sent += 1,
            Err(e) => {
                if s.last_send_error.is_none() {
                    warn!(%to, "beacon couldn't send: {e}");
                }
                s.send_failures += 1;
                s.last_send_error = Some(format!("{to}: {e}"));
            }
        }
    }
}

/// The IPv4 adapters worth announcing on: up, not loopback, not a point-to-point tunnel (VPNs),
/// with a same-network address.
pub fn lan_adapters() -> BTreeSet<Adapter> {
    let mut out = BTreeSet::new();
    for i in netdev::get_interfaces() {
        if !i.is_up() || i.is_loopback() || i.is_point_to_point() {
            continue;
        }
        let name = i.friendly_name.clone().unwrap_or_else(|| i.name.clone());
        for net in &i.ipv4 {
            let addr = net.addr();
            if addr.is_loopback() || !crate::net::is_lan(IpAddr::V4(addr)) {
                continue;
            }
            out.insert(Adapter {
                name: name.clone(),
                addr,
                prefix: net.prefix_len(),
            });
        }
    }
    out
}

/// Whether mDNS can use IPv4 here: tries to share UDP port 5353 the way the mDNS library does.
/// On Windows the DNS Client service usually holds it alone, and the library then quietly runs
/// on IPv6 only.
pub fn probe_mdns_v4() -> Result<(), String> {
    let s = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).map_err(|e| e.to_string())?;
    s.set_reuse_address(true).map_err(|e| e.to_string())?;
    #[cfg(all(unix, not(any(target_os = "solaris", target_os = "illumos", target_os = "cygwin"))))]
    s.set_reuse_port(true).map_err(|e| e.to_string())?;
    s.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 5353).into()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> EndpointId {
        iroh::SecretKey::from_bytes(&[n; 32]).public()
    }

    #[test]
    fn round_trip() {
        for ask in [false, true] {
            let p = Packet { id: id(1), port: 51234, ask };
            assert_eq!(Packet::decode(&p.encode()), Some(p));
        }
    }

    #[test]
    fn ignores_what_isnt_a_beacon() {
        let good = Packet {
            id: id(2),
            port: 4000,
            ask: false,
        }
        .encode();
        assert!(Packet::decode(&good[..LEN - 1]).is_none(), "too short");
        let mut bad = good;
        bad[0] = b'X';
        assert!(Packet::decode(&bad).is_none(), "wrong magic");
        let mut zero = good;
        zero[38..40].copy_from_slice(&[0, 0]);
        assert!(Packet::decode(&zero).is_none(), "no port");
        assert!(Packet::decode(&[0u8; 600]).is_none(), "too long");
        let mut newer = good.to_vec();
        newer[4] = 2;
        newer.extend_from_slice(b"later fields");
        assert_eq!(Packet::decode(&newer).map(|p| p.port), Some(4000), "a newer version is still read");
    }

    #[test]
    fn broadcast_address() {
        let a = |addr: [u8; 4], prefix| Adapter {
            name: "x".into(),
            addr: addr.into(),
            prefix,
        };
        assert_eq!(a([192, 168, 1, 175], 24).broadcast(), Some(Ipv4Addr::new(192, 168, 1, 255)));
        assert_eq!(a([10, 1, 2, 3], 16).broadcast(), Some(Ipv4Addr::new(10, 1, 255, 255)));
        assert_eq!(a([10, 1, 2, 3], 32).broadcast(), None);
    }
}
