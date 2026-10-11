#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod ai;
mod app;
mod auth;
mod github;
fn main() {
    if git_toys_engine::rebase_editor_entry() {
        return;
    }
    if std::env::var_os("GIT_TOYS_AUTH_ADDRESS").is_some() {
        auth::askpass();
        return;
    }
    if let Err(error) = app::run() {
        eprintln!("Git Toys could not start: {error}");
        std::process::exit(1);
    }
}
