#!/bin/bash
#
# ClamAV 杀毒软件 CGI 脚本
# 功能：转发 API 请求到 Rust 守护进程，服务静态文件
#

# Rust 守护进程地址
DAEMON_HOST="127.0.0.1"
DAEMON_PORT="8899"
DAEMON_URL="http://${DAEMON_HOST}:${DAEMON_PORT}"

# 静态文件目录（优先使用环境变量，生产环境）
if [ -n "$TRIM_APPDEST" ]; then
    STATIC_DIR="${TRIM_APPDEST}/www"
else
    # 开发环境回退路径
    SCRIPT_DIR="$(dirname "$0")"
    STATIC_DIR="$(cd "$SCRIPT_DIR/../www" && pwd)"
fi

# 获取请求信息
REQUEST_METHOD="${REQUEST_METHOD:-GET}"
PATH_INFO="${PATH_INFO:-}"

# 日志函数
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" >&2
}

# 从完整路径中提取相对路径
extract_relative_path() {
    local full_path="$1"
    local script_name="${SCRIPT_NAME:-}"

    # 如果 PATH_INFO 包含 CGI 脚本路径，需要提取后面的部分
    if [[ "$full_path" == *"/index.cgi"* ]]; then
        # 移除 /index.cgi 及之前的部分
        local result="${full_path#*/index.cgi}"
        echo "$result"
    else
        echo "$full_path"
    fi
}

# 转发请求到 Rust 守护进程
forward_request() {
    local endpoint="$1"
    local method="${2:-GET}"

    # 输出 JSON 响应头
    echo "Content-Type: application/json"
    echo ""

    # 读取请求体（如果是 POST/PUT）
    local request_body=""
    if [[ "$method" = "POST" || "$method" = "PUT" ]]; then
        request_body=$(cat)
    fi

    # 构建完整 URL
    local url="${DAEMON_URL}/api/${endpoint}"

    # 转发请求（设置 30 秒超时，连接超时 5 秒）
    local response
    if [[ -n "$request_body" ]]; then
        response=$(curl -s -m 30 --connect-timeout 5 -X "$method" \
            -H "Content-Type: application/json" \
            -d "$request_body" \
            "$url" 2>/dev/null)
    else
        response=$(curl -s -m 30 --connect-timeout 5 -X "$method" "$url" 2>/dev/null)
    fi

    # 输出响应
    if [[ -n "$response" ]]; then
        echo "$response"
    else
        echo '{"success":false,"error":"Failed to connect to daemon"}'
    fi
}

# 服务静态文件
serve_static_file() {
    local file_path="$1"

    # 安全检查：防止目录遍历
    if [[ "$file_path" == *".."* ]]; then
        echo "Status: 400 Bad Request"
        echo "Content-Type: text/plain; charset=UTF-8"
        echo ""
        echo "Bad Request"
        return
    fi

    # 检查文件是否存在
    if [[ -f "$file_path" ]]; then
        # 根据文件扩展名设置 Content-Type
        local ext="${file_path##*.}"
        local content_type="text/plain"

        case "$ext" in
            html) content_type="text/html; charset=UTF-8" ;;
            htm)  content_type="text/html; charset=UTF-8" ;;
            css)  content_type="text/css; charset=UTF-8" ;;
            js)   content_type="application/javascript; charset=UTF-8" ;;
            json) content_type="application/json; charset=UTF-8" ;;
            png)  content_type="image/png" ;;
            jpg)  content_type="image/jpeg" ;;
            jpeg) content_type="image/jpeg" ;;
            gif)  content_type="image/gif" ;;
            svg)  content_type="image/svg+xml" ;;
            ico)  content_type="image/x-icon" ;;
            txt)  content_type="text/plain; charset=UTF-8" ;;
            xml)  content_type="text/xml; charset=UTF-8" ;;
        esac

        # 输出 HTTP 头
        echo "Content-Type: $content_type"
        echo ""

        # 输出文件内容
        cat "$file_path"
    else
        # 文件不存在，返回 404
        echo "Status: 404 Not Found"
        echo "Content-Type: text/plain; charset=UTF-8"
        echo ""
        echo "404 Not Found: $file_path"
    fi
}

# 主路由逻辑
main() {
    # 提取相对路径
    local relative_path="$(extract_relative_path "$PATH_INFO")"

    log "Request: $REQUEST_METHOD $PATH_INFO"
    log "Relative path: $relative_path"
    log "Static dir: $STATIC_DIR"

    # 如果是 API 请求，转发到 Rust 守护进程
    if [[ "$relative_path" == /api/* ]]; then
        # 移除 /api/ 前缀
        local endpoint="${relative_path#/api/}"
        # 移除开头的斜杠
        endpoint="${endpoint#/}"

        log "Forwarding to daemon: /api/$endpoint"
        forward_request "$endpoint" "$REQUEST_METHOD"
    else
        # 服务静态文件
        local file_path="$STATIC_DIR${relative_path}"

        # 移除末尾的斜杠（如果是目录）
        file_path="${file_path%/}"

        # 如果请求的是目录，尝试加载 index.html
        if [[ -d "$file_path" ]]; then
            file_path="$file_path/index.html"
        fi

        # 如果 PATH_INFO 是 / 或者空，返回 index.html
        if [[ -z "$relative_path" || "$relative_path" = "/" ]]; then
            file_path="$STATIC_DIR/index.html"
        fi

        log "Serving static file: $file_path"
        serve_static_file "$file_path"
    fi
}

# 执行主逻辑
main "$@"
