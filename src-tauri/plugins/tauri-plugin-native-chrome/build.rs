fn main() {
    tauri_plugin::Builder::new(&["attach", "detach"])
        .ios_path("ios")
        .build();
}
