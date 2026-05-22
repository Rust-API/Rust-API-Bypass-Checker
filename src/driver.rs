#![allow(dead_code)]



use std::{env, process};

use rustc_session::config::ErrorOutputType;
use rustc_session::EarlyDiagCtxt;

use crate::{analysis, utils};
use crate::analysis::option;

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_FAILURE: i32 = 1;

/// 给 mir_wrapper 调用：传入“形如 rustc 的 argv”
/// 约定：argv[0] 是 rustc 路径或任意占位名，argv[1..] 是 rustc 参数
pub fn run_with_rustc_args(mut rustc_args: Vec<String>) -> i32 {
    // logger 多次 init 会报错，用 try_init 即可
    let _ = pretty_env_logger::try_init();

    let result = rustc_driver::catch_fatal_errors(move || {
        // 补 sysroot
        if let Some(sysroot) = utils::compile_time_sysroot() {
            let sysroot_flag = "--sysroot";
            if !rustc_args.iter().any(|e| e == sysroot_flag) {
                rustc_args.push(sysroot_flag.to_owned());
                rustc_args.push(sysroot);
            }
        }

        // 依赖 crate / 纯编译模式：行为与 rustc 等价
        if env::var_os("BYPASSER_BE_RUSTC").is_some() || env::var_os("MIR_CHECKER_BE_RUSTC").is_some() {
            let early_dcx = EarlyDiagCtxt::new(ErrorOutputType::default());
            rustc_driver::init_rustc_env_logger(&early_dcx);

            let mut callbacks = rustc_driver::TimePassesCallbacks::default();
            let run_compiler = rustc_driver::RunCompiler::new(&rustc_args, &mut callbacks);
            return run_compiler.run();
        }

        // 分析模式：加必要 flag
        // Ensure MIR is always available for downstream analysis callbacks.
        let always_encode_mir = "-Zalways_encode_mir";
        if !rustc_args.iter().any(|e| e == always_encode_mir) {
            rustc_args.push(always_encode_mir.to_owned());
        }

        // 简化 CFG
        if !rustc_args.iter().any(|e| e == "-Cpanic=abort") {
            rustc_args.push("-Cpanic=abort".to_owned());
        }

        // 解析你自定义的分析参数（会从 rustc_args 中移除 --entry/--domain 等）
        let analysis_options = option::AnalysisOption::from_args(&mut rustc_args);
        log::info!("Analysis Option: {:?}", analysis_options);

        let mut callbacks = analysis::callback::BypasserCallbacks::new(analysis_options);
        let run_compiler = rustc_driver::RunCompiler::new(&rustc_args, &mut callbacks);
        run_compiler.run()
    });

    let exit_code = match result {
        Ok(_) => EXIT_SUCCESS,
        Err(_) => EXIT_FAILURE,
    };
    process::exit(exit_code);
}

/// 给 bin/api-bypass.rs 调用：从当前进程 env::args 读取
pub fn run_from_env_args() -> i32 {
    let rustc_args = env::args_os()
        .enumerate()
        .map(|(i, arg)| {
            arg.into_string().unwrap_or_else(|arg| {
                panic!("Argument {} is not valid Unicode: {:?}", i, arg);
            })
        })
        .collect::<Vec<_>>();

    run_with_rustc_args(rustc_args)
}

/// 可选：兼容旧写法
pub fn main_like() -> ! {
    let code = run_from_env_args();
    process::exit(code)
}
