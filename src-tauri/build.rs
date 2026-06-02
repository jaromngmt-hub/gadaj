use cmake::Config;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../vendor/parakeet.cpp");
    println!("cargo:rustc-check-cfg=cfg(parakeet_built)");

    let parakeet_dir = std::path::Path::new("../vendor/parakeet.cpp");

    if !parakeet_dir.exists() {
        println!(
            "cargo:warning=parakeet.cpp submodule not found at {}. Run:\n  git submodule update --init --recursive",
            parakeet_dir.display()
        );
        // Stub: build minimal placeholder. Tauri commands that need STT will fail
        // gracefully with a clear error message.
        return;
    }

    println!("cargo:rustc-cfg=parakeet_built");

    println!("cargo:warning=Buduję parakeet.cpp (to może potrwać kilka minut przy pierwszym buildzie)...");

    let mut config = Config::new(parakeet_dir);

    // Disable optional features we don't need
    config
        .define("PARAKEET_BUILD_TESTS", "OFF")
        .define("PARAKEET_BUILD_CLI", "OFF")
        .define("PARAKEET_SHARED", "OFF");

    // Per-platform GPU backends
    if cfg!(target_os = "macos") {
        config.define("PARAKEET_GGML_METAL", "ON");
    } else if cfg!(target_os = "windows") {
        config.define("PARAKEET_GGML_VULKAN", "ON");
    }

    // Build and install
    let dst = config.build();

    // Link the static library
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=parakeet");

    // ggml needs zlib on some platforms
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=framework=Foundation");
    }

    // Tell Rust to rerun if parakeet.cpp source changes
    println!("cargo:rerun-if-changed=../vendor/parakeet.cpp/CMakeLists.txt");
    println!("cargo:rerun-if-changed=../vendor/parakeet.cpp/src");
    println!("cargo:rerun-if-changed=../vendor/parakeet.cpp/include");
    println!("cargo:rerun-if-changed=../vendor/parakeet.cpp/third_party/ggml");
}
