// ClamAV FFI 模块
//
// 此模块提供 ClamAV 的 Rust FFI 绑定，包括：
// - 引擎初始化和生命周期管理
// - 文件扫描功能
// - 引擎状态管理

pub mod ffi;
pub mod manager;
pub mod engine;
pub mod types;

pub use ffi::*;
pub use manager::*;
pub use engine::*;
pub use types::*;
