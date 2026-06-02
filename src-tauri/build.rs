use std::path::PathBuf;
use std::process::Command;

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
        return;
    }

    let manifest_path = parakeet_dir.to_path_buf();
    let build_dir = parakeet_dir.join("build");

    // Uruchom cmake configure (jeśli build/ nie istnieje albo jest pusty)
    let needs_configure = !build_dir.join("CMakeCache.txt").exists();
    if needs_configure {
        let mut cmd = Command::new("cmake");
        cmd.arg("-B").arg(&build_dir);
        cmd.arg("-S").arg(&manifest_path);
        cmd.arg("-DPARAKEET_BUILD_TESTS=OFF");
        cmd.arg("-DPARAKEET_BUILD_CLI=OFF");
        cmd.arg("-DPARAKEET_SHARED=OFF");
        if cfg!(target_os = "macos") {
            cmd.arg("-DPARAKEET_GGML_METAL=ON");
        } else if cfg!(target_os = "windows") {
            cmd.arg("-DPARAKEET_GGML_VULKAN=ON");
        }
        println!("cargo:warning=Uruchamiam cmake configure dla parakeet.cpp...");
        let status = cmd.status().expect("Nie udało się uruchomić cmake");
        if !status.success() {
            panic!("cmake configure nie powiodło się (kod: {:?})", status.code());
        }
    }

    // Uruchom cmake build
    let mut cmd = Command::new("cmake");
    cmd.arg("--build").arg(&build_dir);
    cmd.arg("--config").arg(if cfg!(debug_assertions) { "Debug" } else { "Release" });
    let nproc = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    cmd.arg("-j").arg(nproc.to_string());
    println!("cargo:warning=Buduję parakeet.cpp (cmake --build, to może potrwać kilka minut)...");
    let status = cmd.status().expect("Nie udało się uruchomić cmake --build");
    if !status.success() {
        panic!("cmake build nie powiodło się (kod: {:?})", status.code());
    }

    // Biblioteka statyczna jest w build/lib/libparakeet.{a,lib}
    let lib_path = build_dir.join("lib");
    let lib = if cfg!(target_os = "windows") {
        "parakeet.lib"
    } else {
        "libparakeet.a"
    };
    let lib_file = lib_path.join(lib);
    if !lib_file.exists() {
        // Spróbuj w samym build/
        let alt = build_dir.join(lib);
        if !alt.exists() {
            panic!(
                "Nie znaleziono biblioteki {} ani {}",
                lib_file.display(),
                alt.display()
            );
        }
    }

    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-lib=static=parakeet");

    // ggml buduje się jako shared library - flat katalog ze wszystkimi dylibami
    let ggml_src = build_dir.join("third_party/ggml/src");
    let ggml_flat = stage_ggml_libs(&ggml_src);
    println!("cargo:rustc-link-search=native={}", ggml_flat.display());

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=ggml");
        println!("cargo:rustc-link-lib=dylib=ggml-base");
        println!("cargo:rustc-link-lib=dylib=ggml-cpu");
        println!("cargo:rustc-link-lib=dylib=ggml-blas");
        println!("cargo:rustc-link-lib=dylib=ggml-metal");
        // libc++ jest potrzebne bo libparakeet.a ma C++ std lib symbole
        println!("cargo:rustc-link-lib=dylib=c++");
        println!("cargo:rustc-link-lib=framework=Accelerate");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=Metal");
        println!("cargo:rustc-link-lib=framework=MetalKit");
        // rpath dla dev mode (bin obok dylib w target/debug/)
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
        // rpath dla bundled .app (Contents/MacOS/ → Contents/Frameworks/)
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
    }
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=dylib=ggml");
        println!("cargo:rustc-link-lib=dylib=ggml-cpu");
        println!("cargo:rustc-link-lib=dylib=ggml-blas");
        println!("cargo:rustc-link-lib=dylib=ggml-metal");
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    }
    if cfg!(target_os = "windows") {
        // Windows używa .dll
        println!("cargo:rustc-link-lib=dylib=ggml");
        println!("cargo:rustc-link-lib=dylib=ggml-cpu");
    }

    println!("cargo:rustc-cfg=parakeet_built");

    // Skopiuj ggml dyliby obok naszej binarki (cargo run)
    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        let mut path = PathBuf::from(&out_dir);
        for _ in 0..4 {
            path.pop();
        }
        for sub in &["debug", "release"] {
            let bin_dir = path.join(sub);
            if bin_dir.exists() {
                copy_ggml_dylibs(&ggml_src, &bin_dir);
            }
        }
    }

    // Poinformuj cargo żeby rerun budowania gdy parakeet źródła się zmienią
    println!("cargo:rerun-if-changed=../vendor/parakeet.cpp/CMakeLists.txt");
    println!("cargo:rerun-if-changed=../vendor/parakeet.cpp/src");
    println!("cargo:rerun-if-changed=../vendor/parakeet.cpp/include");
    println!("cargo:rerun-if-changed=../vendor/parakeet.cpp/third_party/ggml");
}

fn copy_ggml_dylibs(src_dir: &std::path::Path, dst_dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(src_dir) else { return; };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file() {
            if let Some(ext) = p.extension() {
                if ext == "dylib" {
                    if let Some(name) = p.file_name() {
                        let dst = dst_dir.join(name);
                        let _ = std::fs::copy(&p, &dst);
                    }
                }
            }
        }
    }
    // Skopiuj też z subfolderów (ggml-blas, ggml-metal)
    if let Ok(subdirs) = std::fs::read_dir(src_dir) {
        for sub in subdirs.flatten() {
            let subp = sub.path();
            if subp.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&subp) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_file() {
                            if let Some(ext) = p.extension() {
                                if ext == "dylib" {
                                    if let Some(name) = p.file_name() {
                                        let dst = dst_dir.join(name);
                                        let _ = std::fs::copy(&p, &dst);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Tworzy flat katalog ze wszystkimi ggml dylibami (do linkowania) i kopiuje je do bin dir.
fn stage_ggml_libs(ggml_src: &std::path::Path) -> PathBuf {
    let stage = ggml_src.parent().unwrap().join("flat");
    let _ = std::fs::create_dir_all(&stage);
    copy_ggml_dylibs(ggml_src, &stage);
    stage
}
