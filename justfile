check:
  cargo clippy -- --no-deps -W clippy::all -W clippy::pedantic -W clippy::nursery -W clippy::cargo -D warnings -A clippy::cargo_common_metadata -A clippy::redundant_pub_crate 
  cargo fmt -- --check
