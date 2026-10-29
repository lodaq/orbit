pub fn text_is_rtl(title: &str) -> bool {
    use unicode_bidi::BidiClass::{AL, L, R};
    title
        .chars()
        .find_map(|c| match unicode_bidi::bidi_class(c) {
            R | AL => Some(true),
            L => Some(false),
            _ => None,
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uses_first_strong_character() {
        for text in [
            "📚 123 اقرأ الكتاب (Rust)",
            "  الحمد لله",
            "نص تجريبي من Orbit",
            "قد تساعدك كلمة لطيفة.",
        ] {
            assert!(text_is_rtl(text), "{text}");
        }
        for text in ["Read كتاب", "kind · لطيف", "123 📚", ""] {
            assert!(!text_is_rtl(text), "{text}");
        }
    }
}
