use std::env;
use std::path::PathBuf;

fn main() {
    let llvm_prefix = env::var("LLVM_SYS_210_PREFIX")
        .unwrap_or_else(|_| "/workspaces/workspace/llvm-project/build-genpay".to_string());

    let lib_dir = PathBuf::from(&llvm_prefix).join("lib");

    // Tell cargo where to find libraries
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // Force whole-archive inclusion by passing the absolute paths directly to the linker
    println!("cargo:rustc-link-arg=-Wl,--whole-archive");
    println!(
        "cargo:rustc-link-arg={}/libLLVMGenpayCodeGen.a",
        lib_dir.display()
    );
    println!(
        "cargo:rustc-link-arg={}/libLLVMGenpayDesc.a",
        lib_dir.display()
    );
    println!(
        "cargo:rustc-link-arg={}/libLLVMGenpayInfo.a",
        lib_dir.display()
    );
    println!("cargo:rustc-link-arg=-Wl,--no-whole-archive");

    // Re-run build script if LLVM path changes
    println!("cargo:rerun-if-env-changed=LLVM_SYS_210_PREFIX");
}
