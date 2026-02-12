#!/bin/bash
set -e

# ClamAV 1.5.1 静态编译脚本
# 用法: ./build-clamav-static.sh

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLAMAV_VERSION="1.5.1"
CLAMAV_BUILD_DIR="${PROJECT_DIR}/clamAV/build-static"
CLAMAV_OUTPUT_DIR="${PROJECT_DIR}/app/bin"

echo "======================================"
echo "  Building ClamAV ${CLAMAV_VERSION} (Static)"
echo "======================================"

# 清理旧文件
echo "🧹 Cleaning old binaries..."
rm -f "${CLAMAV_OUTPUT_DIR}/clamscan" "${CLAMAV_OUTPUT_DIR}/freshclam"

# 创建目录
mkdir -p "${CLAMAV_BUILD_DIR}"
mkdir -p "${CLAMAV_OUTPUT_DIR}"

# 检查依赖
echo "🔍 Checking dependencies..."
missing_deps=()

for cmd in gcc g++ cmake make pkg-config; do
    if ! command -v $cmd &> /dev/null; then
        missing_deps+=($cmd)
    fi
done

if [ ${#missing_deps[@]} -gt 0 ]; then
    echo "❌ Missing dependencies: ${missing_deps[*]}"
    echo "   Please install them first:"
    echo "   sudo apt-get install -y build-essential cmake pkg-config"
    exit 1
fi

# 检查开发库
for lib in libssl-dev libcurl4-openssl-dev zlib1g-dev; do
    if ! dpkg -l | grep -q "^ii  $lib"; then
        missing_deps+=($lib)
    fi
done

if [ ${#missing_deps[@]} -gt 0 ]; then
    echo "⚠️  Missing dev libraries: ${missing_deps[*]}"
    echo "   Install them:"
    echo "   sudo apt-get install -y libssl-dev libcurl4-openssl-dev zlib1g-dev"
fi

# 下载 ClamAV 1.5.1 源码
CLAMAV_TARBALL="${CLAMAV_BUILD_DIR}/clamav-${CLAMAV_VERSION}.tar.gz"
if [ ! -f "${CLAMAV_TARBALL}" ]; then
    echo "📥 Downloading ClamAV ${CLAMAV_VERSION}..."
    wget -O "${CLAMAV_TARBALL}" "https://www.clamav.net/downloads/production/clamav-${CLAMAV_VERSION}.tar.gz"
fi

# 解压
CLAMAV_SRC_DIR="${CLAMAV_BUILD_DIR}/clamav-${CLAMAV_VERSION}"
if [ ! -d "${CLAMAV_SRC_DIR}" ]; then
    echo "📦 Extracting..."
    tar -xzf "${CLAMAV_TARBALL}" -C "${CLAMAV_BUILD_DIR}"
fi

cd "${CLAMAV_SRC_DIR}"

# 首先编译 libclamav 静态库
echo "🔨 Building libclamav static library..."
mkdir -p build
cd build

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

# 先编译库
echo "📚 Compiling libraries..."
make -j$(nproc) libclamav

# 编译 clamscan 和 freshclam（静态链接）
echo "🦀 Compiling clamscan (static)..."
make -j$(nproc) clamscan

echo "🦀 Compiling freshclam (static)..."
make -j$(nproc) freshclam

# 复制二进制文件
echo "📋 Copying binaries..."

# 查找二进制文件位置
if [ -f "clamscan/clamscan" ]; then
    CLAMSCAN_BIN="clamscan/clamscan"
elif [ -f "bin/clamscan" ]; then
    CLAMSCAN_BIN="bin/clamscan"
else
    echo "❌ clamscan not found in build output"
    find . -name "clamscan" -type f
    exit 1
fi

if [ -f "freshclam/freshclam" ]; then
    FRESHCLAM_BIN="freshclam/freshclam"
elif [ -f "bin/freshclam" ]; then
    FRESHCLAM_BIN="bin/freshclam"
else
    echo "❌ freshclam not found in build output"
    find . -name "freshclam" -type f
    exit 1
fi

cp "${CLAMSCAN_BIN}" "${CLAMAV_OUTPUT_DIR}/clamscan"
cp "${FRESHCLAM_BIN}" "${CLAMAV_OUTPUT_DIR}/freshclam"

chmod +x "${CLAMAV_OUTPUT_DIR}/clamscan"
chmod +x "${CLAMAV_OUTPUT_DIR}/freshclam"

# 验证是否是动态链接
echo ""
echo "🔍 Checking binary dependencies..."
echo "clamscan:"
ldd "${CLAMAV_OUTPUT_DIR}/clamscan" || echo "  (not a dynamic executable)"
echo ""
echo "freshclam:"
ldd "${CLAMAV_OUTPUT_DIR}/freshclam" || echo "  (not a dynamic executable)"

# 测试版本
echo ""
echo "🏷️  Version check:"
"${CLAMAV_OUTPUT_DIR}/clamscan" --version 2>&1 | head -3 || echo "  (failed to run)"

echo ""
echo "======================================"
echo "  ✅ Build complete!"
echo "======================================"
echo "📦 Binaries: ${CLAMAV_OUTPUT_DIR}/"
echo "   - clamscan"
echo "   - freshclam"
echo ""
echo "⚠️  Note: If binaries still show shared library dependencies,"
echo "   you may need to use musl-gcc for truly static binaries."
echo ""
