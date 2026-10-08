use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

pub fn normalize_text(value: &str) -> String {
    let decomposed = value
        .nfkd()
        .filter(|character| !is_combining_mark(*character));
    let mut normalized = String::with_capacity(value.len());
    let mut previous_was_space = true;

    for character in decomposed.flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            normalized.push(character);
            previous_was_space = false;
        } else if !previous_was_space {
            normalized.push(' ');
            previous_was_space = true;
        }
    }

    normalized.trim().to_string()
}

pub fn normalize_street(value: &str) -> String {
    normalize_text(value)
        .split_whitespace()
        .map(|token| match token {
            "av" | "aven" | "avenida" => "avenida",
            "r" | "rua" => "rua",
            "rod" | "rodovia" => "rodovia",
            "trav" | "travessa" => "travessa",
            "al" | "alameda" => "alameda",
            "estr" | "estrada" => "estrada",
            _ => token,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{normalize_street, normalize_text};

    #[test]
    fn normalizes_unicode_spacing_and_case() {
        assert_eq!(normalize_text("  São   José-d'Ávila "), "sao jose d avila");
    }

    #[test]
    fn expands_common_street_type_abbreviations() {
        assert_eq!(normalize_street("Av. Paulista"), "avenida paulista");
    }
}
