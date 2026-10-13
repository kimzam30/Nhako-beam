//! The pairing code word list, shared with the app (`desktop/src/lib/engine/words.ts`).

use std::sync::LazyLock;

use rand::RngExt;

const LIST: &str = include_str!("words.txt");

pub static WORDS: LazyLock<Vec<&'static str>> = LazyLock::new(|| LIST.lines().map(str::trim).filter(|w| !w.is_empty()).collect());

pub fn is_word(w: &str) -> bool {
    let w = w.trim().to_ascii_lowercase();
    WORDS.binary_search(&w.as_str()).is_ok()
}

/// How many words a pairing code has. Four from 256 is 4.3 billion codes (see docs/protocol/nhako-1.md, section 7).
pub const CODE_WORDS: usize = 4;

pub fn random_words() -> [String; CODE_WORDS] {
    let mut rng = rand::rng();
    std::array::from_fn(|_| WORDS[rng.random_range(0..WORDS.len())].to_string())
}

/// Splits typed or pasted text like "River-lantern seven harbor" or "nhakobeam://pair/river-lantern-seven-harbor" into words.
pub fn split_code(text: &str) -> Vec<String> {
    let text = text.trim();
    let text = text.strip_prefix("nhakobeam://pair/").unwrap_or(text);
    text.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_lowercase())
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_256_sorted_unique_words() {
        assert_eq!(WORDS.len(), 256);
        assert!(WORDS.windows(2).all(|w| w[0] < w[1]), "words.txt must stay sorted and unique");
        assert!(WORDS.iter().all(|w| (3..=8).contains(&w.len()) && w.bytes().all(|b| b.is_ascii_lowercase())));
    }

    #[test]
    fn checks_and_splits() {
        assert!(is_word(" River "));
        assert!(!is_word("rivers"));
        assert_eq!(split_code("River-lantern  seven harbor"), ["river", "lantern", "seven", "harbor"]);
        assert_eq!(
            split_code("nhakobeam://pair/river-lantern-seven-harbor"),
            ["river", "lantern", "seven", "harbor"]
        );
        let w = random_words();
        assert!(w.iter().all(|w| is_word(w)));
    }
}
