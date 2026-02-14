// ClamAV FFI 绑定层
// 适配 ClamAV 1.5.1 API

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uint, c_int, c_void};
use std::ptr;

// ============ ClamAV C API 类型绑定 ============

/// ClamAV 错误码类型
pub type cl_error_t = c_int;

/// ClamAV 引擎字段常量（直接使用整数值，与 ClamAV 1.5.1 C API 对齐）
pub const CL_ENGINE_CVDCERTSDIR: c_int = 33;

/// ClamAV 引擎结构体 (opaque pointer)
#[repr(C)]
pub struct cl_engine {
    _private: [u8; 0],
}

/// 扫描结果枚举
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum cl_verdict_t {
    CL_VERDICT_NOTHING_FOUND = 0,
    CL_VERDICT_TRUSTED = 1,
    CL_VERDICT_STRONG_INDICATOR = 2,
    CL_VERDICT_POTENTIALLY_UNWANTED = 3,
}

/// 扫描选项结构体
/// 与 ClamAV 1.5.1 C API 完全对齐（5个字段）
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct cl_scan_options {
    pub general: u32,
    pub parse: u32,
    pub heuristic: u32,
    pub mail: u32,
    pub dev: u32,
}

/// ClamAV 引擎字段枚举
/// 与 ClamAV 1.5.1 C API 完全对齐
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum cl_engine_field {
    CL_ENGINE_MAX_SCANSIZE = 0,
    CL_ENGINE_MAX_FILESIZE,
    CL_ENGINE_MAX_RECURSION,
    CL_ENGINE_MAX_FILES,
    CL_ENGINE_MIN_CC_COUNT,
    CL_ENGINE_MIN_SSN_COUNT,
    CL_ENGINE_PUA_CATEGORIES,
    CL_ENGINE_DB_OPTIONS,
    CL_ENGINE_DB_VERSION,
    CL_ENGINE_DB_TIME,
    CL_ENGINE_AC_ONLY,
    CL_ENGINE_AC_MINDEPTH,
    CL_ENGINE_AC_MAXDEPTH,
    CL_ENGINE_TMPDIR,
    CL_ENGINE_KEEPTMP,
    CL_ENGINE_BYTECODE_SECURITY,
    CL_ENGINE_BYTECODE_TIMEOUT,
    CL_ENGINE_BYTECODE_MODE,
    CL_ENGINE_MAX_EMBEDDEDPE,
    CL_ENGINE_MAX_HTMLNORMALIZE,
    CL_ENGINE_MAX_HTMLNOTAGS,
    CL_ENGINE_MAX_SCRIPTNORMALIZE,
    CL_ENGINE_MAX_ZIPTYPERCG,
    CL_ENGINE_FORCETODISK,
    CL_ENGINE_CACHE_SIZE,
    CL_ENGINE_DISABLE_CACHE,
    CL_ENGINE_DISABLE_PE_STATS,
    CL_ENGINE_STATS_TIMEOUT,
    CL_ENGINE_MAX_PARTITIONS,
    CL_ENGINE_MAX_ICONSPE,
    CL_ENGINE_MAX_RECHWP3,
    CL_ENGINE_MAX_SCANTIME,
    CL_ENGINE_PCRE_MATCH_LIMIT,
    CL_ENGINE_PCRE_RECMATCH_LIMIT,
    CL_ENGINE_PCRE_MAX_FILESIZE,
    CL_ENGINE_DISABLE_PE_CERTS,
    CL_ENGINE_PE_DUMPCERTS,
    CL_ENGINE_CVDCERTSDIR,
}

// 扫描选项常量 - general 字段
pub const CL_SCAN_GENERAL_ALLMATCHES: u32 = 0x1;
pub const CL_SCAN_GENERAL_COLLECT_METADATA: u32 = 0x2;
pub const CL_SCAN_GENERAL_HEURISTICS: u32 = 0x4;

// 扫描选项常量 - parse 字段
pub const CL_SCAN_PARSE_ARCHIVE: u32 = 0x1;
pub const CL_SCAN_PARSE_ELF: u32 = 0x2;
pub const CL_SCAN_PARSE_PDF: u32 = 0x4;
pub const CL_SCAN_PARSE_SWF: u32 = 0x8;
pub const CL_SCAN_PARSE_HWP: u32 = 0x10;
pub const CL_SCAN_PARSE_XMLDOCS: u32 = 0x20;
pub const CL_SCAN_PARSE_MAIL: u32 = 0x40;
pub const CL_SCAN_PARSE_OLE2: u32 = 0x80;
pub const CL_SCAN_PARSE_HTML: u32 = 0x100;
pub const CL_SCAN_PARSE_PE: u32 = 0x200;

// 默认解析选项：启用所有解析器
pub const CL_SCAN_PARSE_DEFAULT: u32 = CL_SCAN_PARSE_ARCHIVE
    | CL_SCAN_PARSE_ELF
    | CL_SCAN_PARSE_PDF
    | CL_SCAN_PARSE_SWF
    | CL_SCAN_PARSE_HWP
    | CL_SCAN_PARSE_XMLDOCS
    | CL_SCAN_PARSE_MAIL
    | CL_SCAN_PARSE_OLE2
    | CL_SCAN_PARSE_HTML
    | CL_SCAN_PARSE_PE;

// 错误码常量
pub const CL_CLEAN: cl_error_t = 0;
pub const CL_SUCCESS: cl_error_t = 0;
pub const CL_VIRUS: cl_error_t = 1;

// 数据库选项常量（与 ClamAV 1.5.1 C API 对齐）
pub const CL_DB_PHISHING: u32 = 0x2;
pub const CL_DB_PHISHING_URLS: u32 = 0x8;
pub const CL_DB_PUA: u32 = 0x10;
pub const CL_DB_BYTECODE: u32 = 0x2000;
pub const CL_DB_STDOPT: u32 = CL_DB_PHISHING | CL_DB_PHISHING_URLS | CL_DB_BYTECODE;

// ============ FFI 函数声明 ============

extern "C" {
    /// 初始化 ClamAV 库
    fn cl_init(initoptions: c_uint) -> cl_error_t;

    /// 创建新的扫描引擎
    fn cl_engine_new() -> *mut cl_engine;

    /// 设置引擎字符串选项
    fn cl_engine_set_str(
        engine: *mut cl_engine,
        field: cl_engine_field,
        str: *const c_char,
    ) -> cl_error_t;

    /// 编译扫描引擎
    fn cl_engine_compile(engine: *mut cl_engine) -> cl_error_t;

    /// 释放扫描引擎
    fn cl_engine_free(engine: *mut cl_engine) -> cl_error_t;

    /// 加载病毒数据库
    fn cl_load(
        path: *const c_char,
        engine: *mut cl_engine,
        signo: *mut c_uint,
        dboptions: c_uint,
    ) -> cl_error_t;

    /// 扫描文件（扩展版本）
    fn cl_scanfile_ex(
        filename: *const c_char,
        verdict_out: *mut cl_verdict_t,
        last_alert_out: *mut *const c_char,
        scanned_out: *mut u64,
        engine: *const cl_engine,
        scanoptions: *const cl_scan_options,
        context: *mut c_void,
        hash_hint: *const c_char,
        hash_out: *mut *mut c_char,
        hash_alg: *const c_char,
        file_type_hint: *const c_char,
        file_type_out: *mut *mut c_char,
    ) -> cl_error_t;
}

// ============ Rust 封装结构体 ============

/// ClamAV 引擎实例
pub struct ClamAVEngine {
    engine: *mut cl_engine,
    initialized: bool,
}

/// ClamAV 扫描选项
#[derive(Debug, Clone, Copy)]
pub struct ScanOptions {
    pub scan_archive: bool,
    pub scan_pdf: bool,
    pub scan_elf: bool,
    pub scan_mail: bool,
    pub heuristics: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            scan_archive: true,
            scan_pdf: true,
            scan_elf: true,
            scan_mail: true,
            heuristics: true,
        }
    }
}

/// 扫描结果
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub filename: String,
    pub virus_name: Option<String>,
    pub is_infected: bool,
}

/// ClamAV 错误类型
#[derive(Debug, Clone)]
pub enum ClamAVError {
    InitializationFailed(String),
    EngineCreationFailed(String),
    DatabaseLoadFailed(String),
    EngineCompilationFailed(String),
    ScanFailed(String),
    InvalidPath(String),
}

impl std::fmt::Display for ClamAVError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClamAVError::InitializationFailed(msg) => write!(f, "Initialization failed: {}", msg),
            ClamAVError::EngineCreationFailed(msg) => write!(f, "Engine creation failed: {}", msg),
            ClamAVError::DatabaseLoadFailed(msg) => write!(f, "Database load failed: {}", msg),
            ClamAVError::EngineCompilationFailed(msg) => write!(f, "Engine compilation failed: {}", msg),
            ClamAVError::ScanFailed(msg) => write!(f, "Scan failed: {}", msg),
            ClamAVError::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
        }
    }
}

impl std::error::Error for ClamAVError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

// ============ ClamAVEngine 实现 ============

impl ClamAVEngine {
    /// 初始化 ClamAV 引擎
    ///
    /// # 参数
    /// - db_dir: 病毒库目录路径
    /// - certs_dir: 证书目录路径（可选）
    pub fn initialize(db_dir: &str, certs_dir: Option<&str>) -> Result<Self, ClamAVError> {
        unsafe {
            // 初始化 ClamAV 库
            let ret = cl_init(0);
            if ret != CL_SUCCESS && ret != CL_CLEAN {
                return Err(ClamAVError::InitializationFailed(
                    format!("cl_init failed with code: {}", ret)
                ));
            }

            // 创建新引擎
            let engine = cl_engine_new();
            if engine.is_null() {
                return Err(ClamAVError::EngineCreationFailed(
                    "cl_engine_new returned null".to_string()
                ));
            }

            // 设置证书目录（如果提供）
            if let Some(certs) = certs_dir {
                tracing::info!("Setting ClamAV certificates directory: {}", certs);

                // 检查证书目录是否存在
                if !std::path::Path::new(certs).exists() {
                    tracing::warn!("Certificate directory does not exist: {}, continuing anyway", certs);
                }

                let certs_cstr = CString::new(certs).unwrap_or_else(|_| {
                    CString::new("<invalid>").unwrap()
                });
                tracing::debug!("Calling cl_engine_set_str with CL_ENGINE_CVDCERTSDIR (value: {})", cl_engine_field::CL_ENGINE_CVDCERTSDIR as i32);
                let ret = cl_engine_set_str(engine, cl_engine_field::CL_ENGINE_CVDCERTSDIR, certs_cstr.as_ptr());
                if ret != CL_SUCCESS {
                    tracing::error!("cl_engine_set_str failed with error code: {}", ret);
                    cl_engine_free(engine);
                    return Err(ClamAVError::InitializationFailed(
                        format!("Failed to set certs directory '{}': error code {}", certs, ret)
                    ));
                }
                tracing::info!("Certificate directory set successfully");
            } else {
                tracing::info!("No certificate directory specified, using ClamAV defaults");
            }

            // 加载病毒数据库
            let db_dir_cstr = CString::new(db_dir).unwrap_or_else(|_| {
                CString::new("<invalid>").unwrap()
            });
            let mut signo: c_uint = 0;
            tracing::info!("Loading virus database from: {}", db_dir);
            let ret = cl_load(
                db_dir_cstr.as_ptr(),
                engine,
                &mut signo,
                CL_DB_STDOPT as c_uint,
            );

            if ret != CL_SUCCESS {
                cl_engine_free(engine);
                return Err(ClamAVError::DatabaseLoadFailed(
                    format!("cl_load failed with code: {}", ret)
                ));
            }
            tracing::info!("Loaded {} signatures from database", signo);

            // 编译引擎
            tracing::info!("Compiling ClamAV engine...");
            let ret = cl_engine_compile(engine);
            if ret != CL_SUCCESS {
                cl_engine_free(engine);
                return Err(ClamAVError::EngineCompilationFailed(
                    format!("cl_engine_compile failed with code: {}", ret)
                ));
            }
            tracing::info!("ClamAV engine compiled successfully");

            Ok(Self {
                engine,
                initialized: true,
            })
        }
    }

    /// 扫描单个文件
    ///
    /// # 参数
    /// - path: 文件路径
    /// - options: 扫描选项
    pub fn scan_file(&self, path: &str, options: ScanOptions) -> Result<ScanResult, ClamAVError> {
        if !self.initialized {
            return Err(ClamAVError::ScanFailed("Engine not initialized".to_string()));
        }

        unsafe {
            let path_cstr = CString::new(path).unwrap_or_else(|_| {
                CString::new("<invalid>").unwrap()
            });

            // 构建扫描选项 - 正确初始化所有 5 个字段
            // ClamAV 推荐将 parse 设置为 !0 以启用所有解析器
            let mut scan_opts = cl_scan_options {
                general: 0,
                parse: !0,  // 启用所有解析器（ClamAV 推荐方式）
                heuristic: 0,
                mail: 0,
                dev: 0,
            };

            if options.heuristics {
                scan_opts.general |= CL_SCAN_GENERAL_HEURISTICS;
            }

            // 启用 all-match 模式以确保检测所有威胁
            scan_opts.general |= CL_SCAN_GENERAL_ALLMATCHES;

            let mut verdict: cl_verdict_t = cl_verdict_t::CL_VERDICT_NOTHING_FOUND;
            let mut last_alert: *const c_char = ptr::null();
            let mut scanned: u64 = 0;

            tracing::debug!("Calling cl_scanfile_ex for: {}", path);

            let ret = cl_scanfile_ex(
                path_cstr.as_ptr(),
                &mut verdict,
                &mut last_alert,
                &mut scanned,
                self.engine,
                &scan_opts,
                ptr::null_mut(),  // context
                ptr::null(),      // hash_hint
                ptr::null_mut(),  // hash_out
                ptr::null(),      // hash_alg
                ptr::null(),      // file_type_hint
                ptr::null_mut(),  // file_type_out
            );

            tracing::debug!("cl_scanfile_ex returned: {}, verdict: {:?} (raw value: {})", ret, verdict, verdict as i32);

            // 重要：cl_scanfile_ex 不会返回 CL_VIRUS，需要检查 verdict_out 参数！
            // CL_VERDICT_STRONG_INDICATOR = 2 表示检测到病毒/恶意软件
            // CL_VERDICT_POTENTIALLY_UNWANTED = 3 表示检测到潜在不受欢迎程序

            // 使用整数值进行比较，避免 enum 匹配问题
            let verdict_value = verdict as i32;
            tracing::info!("VERDICT CHECK: value={}, STRONG_INDICATOR=2, POTENTIALLY_UNWANTED=3", verdict_value);

            if verdict_value == 2 || verdict_value == 3 {
                // 病毒/恶意软件检测到
                tracing::warn!("VIRUS DETECTED! verdict={}", verdict_value);
                let virus = if !last_alert.is_null() {
                    let virus_name = CStr::from_ptr(last_alert).to_string_lossy().to_string();
                    tracing::warn!("VIRUS FOUND in {}: {}", path, virus_name);
                    Some(virus_name)
                } else {
                    tracing::warn!("VIRUS FOUND in {}: Unknown", path);
                    Some("Unknown".to_string())
                };

                Ok(ScanResult {
                    filename: path.to_string(),
                    virus_name: virus,
                    is_infected: true,
                })
            } else if verdict_value == 1 {
                // 受信任文件
                tracing::debug!("File trusted: {}", path);
                Ok(ScanResult {
                    filename: path.to_string(),
                    virus_name: None,
                    is_infected: false,
                })
            } else {
                // 没有发现威胁 (verdict_value == 0) 或其他情况
                // 检查返回码是否有错误
                if ret != CL_CLEAN && ret != CL_SUCCESS {
                    tracing::error!("cl_scanfile_ex failed for {}: error code {}", path, ret);
                    Err(ClamAVError::ScanFailed(
                        format!("cl_scanfile_ex failed with code: {}", ret)
                    ))
                } else {
                    tracing::debug!("File clean: {}", path);
                    Ok(ScanResult {
                        filename: path.to_string(),
                        virus_name: None,
                        is_infected: false,
                    })
                }
            }
        }
    }
}

// 实现 Drop trait 确保引擎资源被正确释放
impl Drop for ClamAVEngine {
    fn drop(&mut self) {
        if self.initialized && !self.engine.is_null() {
            unsafe {
                let _ = cl_engine_free(self.engine);
            }
        }
    }
}
