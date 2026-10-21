use deunicode::AsciiChars;

// Current PDN display profile. Keep stored text intact so other clients can use it.
pub(in crate::routes) fn display_text(value: &str) -> String {
    // The library drops some controls; replace them first so they remain visible.
    let text: String = value
        .chars()
        .map(|character| {
            if character.is_control() {
                '?'
            } else {
                character
            }
        })
        .collect();
    text.ascii_chars()
        .flat_map(|part| part.unwrap_or("?").chars())
        .map(|character| match character {
            ' '..='~' => character,
            _ => '?',
        })
        .take(255)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::display_text;

    #[test]
    fn printable_ascii_is_preserved() {
        let printable: String = (' '..='~').collect();
        assert_eq!(display_text(&printable), printable);
        assert_eq!(display_text(""), "");
    }

    #[test]
    fn transliterates_names_and_replaces_unknown_or_control_characters() {
        assert_eq!(display_text("José Ægir Jose\u{301}"), "Jose AEgir Jose");
        assert_eq!(display_text("A\n\t\0\u{7f}\u{10ffff}B"), "A?????B");
    }

    #[test]
    fn limits_apply_after_transliteration() {
        assert_eq!(display_text(&"a".repeat(255)), "a".repeat(255));
        assert_eq!(display_text(&"a".repeat(256)), "a".repeat(255));
        assert_eq!(
            display_text(&format!("{}Æ", "a".repeat(254))),
            format!("{}A", "a".repeat(254))
        );
    }

    #[quickcheck_macros::quickcheck]
    fn any_text_fits_the_pdn_display(value: String) -> bool {
        let display = display_text(&value);
        display.len() <= 255 && display.bytes().all(|byte| (b' '..=b'~').contains(&byte))
    }
}
