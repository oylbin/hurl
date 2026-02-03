$env:VCPKG_ROOT = "C:\src\vcpkg"
$env:VCPKGRS_DYNAMIC = "0"
$env:VCPKGRS_TRIPLET = "x64-windows-static"
$env:RUST_BACKTRACE="full"
cargo build
