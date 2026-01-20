@echo off
echo Starting backend in development mode...
cargo run --no-default-features --release --features dev -- -f .\config\config.yaml
:: pause
