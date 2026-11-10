fn main() {
    let version_path = std::path::Path::new(".version.json");
    if version_path.is_file() {
        if let Ok(contents) = std::fs::read_to_string(version_path) {
            if let Ok(state) = serde_json::from_str::<serde_json::Value>(&contents) {
                if let Some(major) = state.get("major").and_then(|v| v.as_u64()) {
                    println!("cargo:rustc-env=LPPO_VERSION_MAJOR={}", major);
                }
                if let Some(minor) = state.get("minor").and_then(|v| v.as_u64()) {
                    println!("cargo:rustc-env=LPPO_VERSION_MINOR={}", minor);
                }
                if let Some(patch) = state.get("patch").and_then(|v| v.as_u64()) {
                    println!("cargo:rustc-env=LPPO_VERSION_PATCH={}", patch);
                }
                if let Some(build) = state.get("build").and_then(|v| v.as_u64()) {
                    println!("cargo:rustc-env=LPPO_VERSION_BUILD={}", build);
                }
                if let Some(channel) = state.get("channel").and_then(|v| v.as_str()) {
                    println!("cargo:rustc-env=LPPO_VERSION_CHANNEL={}", channel);
                }
            }
        }
    }
    // No build-time actions required for LPPO.
}
