#[macro_use]
extern crate quote;
mod syntax;
mod expand;
mod syntax_utils;
mod lower;
mod wrapper;

/// Define a named provenance program backed by stock Ascent.
#[proc_macro]
pub fn provenance(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand(input.into(), false).unwrap_or_else(|e| e.to_compile_error()).into()
}

/// Execute a provenance program, with initializers in the caller's scope.
#[proc_macro]
pub fn provenance_run(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand(input.into(), true).unwrap_or_else(|e| e.to_compile_error()).into()
}

fn expand(input: proc_macro2::TokenStream, inline: bool) -> syn::Result<proc_macro2::TokenStream> {
    let callback = if inline { syn::parse_quote!(::ascent_provenance::provenance_run) }
        else { syn::parse_quote!(::ascent_provenance::provenance) };
    match syntax::parse_program(input, callback)? {
        syntax::Parsed::Include(tokens) => Ok(tokens),
        syntax::Parsed::Program(mut program) => {
            expand::expand_program(&mut program)?;
            wrapper::emit_wrapper(lower::lower(program)?, inline)
        }
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod diagnostics;
