fn main() {
    // Man pages and completions are generated via `envsafe completions <shell>`
    // and `envsafe man-page` at runtime.
    // This build.rs is a placeholder for future build-time generation.
    println!("cargo:rerun-if-changed=build.rs");
}
