pub mod casting;
pub mod concurrency;
pub mod config;

pub(crate) fn init_os() {
    #[cfg(target_os = "windows")]
    unsafe {
        windows::Win32::Media::timeBeginPeriod(1);
    }

    #[cfg(target_arch = "wasm32")]
    {
        use ion_common::wasm_bindgen;
        use ion_common::wasm_bindgen::prelude::*;

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = console)]
            fn error(msg: String);

            type Error;

            #[wasm_bindgen(constructor)]
            fn new() -> Error;

            #[wasm_bindgen(structural, method, getter)]
            fn stack(error: &Error) -> String;
        }

        fn hook(info: &std::panic::PanicHookInfo) {
            let mut msg = info.to_string();
            msg.push_str("\n\nStack:\n\n");
            let e = Error::new();
            let stack = e.stack();
            msg.push_str(&stack);
            msg.push_str("\n\n");
            error(msg);
        }

        std::panic::set_hook(Box::new(hook));
    }
}

pub(crate) fn uninit_os() {
    #[cfg(target_os = "windows")]
    unsafe {
        windows::Win32::Media::timeEndPeriod(1);
    }
}
