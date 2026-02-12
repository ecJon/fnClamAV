#!/bin/bash
# 飞牛 ClamAV 编译依赖安装脚本

echo "=== 飞牛 ClamAV 编译依赖安装 ==="
echo ""
echo "此脚本将安装编译 ClamAV 所需的依赖库"
echo ""

# 更新包列表
sudo apt-get update

# 安装基础工具
sudo apt-get install -y \
    build-essential \
    cmake \
    pkg-config \
    checkinstall \
    libssl-dev \
    libcurl4-openssl-dev

# 安装可选库（ClamAV 压缩支持）
sudo apt-get install -y \
    libpcre2-dev \
    libjson-c-dev \
    zlib1g-dev \
    libcheck-dev \
    libxml2-dev

echo ""
echo "=== 依赖安装完成 ==="
echo "请运行 ./check_deps.sh 验证安装结果"
