<p align="center">
<img src="assets/sibyl_icon_small.webp" align="center"alt="Project icon">
<h1 align="center">Sibyl System</h1>

<p align="center">
A simple Discord bot written in Rust that uses sentiment analysis to evaluate users.</br>
<a href="https://github.com/g-zhang/Sibyl-System-Bot/actions/workflows/rust.yml">
<img src="https://github.com/g-zhang/Sibyl-System-Bot/actions/workflows/rust.yml/badge.svg?branch=main" align="center" alt="Rust Build Status">
</a>
</p>
</p>

## Build Instructions
### Source
```
git clone --recurse-submodules https://github.com/g-zhang/Sibyl-System-Bot.git
```
### Compiling
#### On Windows
##### Prerequisites
* MSVC from [Visual Studio 2019](https://visualstudio.microsoft.com/downloads/) or [standalone](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
* Rust from [Rustup Installer](https://rustup.rs) (windows-msvc)
##### Build
```
cargo build
```
##### Run
```
cargo run
```

#### On other platforms
Not tested on other platforms, but should work. The windows specific code should be gated behind `[cfg(target_os = "windows")]` 
