fn main() {
    tauri_build::build();
    println!("cargo:rustc-env=SYSTEM_PROXY_URL=http://127.0.0.1:10090");
}

