use crate::cbr::models::CurrencyItem;

pub fn normalize_symbol(symbol: &str) -> Option<String> {
    let normalized = symbol.trim().to_ascii_uppercase();
    (normalized.len() == 3 && normalized.bytes().all(|byte| byte.is_ascii_uppercase()))
        .then_some(normalized)
}

pub fn currency_for<'a>(items: &'a [CurrencyItem], symbol: &str) -> Option<&'a CurrencyItem> {
    items.iter().find(|item| {
        item.char_code
            .as_deref()
            .is_some_and(|code| code.eq_ignore_ascii_case(symbol))
    })
}

#[cfg(test)]
mod tests {
    use super::{currency_for, normalize_symbol};
    use crate::cbr::models::CurrencyItem;

    #[test]
    fn normalizes_three_letter_iso_currency_codes() {
        assert_eq!(normalize_symbol(" usd ").as_deref(), Some("USD"));
        assert_eq!(normalize_symbol("CNY").as_deref(), Some("CNY"));
        assert_eq!(normalize_symbol("US1"), None);
        assert_eq!(normalize_symbol("USDD"), None);
        assert_eq!(normalize_symbol("$UD"), None);
    }

    #[test]
    fn resolves_currency_codes_case_insensitively() {
        let items = vec![CurrencyItem {
            id: "R01235".to_owned(),
            char_code: Some("USD".to_owned()),
        }];
        assert_eq!(currency_for(&items, "usd").unwrap().id, "R01235");
        assert!(currency_for(&items, "EUR").is_none());
    }
}
