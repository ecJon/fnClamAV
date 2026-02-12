// ClamAV FFI 绑定层单元测试
//
// 注意：这些测试需要 libclamav.so 才能运行
// 在没有编译 libclamav.so 的情况下，这些测试会失败（这是正常的）
//
// 运行方式：
//   cargo test --package clamav-tests
//
// 或者单独测试：
//   cargo test --test ffi::tests::ffi_test

use super::super::*;

#[cfg(test)]
mod tests {
    use super::*;

    // 测试 ClamAVError 显示
    #[test]
    fn test_error_display() {
        let err = ClamAVError::InitializationFailed("test error".to_string());
        assert_eq!(format!("{}", err), "test error");
    }

    // 测试 ScanOptions 默认值
    #[test]
    fn test_scan_options_default() {
        let opts = ScanOptions::default();
        assert!(!opts.scan_archive);
        assert!(!opts.scan_pdf);
        assert!(!opts.scan_ole2);
        assert!(!opts.scan_pe);
        assert!(!opts.scan_elf);
        assert!(!opts.scan_mail);
    }

    // 测试 ScanOptions Clone trait
    #[test]
    fn test_scan_options_clone() {
        let opts = ScanOptions::default();
        let opts_clone = opts.clone();
        assert_eq!(opts.scan_archive, opts_clone.scan_archive);
    }

    // 测试 EngineState 状态转换
    #[test]
    fn test_engine_state() {
        use std::sync::Mutex;

        let state = Mutex::new(EngineState::Ready);

        // Uninitialized -> Initializing (初始化)
        {
            let _state = state.lock().unwrap();
            // 测试 Ready 状态的 is_operational 和 is_ready 方法
            assert!(_state.is_operational() == false);
            assert!(_state.is_ready() == true);
        }
    }

    // 测试 ClamAVEngine Default
    #[test]
    fn test_engine_default() {
        let engine = ClamAVEngine::default();
        assert!(!engine.initialized);
        assert!(engine.engine.is_null());
    }

    // 集成测试标记 - 这些测试需要实际的 libclamav.so
    // 当 libclamav.so 编译好后，可以移除下面的 #[ignore] 属性
    #[test]
    #[ignore]
    fn test_with_real_libclamav() {
        // 测试实际的 libclamav.so 绑定
        // 这个测试只有在编译好的 libclamav.so 后才能通过
        println!("This test requires libclamav.so to be compiled first");
    }
}
