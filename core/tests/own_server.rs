//! Your own server: a real iroh relay (with a token) and a real address lookup server run inside
//! the test, the same software the Docker image in `relay/` runs. The engines may only use the
//! relay (no direct paths, no mDNS), so everything below has to go through both servers.

use std::{net::Ipv4Addr, path::PathBuf, sync::Arc, time::Duration};

use iroh_relay::server::{Access, AccessControl, ClientRequest, RelayConfig, Server, ServerConfig};
use nhako_core::{
    CheckResult, Connection, Engine, EngineConfig, EngineError, EngineEvent, Network, OwnNetwork, OwnServer, RedeemFailure, RedeemResult, Route, ServerProblem,
    SettingError, TransferStatus, check_server, codekeys::KdfCost, device_id,
};
use tokio::sync::broadcast;

const TOKEN: &str = "test-token-7f3a";
const WAIT: Duration = Duration::from_secs(40);

/// Lets in only clients that send the token, like `access.shared_token` in `relay/config/relay.toml`.
#[derive(Debug)]
struct TokenAccess;

impl AccessControl for TokenAccess {
    async fn on_connect(&self, request: &ClientRequest) -> Access {
        if request.auth_token().as_deref() == Some(TOKEN) {
            Access::Allow
        } else {
            Access::Deny { reason: None }
        }
    }
}

struct Servers {
    relay: Server,
    lookup: iroh_dns_server::Server,
    data: PathBuf,
}

impl Servers {
    async fn start() -> Self {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .try_init();
        let mut relay = RelayConfig::new((Ipv4Addr::LOCALHOST, 0));
        relay.access = Arc::new(TokenAccess);
        let mut cfg = ServerConfig::default();
        cfg.relay = Some(relay);
        let relay = Server::spawn(cfg).await.unwrap();

        let data = std::env::temp_dir().join(format!("nhako-lookup-{}", rand::random::<u32>()));
        let cfg: iroh_dns_server::config::Config = serde_json::from_value(serde_json::json!({
            "pkarr_put_rate_limit": "disabled",
            "http": { "port": 0, "bind_addr": "127.0.0.1" },
            "dns": {
                "port": 0,
                "bind_addr": "127.0.0.1",
                "default_soa": "lookup.nhakobeam.invalid hostmaster.nhakobeam.invalid 0 10800 3600 604800 3600",
                "default_ttl": 30,
                "origins": ["nhakobeam.invalid.", "."]
            },
            "metrics": { "disabled": true },
            "data_dir": data,
        }))
        .unwrap();
        let lookup = iroh_dns_server::Server::bind(cfg).await.unwrap();
        Self { relay, lookup, data }
    }

    fn own(&self, token: Option<&str>) -> OwnServer {
        OwnServer {
            relay: format!("http://{}", self.relay.http_addr().unwrap()),
            token: token.map(Into::into),
            // A bare server address: the engine adds /pkarr, as it does for relay/.
            lookup: Some(format!("http://{}", self.lookup.http_addr().unwrap())),
        }
    }

    async fn stop(self) {
        let _ = self.relay.shutdown().await;
        let _ = self.lookup.shutdown().await;
        let _ = std::fs::remove_dir_all(&self.data);
    }
}

struct Node {
    engine: Engine,
    events: broadcast::Receiver<EngineEvent>,
    dir: PathBuf,
}

impl Node {
    /// An engine that takes its network from the saved setting, like the app does.
    async fn new(name: &str, conn: Connection) -> Self {
        let dir = std::env::temp_dir().join(format!("nhako-own-{name}-{}", rand::random::<u32>()));
        let engine = Self::start(&dir, Some(conn)).await.unwrap();
        engine.rename_self(name);
        let events = engine.subscribe();
        Self { engine, events, dir }
    }

    async fn start(dir: &std::path::Path, save: Option<Connection>) -> Result<Engine, EngineError> {
        let mut cfg = EngineConfig::new(dir.join("config"));
        // Never GitHub from a test.
        cfg.update_url = None;
        cfg.save_dir = Some(dir.join("save"));
        cfg.relay_only = true;
        cfg.mdns = false;
        cfg.beacon = None;
        cfg.kdf = KdfCost::TEST;
        cfg.code_lookup_for = Duration::from_secs(15);
        if let Some(c) = save {
            // Save the setting with a throwaway local engine first, then start on it.
            let mut first = EngineConfig::new(dir.join("config"));
            // Never GitHub from a test.
            first.update_url = None;
            first.network = Some(Network::LocalOnly);
            first.mdns = false;
            first.beacon = None;
            let e = Engine::start(first).await?;
            e.set_connection(c)?;
            e.stop().await;
        }
        Engine::start(cfg).await
    }

    async fn wait<T>(&mut self, what: &str, mut f: impl FnMut(&EngineEvent) -> Option<T>) -> T {
        let fut = async {
            loop {
                match self.events.recv().await {
                    Ok(e) => {
                        if let Some(t) = f(&e) {
                            return t;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(e) => panic!("events closed: {e}"),
                }
            }
        };
        tokio::time::timeout(WAIT, fut).await.unwrap_or_else(|_| panic!("timed out waiting for {what}"))
    }

    async fn stop(self) {
        self.engine.stop().await;
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn check_tells_token_problems_apart() {
    let s = Servers::start().await;

    let ok = check_server(&s.own(Some(TOKEN))).await;
    assert!(matches!(ok.relay, CheckResult::Ok { .. }), "{ok:?}");
    assert!(matches!(ok.lookup, Some(CheckResult::Ok { .. })), "{ok:?}");

    assert_eq!(check_server(&s.own(Some("wrong"))).await.relay, CheckResult::TokenRejected);
    assert_eq!(check_server(&s.own(None)).await.relay, CheckResult::TokenRejected);

    // Nothing listens on port 9 (discard) on localhost.
    let mut gone = s.own(Some(TOKEN));
    gone.relay = "http://127.0.0.1:9".into();
    gone.lookup = Some("http://127.0.0.1:9".into());
    let r = check_server(&gone).await;
    assert!(matches!(r.relay, CheckResult::Unreachable { .. }), "{r:?}");
    assert!(matches!(r.lookup, Some(CheckResult::Unreachable { .. })), "{r:?}");

    let mut typo = s.own(Some("has space"));
    typo.lookup = Some("lookup.example.com".into());
    let r = check_server(&typo).await;
    assert!(matches!(r.relay, CheckResult::Invalid { .. }), "{r:?}");
    assert!(matches!(r.lookup, Some(CheckResult::Invalid { .. })), "{r:?}");

    s.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn saved_setting_is_checked_and_used() {
    let s = Servers::start().await;
    let dir = std::env::temp_dir().join(format!("nhako-own-setting-{}", rand::random::<u32>()));
    let engine = Node::start(&dir, None).await.unwrap();
    assert_eq!(engine.connection(), Connection::Public);
    assert_eq!(engine.network(), &Network::N0);

    let mut bad = s.own(Some(TOKEN));
    bad.relay = "192.168.1.20:3340".into();
    assert!(matches!(
        engine.set_connection(Connection::Own(bad)),
        Err(EngineError::Setting(SettingError::Relay(_)))
    ));
    assert_eq!(engine.connection(), Connection::Public, "a bad setting isn't saved");

    let good = Connection::Own(s.own(Some(TOKEN)));
    engine.set_connection(good.clone()).unwrap();
    engine.stop().await;

    let engine = Node::start(&dir, None).await.unwrap();
    assert_eq!(engine.connection(), good);
    let Network::Own(OwnNetwork { lookup: Some(url), token, .. }) = engine.network() else {
        panic!("expected own network, got {:?}", engine.network())
    };
    assert!(url.as_str().ends_with("/pkarr"));
    assert_eq!(token.as_deref(), Some(TOKEN));
    engine.stop().await;
    let _ = std::fs::remove_dir_all(&dir);
    s.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn code_and_transfer_through_own_relay_only() {
    let s = Servers::start().await;
    let conn = Connection::Own(s.own(Some(TOKEN)));
    let mut a = Node::new("Laptop", conn.clone()).await;
    let mut b = Node::new("Phone", conn).await;

    // The code endpoint is found through our lookup server and reached through our relay.
    let code = a.engine.create_code();
    a.wait("code shown", |e| matches!(e, EngineEvent::Code { code: Some(_) }).then_some(())).await;
    let r = b.engine.redeem_code(&code.words).await;
    let RedeemResult::Ok(dev) = r else { panic!("redeem failed: {r:?}") };
    assert_eq!(dev.id, device_id(&a.engine.id()));
    a.wait("paired", |e| matches!(e, EngineEvent::Paired { .. }).then_some(())).await;

    let out = a.dir.join("out");
    std::fs::create_dir_all(&out).unwrap();
    let f = out.join("through-relay.bin");
    let data: Vec<u8> = (0..3 * 1024 * 1024).map(|i: u32| (i % 251) as u8).collect();
    std::fs::write(&f, &data).unwrap();
    a.engine.send(&device_id(&b.engine.id()), std::slice::from_ref(&f)).unwrap();
    let req = b
        .wait("incoming", |e| match e {
            EngineEvent::Incoming { request } => Some(request.id.clone()),
            _ => None,
        })
        .await;
    b.engine.accept(&req, false, false);
    let t = b
        .wait("done", |e| match e {
            EngineEvent::Transfer { transfer } if transfer.status == TransferStatus::Done => Some(transfer.clone()),
            _ => None,
        })
        .await;
    assert_eq!(t.route, Route::Relayed);
    assert_eq!(std::fs::read(b.dir.join("save").join("through-relay.bin")).unwrap(), data);

    a.stop().await;
    b.stop().await;
    s.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn wrong_token_cant_reach_anyone() {
    let s = Servers::start().await;
    let mut a = Node::new("Has token", Connection::Own(s.own(Some(TOKEN)))).await;
    let b = Node::new("Wrong token", Connection::Own(s.own(Some("wrong")))).await;

    let code = a.engine.create_code();
    a.wait("code shown", |e| matches!(e, EngineEvent::Code { code: Some(_) }).then_some(())).await;
    // B can find the record (lookup is open) but the relay turns it away, and B is told why.
    assert_eq!(b.engine.redeem_code(&code.words).await, RedeemResult::Err(RedeemFailure::TokenRejected));
    assert_eq!(b.engine.server_problem(), Some(ServerProblem::TokenRejected));
    assert_eq!(b.engine.snapshot().server_problem, Some(ServerProblem::TokenRejected));
    assert_eq!(a.engine.check_own_server().await, None);

    a.stop().await;
    b.stop().await;
    s.stop().await;
}
