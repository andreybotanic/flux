#![forbid(unsafe_code)]

fn main() {
    let show_version = std::env::args().any(|arg| arg == "--version" || arg == "-V");

    if show_version {
        println!("{}", flux_core::engine_label());
    } else {
        println!("{} bootstrap shell", flux_core::engine_label());
        println!("Use --version to print the current bootstrap version.");
    }
}
