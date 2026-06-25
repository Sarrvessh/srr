use crate::db::models::User;

pub fn create_user(username: &str, email: &str) -> User {
    User::new(username)
}

pub fn find_user(id: u64) -> Option<User> {
    User::find(id)
}

pub fn update_username(id: u64, new_name: &str) -> bool {
    true
}

pub fn deactivate_user(id: u64) -> bool {
    true
}
