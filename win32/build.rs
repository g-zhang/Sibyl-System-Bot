// build.rs

#[cfg(target_os = "windows")]
fn main() {
    cc::Build::new()
        .cpp(true)
        .file("src/win32.cpp")
        .flag("/EHsc")
        .flag("/Qspectre-load")
        .flag("/guard:cf")
        .warnings(true)
        .warnings_into_errors(true)
        .compile("win32");
}
