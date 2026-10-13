//! Key derivation for four-word codes. See docs/protocol/nhako-1.md, section 7.

use iroh::{EndpointId, SecretKey};

use crate::words::CODE_WORDS;

/// Codes live 10 minutes, and the Argon2 salt changes with the same period.
pub const WINDOW_SECS: u64 = 600;

const RENDEZVOUS_CTX: &str = "nhakobeam 2026-10 pair rendezvous";
const CONFIRM_CTX: &str = "nhakobeam 2026-10 pair confirm";
const CHECK_CTX: &str = "nhakobeam 2026-10 pair check";

/// What both sides derive from the words and the time window.
#[derive(Clone)]
pub struct CodeKeys {
    pub window: u64,
    pub rendezvous: SecretKey,
    confirm: [u8; 32],
}

impl std::fmt::Debug for CodeKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeKeys")
            .field("window", &self.window)
            .field("rendezvous", &self.rendezvous.public().fmt_short().to_string())
            .finish()
    }
}

/// Argon2id parameters. Tests use cheap ones so the suite stays fast.
#[derive(Debug, Clone, Copy)]
pub struct KdfCost {
    pub memory_kib: u32,
    pub iterations: u32,
}

impl KdfCost {
    pub const REAL: Self = Self {
        memory_kib: 64 * 1024,
        iterations: 3,
    };
    pub const TEST: Self = Self {
        memory_kib: 256,
        iterations: 1,
    };
}

impl CodeKeys {
    /// Slow on purpose (64 MB and about 0.13 s on a desktop CPU with `KdfCost::REAL`). Run it off the async runtime.
    pub fn derive(words: &[String; CODE_WORDS], window: u64, cost: KdfCost) -> Self {
        let password = words.iter().map(|w| w.trim().to_ascii_lowercase()).collect::<Vec<_>>().join(" ");
        // v2: four words. A v1 (three word) code never derives the same keys.
        let salt = format!("nhakobeam-code-v2:{window}");
        let params = argon2::Params::new(cost.memory_kib, cost.iterations, 1, Some(32)).expect("valid argon2 params");
        let argon = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
        let mut k = [0u8; 32];
        argon
            .hash_password_into(password.as_bytes(), salt.as_bytes(), &mut k)
            .expect("argon2 output length is valid");
        Self {
            window,
            rendezvous: SecretKey::from_bytes(&blake3::derive_key(RENDEZVOUS_CTX, &k)),
            confirm: blake3::derive_key(CONFIRM_CTX, &k),
        }
    }

    pub fn rendezvous_id(&self) -> EndpointId {
        self.rendezvous.public()
    }

    pub fn join_mac(&self, joiner: &EndpointId) -> [u8; 32] {
        let mut h = blake3::Hasher::new_keyed(&self.confirm);
        h.update(b"join");
        h.update(self.rendezvous_id().as_bytes());
        h.update(joiner.as_bytes());
        *h.finalize().as_bytes()
    }

    pub fn welcome_mac(&self, joiner: &EndpointId, host: &EndpointId) -> [u8; 32] {
        let mut h = blake3::Hasher::new_keyed(&self.confirm);
        h.update(b"welcome");
        h.update(self.rendezvous_id().as_bytes());
        h.update(joiner.as_bytes());
        h.update(host.as_bytes());
        *h.finalize().as_bytes()
    }

    /// The short value both screens show after pairing, like `5D19 E04A`.
    pub fn check(&self, host: &EndpointId, joiner: &EndpointId) -> String {
        let mut material = Vec::with_capacity(96);
        material.extend_from_slice(&self.confirm);
        material.extend_from_slice(host.as_bytes());
        material.extend_from_slice(joiner.as_bytes());
        hex_groups(&blake3::derive_key(CHECK_CTX, &material)[..4])
    }
}

const TRUST_COMMIT_CTX: &str = "nhakobeam 2026-10 trust commit";
const TRUST_CHECK_CTX: &str = "nhakobeam 2026-10 trust check";

/// What a device asking to be trusted sends first: a commitment to its random number, revealed
/// only after the other device has sent its own. Neither side can pick its number to steer the
/// check value, so someone in the middle gets one guess in 4 billion at making both screens match.
pub fn trust_commit(nonce: &[u8; 32]) -> [u8; 32] {
    blake3::derive_key(TRUST_COMMIT_CTX, nonce)
}

/// The short value both screens show while one device asks the other to trust it, like `5D19 E04A`.
pub fn trust_check(asker_nonce: &[u8; 32], asked_nonce: &[u8; 32], asker: &EndpointId, asked: &EndpointId) -> String {
    let mut material = Vec::with_capacity(128);
    material.extend_from_slice(asker_nonce);
    material.extend_from_slice(asked_nonce);
    material.extend_from_slice(asker.as_bytes());
    material.extend_from_slice(asked.as_bytes());
    hex_groups(&blake3::derive_key(TRUST_CHECK_CTX, &material)[..4])
}

/// Constant time comparison for macs.
pub fn macs_equal(a: &[u8; 32], b: &[u8; 32]) -> bool {
    blake3::Hash::from_bytes(*a) == blake3::Hash::from_bytes(*b)
}

/// The windows a joiner should try at `now_secs`: those overlapping the last 11 minutes and the
/// next 2 (to allow for clock differences), newest first.
pub fn candidate_windows(now_secs: u64) -> Vec<u64> {
    let first = now_secs.saturating_sub(11 * 60) / WINDOW_SECS;
    let last = (now_secs + 2 * 60) / WINDOW_SECS;
    (first..=last).rev().collect()
}

/// `[0x4F, 0x2A, 0x91, 0xC3]` becomes `4F2A 91C3`.
pub fn hex_groups(bytes: &[u8]) -> String {
    let hex = data_encoding::HEXUPPER.encode(bytes);
    hex.as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).expect("hex is ascii"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The fingerprint shown for a device: first 4 bytes of its public key.
pub fn fingerprint(id: &EndpointId) -> String {
    hex_groups(&id.as_bytes()[..4])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(a: &str, b: &str, c: &str) -> [String; CODE_WORDS] {
        [a.into(), b.into(), c.into(), "harbor".into()]
    }

    #[test]
    fn same_words_same_keys_and_case_does_not_matter() {
        let a = CodeKeys::derive(&words("river", "lantern", "seven"), 7, KdfCost::TEST);
        let b = CodeKeys::derive(&words("River", " lantern", "SEVEN"), 7, KdfCost::TEST);
        assert_eq!(a.rendezvous_id(), b.rendezvous_id());
        let other = CodeKeys::derive(&words("river", "lantern", "seven"), 8, KdfCost::TEST);
        assert_ne!(a.rendezvous_id(), other.rendezvous_id(), "salt must change with the window");
        let other = CodeKeys::derive(&words("river", "lantern", "zebra"), 7, KdfCost::TEST);
        assert_ne!(a.rendezvous_id(), other.rendezvous_id());
    }

    #[test]
    fn macs_and_check_bind_both_ids() {
        let k = CodeKeys::derive(&words("river", "lantern", "seven"), 1, KdfCost::TEST);
        let host = SecretKey::generate().public();
        let joiner = SecretKey::generate().public();
        let mallory = SecretKey::generate().public();
        assert!(macs_equal(&k.join_mac(&joiner), &k.join_mac(&joiner)));
        assert!(!macs_equal(&k.join_mac(&joiner), &k.join_mac(&mallory)));
        assert!(!macs_equal(&k.welcome_mac(&joiner, &host), &k.welcome_mac(&joiner, &mallory)));
        let check = k.check(&host, &joiner);
        assert_eq!(check.len(), 9);
        assert_ne!(check, k.check(&mallory, &joiner));
    }

    #[test]
    fn windows_cover_the_code_lifetime() {
        // A code made at the very start of window 100 is still valid 10 minutes later.
        let made = 100 * WINDOW_SECS;
        for later in [0, 60, 300, 599, 600] {
            assert!(candidate_windows(made + later).contains(&100), "missed at +{later}s");
        }
        // Joiner clock 90 s behind the host.
        assert!(candidate_windows(made - 90).contains(&100));
        assert!(candidate_windows(made + 300).len() <= 3);
    }

    #[test]
    fn hex_groups_format() {
        assert_eq!(hex_groups(&[0x4f, 0x2a, 0x91, 0xc3]), "4F2A 91C3");
    }
}

#[cfg(test)]
mod timing {
    use super::*;

    /// `cargo test --release -p nhako-core real_cost -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_cost() {
        let words = ["river", "lantern", "seven", "harbor"].map(String::from);
        let start = std::time::Instant::now();
        for w in 0..5 {
            CodeKeys::derive(&words, w, KdfCost::REAL);
        }
        println!("argon2id 64 MiB x3: {:?} per code try", start.elapsed() / 5);
    }
}
