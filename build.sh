#!/bin/bash
set -e

# ========================================
# ClamAV 杀毒软件 - 统一构建打包脚本
# ========================================
# 用法: ./build.sh [--clean] [--skip-clamav]
#
# 选项:
#   --clean         清理所有构建缓存和产物
#   --skip-clamav   跳过 ClamAV 动态库构建（假设已存在）
#
# ========================================

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${PROJECT_DIR}/dist"
BUILD_TEMP="/tmp/fpk_build_$$"
CLAMAV_BUILD_DIR="${PROJECT_DIR}/clamAV/build"

# 解析参数
CLEAN_BUILD=false
SKIP_CLAMAV=false

for arg in "$@"; do
    case $arg in
        --clean)
            CLEAN_BUILD=true
            shift
            ;;
        --skip-clamav)
            SKIP_CLAMAV=true
            shift
            ;;
    esac
done

# ========================================
# 1. 清理缓存
# ========================================
if [ "$CLEAN_BUILD" = true ]; then
    echo "======================================"
    echo "  🧹 Cleaning build cache..."
    echo "======================================"
    echo ""

    # 清理 Rust 构建缓存
    echo "🦀 Cleaning Rust build cache..."
    cd "${PROJECT_DIR}/rust-server"
    cargo clean 2>/dev/null || true

    # 清理 ClamAV 构建目录
    echo "📦 Cleaning ClamAV build directory..."
    rm -rf "${CLAMAV_BUILD_DIR}"

    # 清理输出目录
    echo "🗑️  Cleaning output directory..."
    rm -rf "${OUTPUT_DIR}"

    # 清理旧的二进制文件
    echo "🗑️  Cleaning old binaries..."
    rm -f "${PROJECT_DIR}/app/server/clamav-daemon"
    rm -rf "${PROJECT_DIR}/app/lib"
    rm -rf "${PROJECT_DIR}/app/bin"

    echo "✅ Cache cleaned!"
    echo ""
fi

# ========================================
# 2. 检查构建依赖
# ========================================
echo "======================================"
echo "  🔍 Checking build dependencies..."
echo "======================================"
echo ""

# 检查基础工具
check_command() {
    if ! command -v "$1" &> /dev/null; then
        echo "❌ $1 not found!"
        echo "   Please install: sudo apt-get install $2"
        exit 1
    else
        echo "✅ $1 found"
    fi
}

check_command cargo "rustc cargo"
check_command gcc "build-essential"
check_command cmake "cmake"
check_command pkg-config "pkg-config"

echo ""

# ========================================
# 3. 构建 ClamAV 动态库
# ========================================
if [ "$SKIP_CLAMAV" = false ]; then
    echo "======================================"
    echo "  📦 Building ClamAV shared library..."
    echo "======================================"
    echo ""

    # 检查是否已构建
    CLAMAV_LIB_DIR="${PROJECT_DIR}/app/lib"
    if [ -f "${CLAMAV_LIB_DIR}/libclamav.so" ]; then
        echo "ℹ️  libclamav.so already exists, skipping ClamAV build."
        echo "   To rebuild, run: ./build.sh --clean"
    else
        echo "Building ClamAV shared library..."

        # 创建构建目录
        mkdir -p "${CLAMAV_BUILD_DIR}"
        cd "${CLAMAV_BUILD_DIR}"

        # 配置 CMake
        cmake "${PROJECT_DIR}/clamAV" \
            -DCMAKE_BUILD_TYPE=Release \
            -DCMAKE_INSTALL_PREFIX="${PROJECT_DIR}/app" \
            -DBUILD_SHARED_LIBS=ON \
            -DENABLE_STATIC_LIB=OFF \
            -DDISABLE_MILTER=ON \
            -DDISABLE_CLAMSCAN=ON \
            -DDISABLE_CLAMD=ON \
            -DDISABLE_FRESHCLAM=OFF \
            -DDISABLE_CLAMONACC=ON \
            -DDISABLE_CLAMAV_SUBMIT=ON \
            -DDISABLE_UNRAR=ON

        # 编译并安装
        make -j$(nproc)
        make install

        echo "✅ ClamAV shared library built successfully!"
    fi
    echo ""
else
    echo "ℹ️  Skipping ClamAV build (--skip-clamav flag)"
    echo ""
fi

# ========================================
# 4. 验证 ClamAV 组件
# ========================================
echo "======================================"
echo "  ✅ Verifying ClamAV components..."
echo "======================================"
echo ""

# 检查 libclamav.so
CLAMAV_LIB_DIR="${PROJECT_DIR}/app/lib"
if [ -d "${CLAMAV_LIB_DIR}" ] && [ -n "$(ls -A ${CLAMAV_LIB_DIR}/*.so 2>/dev/null)" ]; then
    echo "✅ libclamav.so found:"
    ls -lh "${CLAMAV_LIB_DIR}/"libclamav.so* 2>/dev/null | head -1
else
    echo "❌ libclamav.so NOT found in ${CLAMAV_LIB_DIR}/"
    echo ""
    echo "To build ClamAV shared library:"
    echo "  ./build.sh"
    exit 1
fi

# 检查 freshclam
CLAMAV_BIN_DIR="${PROJECT_DIR}/app/bin"
if [ -f "${CLAMAV_BIN_DIR}/freshclam" ]; then
    echo "✅ freshclam found"
else
    echo "❌ freshclam NOT found in ${CLAMAV_BIN_DIR}/"
    echo ""
    echo "freshclam is required for virus database updates."
    exit 1
fi

# 检查病毒库
CLAMAV_DB_DIR="${PROJECT_DIR}/app/share/clamav"
if [ -d "${CLAMAV_DB_DIR}" ]; then
    DB_COUNT=$(ls -1 "${CLAMAV_DB_DIR}"/*.cvd "${CLAMAV_DB_DIR}"/*.cld 2>/dev/null | wc -l)
    if [ "$DB_COUNT" -gt 0 ]; then
        DB_SIZE=$(du -sh "${CLAMAV_DB_DIR}" 2>/dev/null | cut -f1)
        echo "✅ Virus database found (${DB_COUNT} files, ${DB_SIZE})"
    else
        echo "⚠️  Virus database directory exists but no .cvd/.cld files found"
    fi
else
    echo "⚠️  Virus database directory not found: ${CLAMAV_DB_DIR}"
    echo "   (Will be created on first run)"
fi

echo ""

# ========================================
# 5. 编译 Rust 守护进程
# ========================================
echo "======================================"
echo "  🦀 Building Rust daemon..."
echo "======================================"
echo ""

cd "${PROJECT_DIR}/rust-server"
cargo build --release

mkdir -p "${PROJECT_DIR}/app/server"
cp target/release/clamav-daemon "${PROJECT_DIR}/app/server/"
chmod +x "${PROJECT_DIR}/app/server/clamav-daemon"

echo "✅ clamav-daemon built successfully"
echo ""

# ========================================
# 6. 创建 .fpk 包
# ========================================
echo "======================================"
echo "  📦 Creating .fpk package..."
echo "======================================"
echo ""

# 清理并创建输出目录
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
# wizard 目录可选（如果存在且非空）
if [ -d "${PROJECT_DIR}/wizard" ] && [ "$(ls -A ${PROJECT_DIR}/wizard 2>/dev/null)" ]; then
    cp -r "${PROJECT_DIR}/wizard" "${BUILD_TEMP}/"
fi

# 创建 app.tgz
echo "📦 Creating app.tgz..."
cd "${PROJECT_DIR}/app"
tar -czf "${BUILD_TEMP}/app.tgz" .

APP_TGZ_SIZE=$(du -h "${BUILD_TEMP}/app.tgz" | cut -f1)
echo "✅ app.tgz created (${APP_TGZ_SIZE})"

# 创建最终的 .fpk 包
echo "🎁 Creating .fpk package..."
cd "${BUILD_TEMP}"
FPK_NAME="fnnas.clamav.fpk"
tar -czf "${OUTPUT_DIR}/${FPK_NAME}" .

FPK_SIZE=$(du -h "${OUTPUT_DIR}/${FPK_NAME}" | cut -f1)

# 清理临时目录
rm -rf "${BUILD_TEMP}"

# ========================================
# 7. 完成
# ========================================
echo ""
echo "======================================"
echo "  ✅ Build complete!"
echo "======================================"
echo ""
echo "📦 Package: ${OUTPUT_DIR}/${FPK_NAME}"
echo "📊 Size: ${FPK_SIZE}"
echo ""
echo "🚀 Ready to install on fnOS!"
echo ""
echo "Install on fnOS:"
echo "  1. Upload ${FPK_NAME} to fnOS"
echo "  2. Install via App Store"
echo ""
