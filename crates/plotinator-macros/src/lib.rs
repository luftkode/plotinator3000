/// Add puffin profiling to the function if the profiling feature is enabled and the target is not wasm32
#[macro_export]
macro_rules! profile_function {
    () => {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();
    };
}
