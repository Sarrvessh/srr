pub fn auth_middleware(token: &str) -> bool {
    !token.is_empty()
}

pub fn rate_limiter(ip: &str) -> bool {
    ip != "0.0.0.0"
}
