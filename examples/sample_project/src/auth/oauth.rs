pub fn authorize(client_id: &str, redirect_uri: &str) -> String {
    format!("code_{}", client_id)
}

pub fn exchange_code(code: &str) -> String {
    format!("token_{}", code)
}

pub fn refresh_token(token: &str) -> String {
    format!("refreshed_{}", token)
}
