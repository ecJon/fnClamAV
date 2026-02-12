// rust-server/src/lib.rs
//
// 模块声明，导出所有子模块

pub mod clamav;
pub mod env;
pub mod models;
pub mod services;
pub mod handlers;

// 重新导出常用类型和常量
pub use env::FnosEnv;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
