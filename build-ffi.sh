#!/bin/bash
set -e

# ClamAV FFI 版本打包脚本
# 用法: ./build-ffi.sh

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${PROJECT_DIR}/dist"
BUILD_TEMP="/tmp/fpk_build_$$"

echo "======================================"
echo "  Building ClamAV Antivirus (FFI)"
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

# 编译 Rust 守护进程 (FFI 版本)
echo ""
echo "🦀 Building Rust daemon (FFI version)..."
cd "${PROJECT_DIR}/rust-server"
cargo build --release --quiet
mkdir -p "${PROJECT_DIR}/app/server"
cp target/release/clamav-daemon "${PROJECT_DIR}/app/server/"
chmod +x "${PROJECT_DIR}/app/server/clamav-daemon"
echo "✅ clamav-daemon built successfully"

# 检查 ClamAV FFI 依赖
echo ""
echo "🔍 Checking ClamAV FFI dependencies..."

# 检查 libclamav.so 动态库
CLAMAV_LIB_DIR="${PROJECT_DIR}/app/lib"
if [ -d "${CLAMAV_LIB_DIR}" ]; then
    echo "✅ libclamav.so found:"
    ls -lh "${CLAMAV_LIB_DIR}/"libclamav.so*
else
    echo "⚠️  libclamav.so NOT found in ${CLAMAV_LIB_DIR}/"
    echo ""
    echo "To build ClamAV shared library, run:"
    echo "  ./build-clamav-shared.sh"
    echo ""
    exit 1
fi

# 检查 freshclam（用于病毒库更新）
CLAMAV_BIN_DIR="${PROJECT_DIR}/app/bin"
if [ -f "${CLAMAV_BIN_DIR}/freshclam" ]; then
    echo "✅ freshclam found:"
    ls -lh "${CLAMAV_BIN_DIR}/freshclam"
else
    echo "⚠️  freshclam NOT found in ${CLAMAV_BIN_DIR}/"
    echo ""
    echo "freshclam is required for virus database updates."
    echo "To build ClamAV with freshclam, run:"
    echo "  ./build-clamav-shared.sh"
    echo ""
    exit 1
fi

# 显示病毒库信息
echo ""
echo "🦠 Checking virus database..."
CLAMAV_DB_DIR="${PROJECT_DIR}/app/share/clamav"
if [ -d "${CLAMAV_DB_DIR}" ]; then
    DB_SIZE=$(du -sh "${CLAMAV_DB_DIR}" 2>/dev/null | cut -f1)
    echo "✅ Virus database found (size: ${DB_SIZE}):"
    ls -lh "${CLAMAV_DB_DIR}/"*.cvd 2>/dev/null || ls -lh "${CLAMAV_DB_DIR}/"
else
    echo "⚠️  Virus database directory not found: ${CLAMAV_DB_DIR}"
fi

# 检查 freshclam 配置
echo ""
echo "📝 Checking freshclam configuration..."
FRESHCLAM_CONF="${PROJECT_DIR}/app/config/freshclam.conf"
if [ -f "${FRESHCLAM_CONF}" ]; then
    echo "✅ freshclam.conf found"
else
    echo "⚠️  freshclam.conf NOT found"
fi

# 显示 app 目录结构
echo ""
echo "📦 App directory structure:"
cd "${PROJECT_DIR}/app"
tree -L 2 . 2>/dev/null || find . -maxdepth 2 -type d | sort

# 创建 app.tgz（包含所有运行时文件）
echo ""
echo "📦 Creating app.tgz..."
tar -czf "${BUILD_TEMP}/app.tgz" .

# 获取 app.tgz 大小
APP_TGZ_SIZE=$(du -h "${BUILD_TEMP}/app.tgz" | cut -f1)
echo "✅ app.tgz created (${APP_TGZ_SIZE})"

# 创建最终的 .fpk 包
echo ""
echo "🎁 Creating .fpk package..."
cd "${BUILD_TEMP}"
FPK_NAME="fnnas.clamav.fpk"
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
echo "🚀 Ready to install on fnOS!"
echo ""
