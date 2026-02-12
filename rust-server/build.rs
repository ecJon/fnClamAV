// Build script for ClamAV FFI linking
//
// This script links the binary against the pre-built libclamav.so
// that is located in app/lib directory.

use std::path::PathBuf;

fn main() {
    // 获取 libclamav.so 的绝对路径
    let mut lib_path = std::env::current_dir().expect("Cannot get current directory");
    lib_path.push("../app/lib");
    let lib_path = lib_path.canonicalize().unwrap_or(lib_path);

    // 检查库文件是否存在
    let libclamav_so = lib_path.join("libclamav.so.12");
    if libclamav_so.exists() {
        println!("cargo:rustc-link-search={}", lib_path.display());
        println!("cargo:rustc-link-lib=clamav");
        println!("cargo:rustc-link-arg=-Wl,--no-as-needed");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_path.display());
        println!("cargo:rerun-if-changed={}", libclamav_so.display());

        // 尝试直接链接具体的库文件
        println!("cargo:rustc-link-arg={}", libclamav_so.display());
    } else {
        // 尝试系统路径
        println!("cargo:rustc-link-lib=clamav");
    }

    // 链接 OpenSSL（ClamAV 依赖）
    println!("cargo:rustc-link-lib=ssl");
    println!("cargo:rustc-link-lib=crypto");
}
