//! Endpoint setup, route detection and the `Ping` exchange.

use std::{net::IpAddr, time::Duration};

use iroh::{
    Endpoint, EndpointId, RelayConfig, RelayMap, RelayMode, SecretKey, TransportAddr,
    address_lookup::{DnsAddressLookup, MemoryLookup, PkarrPublisher, PkarrResolver},
    endpoint::{Connection as Conn, IdleTimeout, QuicTransportConfig, presets},
};
use iroh_mdns_address_lookup::MdnsAddressLookup;
use iroh_relay::RelayQuicConfig;

use crate::{
    server::{self, OwnNetwork, SettingError},
    types::{Connection, Route},
    wire::{self, ClientHello, DeviceInfo, HelloExt, Intent, ServerHello},
};

/// mDNS service name, so we only see other Nhako Beam devices.
pub const MDNS_SERVICE: &str = "nhakobeam";

pub const KEEP_ALIVE: Duration = Duration::from_secs(2);
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(12);

/// Which relay and address lookup servers to use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Network {
    /// n0's free public relays and address lookup.
    N0,
    /// Your own relay, and your own address lookup if set (n0's otherwise).
    Own(OwnNetwork),
    /// Same network only: no relay, no internet lookup. mDNS still works.
    LocalOnly,
}

impl Network {
    /// The network for a saved connection setting.
    pub fn from_connection(c: &Connection) -> Result<Self, SettingError> {
        Ok(match c {
            Connection::Public => Self::N0,
            Connection::Own(s) => Self::Own(OwnNetwork::parse(s)?),
        })
    }
}

/// What a bound endpoint looks like to the rest of the engine.
pub struct Bound {
    pub endpoint: Endpoint,
    pub mdns: Option<MdnsAddressLookup>,
    /// Set when mDNS couldn't start (for example multicast is blocked).
    pub mdns_error: Option<String>,
}

pub struct EndpointOpts<'a> {
    pub secret: SecretKey,
    pub alpns: Vec<Vec<u8>>,
    pub network: &'a Network,
    /// Never use direct paths, only the relay. For testing a relay.
    pub relay_only: bool,
    pub mdns: bool,
    /// Tests: resolve addresses from a shared in-memory table instead of the network.
    pub memory: Option<&'a MemoryLookup>,
}

pub async fn bind(opts: EndpointOpts<'_>) -> Result<Bound, iroh::endpoint::BindError> {
    let transport = QuicTransportConfig::builder()
        .keep_alive_interval(KEEP_ALIVE)
        .max_idle_timeout(Some(IdleTimeout::try_from(IDLE_TIMEOUT).expect("idle timeout fits")))
        // Bigger windows than the defaults so one stream can fill a fast link with some latency.
        .stream_receive_window(iroh::endpoint::VarInt::from_u32(8 * 1024 * 1024))
        .send_window(16 * 1024 * 1024)
        .build();

    let mut b = match opts.network {
        Network::N0 => Endpoint::builder(presets::N0).proxy_from_env(),
        Network::Own(own) => {
            // QUIC address discovery needs TLS, so only an https relay offers it.
            let quic = (own.relay.scheme() == "https").then(RelayQuicConfig::default);
            let mut relay = RelayConfig::new(own.relay.clone(), quic);
            if let Some(t) = &own.token {
                relay = relay.with_auth_token(t.clone());
            }
            let mut b = Endpoint::builder(presets::Minimal).relay_mode(RelayMode::Custom(RelayMap::from(relay)));
            b = match &own.lookup {
                Some(url) => b
                    .address_lookup(PkarrPublisher::builder(url.clone()))
                    .address_lookup(PkarrResolver::builder(url.clone())),
                None => b
                    .address_lookup(PkarrPublisher::n0_dns())
                    .address_lookup(PkarrResolver::n0_dns())
                    .address_lookup(DnsAddressLookup::n0_dns()),
            };
            if let Some(p) = own.relay.host_str().and_then(server::proxy_for) {
                b = b.proxy_url(p);
            }
            b
        }
        Network::LocalOnly => Endpoint::builder(presets::Minimal).relay_mode(RelayMode::Disabled),
    };
    if opts.relay_only {
        b = b.clear_ip_transports();
    }
    if opts.memory.is_some() {
        b = b.clear_address_lookup().relay_mode(RelayMode::Disabled);
    }
    if let Some(mem) = opts.memory {
        b = b.address_lookup(mem.clone());
    }
    let endpoint = b.secret_key(opts.secret).alpns(opts.alpns).transport_config(transport).bind().await?;

    if let Some(mem) = opts.memory {
        // Bound sockets are known right away; publish them for the other test engines.
        let addr = iroh::EndpointAddr::from_parts(
            endpoint.id(),
            endpoint.bound_sockets().into_iter().map(|a| TransportAddr::Ip(localhost_if_unspecified(a))),
        );
        mem.add_endpoint_info(addr);
    }

    let (mdns, mdns_error) = if opts.mdns {
        match MdnsAddressLookup::builder().service_name(MDNS_SERVICE).build(endpoint.id()) {
            Ok(m) => match endpoint.address_lookup() {
                Ok(services) => {
                    services.add(m.clone());
                    (Some(m), None)
                }
                Err(e) => (None, Some(e.to_string())),
            },
            Err(e) => (None, Some(e.to_string())),
        }
    } else {
        (None, None)
    };
    Ok(Bound { endpoint, mdns, mdns_error })
}

fn localhost_if_unspecified(mut a: std::net::SocketAddr) -> std::net::SocketAddr {
    if a.ip().is_unspecified() {
        a.set_ip(match a.ip() {
            IpAddr::V4(_) => IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            IpAddr::V6(_) => IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
        });
    }
    a
}

/// Private, link-local, loopback and unique-local addresses count as "same network".
pub fn is_lan(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local() || v4.is_loopback() || (v4.octets()[0] == 100 && (v4.octets()[1] & 0xc0) == 64),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_lan(IpAddr::V4(v4));
            }
            v6.is_loopback() || (v6.segments()[0] & 0xfe00) == 0xfc00 || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

pub fn route_for(addr: &TransportAddr) -> Route {
    match addr {
        TransportAddr::Ip(a) if is_lan(a.ip()) => Route::Nearby,
        TransportAddr::Ip(_) => Route::Direct,
        _ => Route::Relayed,
    }
}

/// The route the connection is using right now.
pub fn route_of(conn: &Conn) -> Route {
    let paths = conn.paths();
    let selected = paths.iter().find(|p| p.is_selected());
    selected.map(|p| route_for(p.remote_addr())).unwrap_or(Route::Relayed)
}

/// True if the connection runs over a same-network address.
pub fn is_lan_conn(conn: &Conn) -> bool {
    route_of(conn) == Route::Nearby
}

#[derive(Debug, thiserror::Error)]
pub enum PingError {
    #[error("couldn't connect: {0}")]
    Connect(String),
    #[error("timed out")]
    Timeout,
    #[error(transparent)]
    Wire(#[from] wire::WireError),
    #[error("refused")]
    Refused,
}

/// Connects, swaps `DeviceInfo` (and the hello extension), and reports the route.
pub async fn ping(
    endpoint: &Endpoint,
    peer: EndpointId,
    me: DeviceInfo,
    ext: &HelloExt,
    timeout: Duration,
) -> Result<(DeviceInfo, HelloExt, Route), PingError> {
    tokio::time::timeout(timeout, async {
        let conn = endpoint.connect(peer, wire::ALPN).await.map_err(|e| PingError::Connect(e.to_string()))?;
        let (mut send, mut recv) = conn.open_bi().await.map_err(|e| PingError::Connect(e.to_string()))?;
        wire::write_hello(
            &mut send,
            &ClientHello {
                version: wire::VERSION,
                info: me,
                intent: Intent::Ping,
            },
            ext,
        )
        .await?;
        let (hello, their_ext): (ServerHello, HelloExt) = match wire::read_hello(&mut recv).await {
            Ok(h) => h,
            Err(wire::WireError::Closed) => return Err(PingError::Refused),
            Err(e) => return Err(e.into()),
        };
        let route = route_of(&conn);
        conn.close(wire::close::DONE, b"");
        Ok((hello.info, their_ext, route))
    })
    .await
    .map_err(|_| PingError::Timeout)?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lan_detection() {
        for ip in [
            "192.168.1.5",
            "10.0.0.2",
            "172.16.4.4",
            "169.254.1.1",
            "127.0.0.1",
            "100.64.0.1",
            "fe80::1",
            "fd00::1",
            "::1",
            "::ffff:192.168.0.1",
        ] {
            assert!(is_lan(ip.parse().unwrap()), "{ip} should be same network");
        }
        for ip in ["8.8.8.8", "2001:4860::8888", "100.128.0.1", "::ffff:1.1.1.1"] {
            assert!(!is_lan(ip.parse().unwrap()), "{ip} should be internet");
        }
    }
}
