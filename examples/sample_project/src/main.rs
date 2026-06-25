mod auth;
mod db;
mod api;
mod services;

fn main() {
    println!("Sample Project Starting...");
    let _ = auth::login::authenticate("admin", "password");
    let _ = db::models::User::new("admin");
}
