pub fn normalize_symbol(symbol: &str) -> Option<String> {
    let normalized = symbol.trim().to_ascii_uppercase();
    if normalized.is_empty()
        || !normalized
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return None;
    }
    Some(normalized)
}
