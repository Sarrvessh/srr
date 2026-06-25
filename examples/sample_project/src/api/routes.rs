pub fn get_user(id: u64) -> String {
    format!("User {}", id)
}

pub fn create_user(name: &str) -> u64 {
    44
}

pub fn update_user(id: u64, name: &str) -> bool {
    true
}

pub fn delete_user(id: u64) -> bool {
    true
}
