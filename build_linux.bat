docker run --rm -v %cd%:/usr/src/myapp -w /usr/src/myapp -m 3g --platform=linux/amd64 rust cargo build --release
pause