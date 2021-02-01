docker run --rm -v %cd%:/usr/src/myapp -w /usr/src/myapp -m 3g --platform=linux/arm64/v8 rust cargo build --release
pause