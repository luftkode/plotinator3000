/// Conditionally includes modules only for non-WebAssembly targets.
///
/// This macro simplifies the process of conditionally compiling modules for non-WebAssembly
/// platforms. Each module declaration provided to this macro will be automatically prefixed
/// with `#[cfg(not(target_arch = "wasm32"))]`, excluding it from WebAssembly builds.
///
/// # Examples
///
/// ```ignore
/// // Declare multiple modules that will only be included in non-WebAssembly builds
/// non_wasm_modules!(
///     // Public module
///     pub mod mqtt_client;
///
///     // Module with crate-level visibility
///     pub(crate) mod topic_handler;
///
///     // Private module
///     mod internal_helpers;
///
///     // Module with attributes
///     #[allow(dead_code)]
///     pub mod config;
/// );
/// ```
///
/// # Features
///
/// - Supports multiple module declarations in a single invocation
/// - Preserves module visibility modifiers (pub, pub(crate), etc.)
/// - Preserves additional attributes on module declarations
/// - Maintains the same module directory structure as standard module declarations
///
/// # Notes
///
/// - This macro can be exported and used across crates with `#[macro_export]`
/// - To use this macro in other crates, include the crate that defines it and
///   import the macro with `use crate_name::non_wasm_modules;`
/// - For modules that should be available on both WebAssembly and non-WebAssembly targets,
///   declare them normally without using this macro
#[macro_export]
macro_rules! non_wasm_modules {
    (
        $(
            $(#[$attr:meta])*
            $vis:vis mod $name:ident;
        )+
    ) => {
        $(
            $(#[$attr])*
            #[cfg(not(target_arch = "wasm32"))]
            $vis mod $name;
        )+
    };
}

/// Add puffin profiling to the function if the profiling feature is enabled and the target is not wasm32
#[macro_export]
macro_rules! profile_function {
    () => {
        #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
        puffin::profile_function!();
    };
}
