#!/bin/bash
set -e

# ClamAV 1.5.1 静态编译脚本（使用本地源码）
# 用法: ./build-clamav-local.sh

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLAMAV_SRC_DIR="${PROJECT_DIR}/clamAV"
CLAMAV_BUILD_DIR="${CLAMAV_SRC_DIR}/build-static"
CLAMAV_OUTPUT_DIR="${PROJECT_DIR}/app/bin"

echo "======================================"
echo "  Building ClamAV from local source"
echo "======================================"

# 检查源码目录
if [ ! -d "${CLAMAV_SRC_DIR}" ]; then
    echo "❌ ClamAV source not found at ${CLAMAV_SRC_DIR}"
    exit 1
fi

# 显示当前版本
cd "${CLAMAV_SRC_DIR}"
VERSION=$(git describe --tags 2>/dev/null || echo "unknown")
echo "📌 ClamAV version: ${VERSION}"

# 清理旧文件
echo "🧹 Cleaning old binaries..."
rm -f "${CLAMAV_OUTPUT_DIR}/clamscan" "${CLAMAV_OUTPUT_DIR}/freshclam"
rm -rf "${CLAMAV_BUILD_DIR}"

# 创建目录
mkdir -p "${CLAMAV_BUILD_DIR}"
mkdir -p "${CLAMAV_OUTPUT_DIR}"

# 检查依赖
echo "🔍 Checking dependencies..."
if ! command -v cmake &> /dev/null; then
    echo "❌ cmake not found. Install: sudo apt-get install -y cmake"
    exit 1
fi

cd "${CLAMAV_BUILD_DIR}"

# 配置（静态编译）
echo "⚙️  Configuring for static build..."
cmake .. \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX=/usr/local \
    -DENABLE_STATIC_LIB=ON \
    -DENABLE_SHARED_LIB=OFF \
    -DBUILD_SHARED_LIBS=OFF \
    -DENABLE_JSON=OFF \
    -DENABLE_CURSORS=OFF \
    -DENABLE_MILTER=OFF \
    -DENABLE_CLAMSUBMIT=OFF \
    -DENABLE_CLAMONACC=OFF \
    -DENABLE_TESTS=OFF \
    -DENABLE_EXAMPLES=OFF

# 编译库和二进制
echo "🔨 Building (this may take 10-20 minutes)..."
make -j$(nproc) libclamav clamscan freshclam

# 查找并复制二进制文件
echo "📋 Copying binaries..."
FOUND=0

# 尝试不同位置
if [ -f "clamscan/clamscan" ]; then
    cp clamscan/clamscan "${CLAMAV_OUTPUT_DIR}/"
    FOUND=$((FOUND + 1))
fi
if [ -f "freshclam/freshclam" ]; then
    cp freshclam/freshclam "${CLAMAV_OUTPUT_DIR}/"
    FOUND=$((FOUND + 1))
fi
if [ -f "bin/clamscan" ]; then
    cp bin/clamscan "${CLAMAV_OUTPUT_DIR}/"
    FOUND=$((FOUND + 1))
fi
if [ -f "bin/freshclam" ]; then
    cp bin/freshclam "${CLAMAV_OUTPUT_DIR}/"
    FOUND=$((FOUND + 1))
fi

if [ $FOUND -lt 2 ]; then
    echo "❌ Failed to find binaries"
    find . -name "clamscan" -o -name "freshclam" 2>/dev/null
    exit 1
fi

chmod +x "${CLAMAV_OUTPUT_DIR}/clamscan"
chmod +x "${CLAMAV_OUTPUT_DIR}/freshclam"

# 验证
echo ""
echo "🔍 Checking binaries..."
ls -lh "${CLAMAV_OUTPUT_DIR}/"

echo ""
echo "📦 Binary dependencies:"
echo "clamscan:"
ldd "${CLAMAV_OUTPUT_DIR}/clamscan" 2>&1 || echo "  ✅ Static or no dynamic linker"
echo ""
echo "freshclam:"
ldd "${CLAMAV_OUTPUT_DIR}/freshclam" 2>&1 || echo "  ✅ Static or no dynamic linker"

echo ""
echo "🏷️  Version check:"
"${CLAMAV_OUTPUT_DIR}/clamscan" --version 2>&1 | head -3

echo ""
echo "======================================"
echo "  ✅ Build complete!"
echo "======================================"
echo ""
