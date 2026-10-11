fn main() {
    println!("cargo:rerun-if-env-changed=GIT_TOYS_GITHUB_CLIENT_ID");
    tauri_build::build()
}
