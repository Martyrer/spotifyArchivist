fn main() {
    // option_env! bakes this at compile time; force a rebuild when it changes.
    println!("cargo::rerun-if-env-changed=SPOTIFY_ARCHIVIST_CLIENT_ID");
    tauri_build::build();
}
