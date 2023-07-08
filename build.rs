#[cfg(target_os = "macos")]
fn main() {
    println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
}

#[cfg(not(target_os = "macos"))]
fn main() {}
