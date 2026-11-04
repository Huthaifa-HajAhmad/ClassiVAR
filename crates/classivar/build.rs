// Windows gives the main thread only 1 MB of stack. The screen is a deep tree of
// elements, and in a debug build that was not enough: the app closed with "stack
// overflow" when the Reported variant button was clicked. This asks the linker for 16 MB.
fn main() {
    let is_windows_msvc = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if is_windows_msvc {
        println!("cargo:rustc-link-arg-bins=/STACK:16777216");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
