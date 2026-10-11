//! Standalone editor helper for integration tests. The desktop executable uses the same entrypoint.
fn main() {
    if !git_toys_engine::rebase_editor_entry() {
        std::process::exit(1);
    }
}
