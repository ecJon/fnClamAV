#!/bin/bash
set -e

# ClamAV 杀毒软件打包脚本
# 用法: ./build.sh

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${PROJECT_DIR}/dist"
BUILD_TEMP="/tmp/fpk_build_$$"

echo "======================================"
echo "  Building ClamAV Antivirus"
echo "======================================"

# 清理并创建输出目录
echo ""
echo "📁 Creating output directory..."
rm -rf "${OUTPUT_DIR}"
mkdir -p "${OUTPUT_DIR}"
rm -rf "${BUILD_TEMP}"
mkdir -p "${BUILD_TEMP}"

# 复制根目录文件
echo "📋 Copying root files..."
cp "${PROJECT_DIR}/manifest" "${BUILD_TEMP}/"
cp "${PROJECT_DIR}/ICON.PNG" "${BUILD_TEMP}/"
cp "${PROJECT_DIR}/ICON_256.PNG" "${BUILD_TEMP}/"

# 复制目录
echo "📂 Copying directories..."
cp -r "${PROJECT_DIR}/cmd" "${BUILD_TEMP}/"
cp -r "${PROJECT_DIR}/config" "${BUILD_TEMP}/"
cp -r "${PROJECT_DIR}/wizard" "${BUILD_TEMP}/"

# 编译 Rust 守护进程
echo ""
echo "🦀 Building Rust daemon..."
cd "${PROJECT_DIR}/rust-server"
cargo build --release --quiet
mkdir -p "${PROJECT_DIR}/app/server"
cp target/release/clamav-daemon "${PROJECT_DIR}/app/server/"
chmod +x "${PROJECT_DIR}/app/server/clamav-daemon"

# 检查 ClamAV 二进制和病毒库
echo ""
echo "🔍 Checking ClamAV binaries..."
CLAMAV_BIN_DIR="${PROJECT_DIR}/app/bin"
if [ -f "${CLAMAV_BIN_DIR}/clamscan.bin" ] && [ -f "${CLAMAV_BIN_DIR}/freshclam.bin" ]; then
    echo "✅ ClamAV binaries found:"
    ls -lh "${CLAMAV_BIN_DIR}/"
else
    echo "⚠️  ClamAV binaries NOT found in ${CLAMAV_BIN_DIR}/"
    echo ""
    echo "To add ClamAV support, choose one:"
    echo "  1. Copy from system (if installed):"
    echo "     ./copy-clamav.sh"
    echo "  2. Build from source:"
    echo "     ./build-clamav.sh"
    echo ""
fi

# 显示病毒库信息
echo ""
echo "🦠 Checking virus database..."
CLAMAV_DB_DIR="${PROJECT_DIR}/app/share/clamav"
if [ -d "${CLAMAV_DB_DIR}" ]; then
    DB_SIZE=$(du -sh "${CLAMAV_DB_DIR}" 2>/dev/null | cut -f1)
    echo "✅ Virus database found (size: ${DB_SIZE}):"
    ls -lh "${CLAMAV_DB_DIR}/"
else
    echo "⚠️  Virus database directory not found: ${CLAMAV_DB_DIR}"
fi

# 创建 app.tgz（包含病毒库文件，方便国内用户首次使用）
echo ""
echo "📦 Creating app.tgz..."
cd "${PROJECT_DIR}/app"
tar -czf "${BUILD_TEMP}/app.tgz" .

# 创建最终的 .fpk 包
echo "🎁 Creating .fpk package..."
cd "${BUILD_TEMP}"
FPK_NAME="App.Native.ClamAVAntivirus.fpk"
tar -czf "${OUTPUT_DIR}/${FPK_NAME}" .

# 获取文件大小
FPK_SIZE=$(du -h "${OUTPUT_DIR}/${FPK_NAME}" | cut -f1)

# 清理临时目录
rm -rf "${BUILD_TEMP}"

echo ""
echo "======================================"
echo "  ✅ Build complete!"
echo "======================================"
echo "📦 Package: ${OUTPUT_DIR}/${FPK_NAME}"
echo "📊 Size: ${FPK_SIZE}"
echo ""
