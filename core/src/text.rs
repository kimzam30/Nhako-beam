//! Text from other devices (device names, file names) made safe to show: no control characters,
//! and nothing invisible that could make a name look like something else. A right-to-left
//! override can make `invoice{U+202E}fdp.exe` show as `invoiceexe.pdf`, and zero-width characters can
//! make a copy of "Kim's Pixel" that only looks the same.

/// The longest device name, in characters. The apps' rename fields stop at the same length.
pub const NAME_MAX: usize = 32;

/// Characters that change how text around them is shown, or show nothing at all: direction
/// marks, embeddings, overrides and isolates, zero-width characters, word joiners, the byte
/// order mark, interlinear annotation marks, and Unicode tag characters.
pub fn is_invisible(c: char) -> bool {
    matches!(c,
        '\u{00AD}' | '\u{034F}' | '\u{061C}' | '\u{115F}' | '\u{1160}' | '\u{17B4}' | '\u{17B5}' | '\u{180B}'..='\u{180F}'
        | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{206F}' | '\u{3164}' | '\u{FE00}'..='\u{FE0F}'
        | '\u{FEFF}' | '\u{FFA0}' | '\u{FFF0}'..='\u{FFFB}' | '\u{E0000}'..='\u{E0FFF}')
}

/// A device name as this device shows it: visible characters only, spaces collapsed, at most
/// [`NAME_MAX`] characters. Empty when nothing visible is left.
pub fn device_name(raw: &str) -> String {
    let visible: String = raw.chars().filter(|c| !is_invisible(*c)).collect();
    let mut out = String::new();
    for word in visible.split(|c: char| c.is_whitespace() || c.is_control()).filter(|w| !w.is_empty()) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out.chars().take(NAME_MAX).collect::<String>().trim_end().to_string()
}

/// Characters that reorder the text around them: direction embeddings, overrides and isolates.
/// They could make a link read as another one (`https://bank.example/{U+202E}moc.live`).
fn reorders(c: char) -> bool {
    matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// Text from another device made safe to show and copy: line breaks and tabs stay (Windows line
/// endings become plain ones), other control characters and the ones that reorder text go.
/// Everything else stays, emoji joiners and right-to-left letters included.
pub fn clean_text(raw: &str) -> String {
    raw.replace("\r\n", "\n")
        .chars()
        .filter(|c| *c == '\n' || *c == '\t' || !(c.is_control() || reorders(*c)))
        .collect()
}

/// The text is one web link and nothing else (spaces around it aside), so it can be opened.
pub fn is_link(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with("https://") || t.starts_with("http://")) && t.len() > 8 && !t.chars().any(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_keeps_lines_and_loses_tricks() {
        assert_eq!(clean_text("a\r\nb\tc\u{7}"), "a\nb\tc");
        assert_eq!(clean_text("https://bank.example/\u{202E}moc.live"), "https://bank.example/moc.live");
        assert_eq!(
            clean_text("👨\u{200D}👩\u{200D}👧 مرحبا"),
            "👨\u{200D}👩\u{200D}👧 مرحبا",
            "emoji joiners and Arabic stay"
        );
        assert!(is_link(" https://nhako.example/a?b=c \n"));
        assert!(!is_link("see https://nhako.example"));
        assert!(!is_link("javascript:alert(1)"));
        assert!(!is_link("https://"));
    }

    #[test]
    fn names_lose_what_hides_or_disguises() {
        assert_eq!(device_name("  Kim's   Pixel "), "Kim's Pixel");
        assert_eq!(device_name("Kim\u{200B}'s Pixel"), "Kim's Pixel");
        assert_eq!(device_name("Pixel\u{202E}lexiP"), "PixellexiP");
        assert_eq!(device_name("tab\there\nline"), "tab here line");
        assert_eq!(device_name("\u{E0041}\u{FEFF}"), "");
        assert_eq!(device_name(&"x".repeat(5000)).chars().count(), NAME_MAX);
        assert_eq!(device_name("Ünïcödé 電腦 📱"), "Ünïcödé 電腦 📱", "real letters and emoji stay");
    }
}
