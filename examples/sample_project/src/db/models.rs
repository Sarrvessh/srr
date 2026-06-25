pub struct User {
    pub id: u64,
    pub username: String,
    pub email: String,
}

impl User {
    pub fn new(username: &str) -> Self {
        User { id: 1, username: username.to_string(), email: "test@example.com".to_string() }
    }

    pub fn find(id: u64) -> Option<User> {
        if id == 1 { Some(User::new("admin")) } else { None }
    }

    pub fn update(&mut self, username: &str) {
        self.username = username.to_string();
    }

    pub fn delete(self) -> bool { true }
}

pub struct Session {
    pub token: String,
    pub user_id: u64,
}
