check:
  # starting from Rust 1.85.0 all -W and -A flags can be removed
  cargo +stable clippy -- --no-deps -W clippy::all -W clippy::pedantic -W clippy::nursery -W clippy::cargo -D warnings -A clippy::cargo_common_metadata -A clippy::redundant_pub_crate -A clippy::missing_errors_doc -A clippy::allow-attributes
  cargo +stable fmt -- --check
