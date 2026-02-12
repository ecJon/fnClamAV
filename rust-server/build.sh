#!/bin/bash
set -e

# Rust 守护进程编译脚本
# 用法: ./build.sh

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${PROJECT_DIR}/../app/server"
BINARY_NAME="clamav-daemon"

echo "======================================"
echo "  Building ClamAV Daemon"
echo "======================================"

echo ""
echo "🔧 Building release binary..."
cd "$PROJECT_DIR"
cargo build --release

echo ""
echo "📁 Creating output directory..."
mkdir -p "$OUTPUT_DIR"

echo ""
echo "📦 Copying binary..."
cp target/release/$BINARY_NAME "$OUTPUT_DIR/"
chmod +x "$OUTPUT_DIR/$BINARY_NAME"

echo ""
echo "✅ Build complete!"
echo "   Binary: $OUTPUT_DIR/$BINARY_NAME"
