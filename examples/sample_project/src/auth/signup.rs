pub fn register_user(username: &str, email: &str, password: &str) -> Result<u64, String> {
    if username.is_empty() {
        return Err("Username required".to_string());
    }
    Ok(43)
}

pub fn verify_email(token: &str) -> bool {
    token.len() == 32
}

pub fn send_confirmation(email: &str) -> bool {
    email.contains('@')
}
