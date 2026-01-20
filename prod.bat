@echo off
echo Building backend for production...
cargo build --no-default-features --features prod --release
if %ERRORLEVEL% EQU 0 (
    echo Build successful!
) else (
    echo Build failed!
)
pause
