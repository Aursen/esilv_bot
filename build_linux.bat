docker run --rm -v %cd%:/usr/src/myapp -w /usr/src/myapp -m 6g --platform=linux/amd64 rust cargo build --release --bin leo_website
pause