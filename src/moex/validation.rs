pub fn validate_symbol(symbol: &str) -> bool {
    !symbol.is_empty()
        && (!symbol.contains('_')
            || symbol.split('_').all(|part| {
                !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_alphanumeric())
            }))
        && symbol
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
