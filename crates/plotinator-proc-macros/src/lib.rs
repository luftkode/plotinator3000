/// If the `profiling` feature is enabled, logs the execution time of the function at `info` verbosity
#[proc_macro_attribute]
pub fn log_time(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // This function will be replaced by one of the two modules below,
    // depending on whether the `profiling` feature is enabled for the
    // crate that USES this macro.
    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    return timed_impl(item);

    #[cfg(not(all(feature = "profiling", not(target_arch = "wasm32"))))]
    return item; // If not profiling, return the original function unchanged.
}

#[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
fn timed_impl(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use quote::quote;
    use syn::{ItemFn, parse_macro_input};

    let mut func = parse_macro_input!(item as ItemFn);
    // Get the function's name (identifier).
    let func_name = &func.sig.ident;
    let func_name_str = func_name.to_string();

    // Get the function's body (the block of code).
    let block = &func.block;

    // Create the new function body with timing logic.
    // We wrap the original block to capture its return value.
    let new_body = quote! {
        {
            let start = ::std::time::Instant::now();
            let result = #block; // Execute original function body
            ::log::info!(
                "Function '{}' executed in {:.2?}",
                #func_name_str,
                start.elapsed()
            );
            result // Return the original result
        }
    };

    // Replace the old block with our new timed block.
    func.block = syn::parse2(new_body).expect("Failed to parse new function body.");

    // Convert the modified function syntax tree back into tokens.
    proc_macro::TokenStream::from(quote! { #func })
}
