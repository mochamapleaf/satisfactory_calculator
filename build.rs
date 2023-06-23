#[cfg(target_os = "macos")]
fn main(){
	println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
}
