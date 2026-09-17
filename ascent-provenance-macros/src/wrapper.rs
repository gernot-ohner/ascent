use std::collections::HashSet;
use proc_macro2::{Span, TokenStream};
use syn::{parse_quote, Attribute, Error, Ident, Meta, Result, Token};
use syn::punctuated::Punctuated;
use syn::parse::Parser;

use crate::lower::LoweredProgram;
use crate::syntax::{emit_program, Signatures};
use crate::syntax_utils::token_stream_idents;

fn validate_attribute(meta: &Meta) -> Result<()> {
    let path = meta.path();
    if ["doc", "cfg", "allow", "warn", "deny", "forbid", "expect"].iter().any(|name| path.is_ident(name)) {
        return Ok(());
    }
    if path.is_ident("cfg_attr") {
        let args = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(meta.require_list()?.tokens.clone())?;
        if args.len() < 2 { return Err(Error::new_spanned(meta, "expected condition and attributes")); }
        for attr in args.iter().skip(1) { validate_attribute(attr)?; }
        return Ok(());
    }
    Err(Error::new_spanned(meta, "unsupported provenance wrapper attribute; use documentation, cfg, or lint attributes"))
}

pub(crate) fn emit_wrapper(mut lowered: LoweredProgram, inline: bool) -> Result<TokenStream> {
    let mut names = token_stream_idents(emit_program(&lowered.program)).into_iter()
        .map(|i| i.to_string()).collect::<HashSet<_>>();
    let mut fresh = |base: &str| {
        let mut name = base.to_owned();
        while names.contains(&name) { name.push('_'); }
        names.insert(name.clone());
        Ident::new(&name, Span::mixed_site())
    };
    let signatures: Signatures = lowered.program.signatures.take().unwrap_or_else(|| parse_quote!(struct AscentProgram;));
    let declaration = &signatures.declaration;
    let name = &declaration.ident;
    let engine = fresh(&format!("__AscentProvenanceEngine_{name}"));
    let inner = fresh("__provenance_inner");
    let started = fresh("__provenance_started");
    let normalize = fresh("__provenance_normalize");
    let prepare = fresh("__provenance_prepare");
    let value = fresh("__provenance_value");
    let attrs = &declaration.attrs;
    for attr in attrs { validate_attribute(&attr.meta)?; }
    let gating: Vec<&Attribute> = attrs.iter().filter(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr")).collect();
    let vis = &declaration.visibility;
    let (ty_impl, ty_args, ty_bounds) = signatures.split_ty_generics_for_impl();
    let (impl_params, impl_args, impl_bounds) = signatures.split_impl_generics_for_impl();
    if let Some(i) = &signatures.implementation {
        if i.ident != *name || quote!(#ty_args).to_string() != quote!(#impl_args).to_string() {
            return Err(Error::new_spanned(&i.ident, "the struct and impl names and generic parameters must match"));
        }
    }
    let mut engine_signature = signatures.clone();
    engine_signature.declaration.ident = engine.clone();
    engine_signature.declaration.attrs = vec![parse_quote!(#[doc(hidden)]), parse_quote!(#[allow(non_camel_case_types)])];
    if let Some(i) = &mut engine_signature.implementation { i.ident = engine.clone(); }
    lowered.program.signatures = Some(engine_signature);

    // Stock Ascent evaluates initializers in relation-name order.
    let mut initializers = Vec::new();
    for relation in &mut lowered.program.relations {
        if let Some(expression) = relation.initialization.take() {
            initializers.push((relation.name.clone(), expression));
        }
    }
    initializers.sort_by_key(|(name,_)| name.to_string());
    let initialize: Vec<_> = initializers.iter().map(|(field, expression)| quote! {
        #value.#inner.#field = #expression;
    }).collect();
    let normalization = lowered.annotated.iter().map(|a| {
        let field = &a.name;
        let types = &a.key_types;
        let annotation = &a.annotation_type;
        let index = syn::Index::from(types.len());
        let keys: Vec<_> = (0..types.len()).map(syn::Index::from).collect();
        quote! {
            {
                let mut rows: ::std::vec::Vec<(#(#types,)* #annotation,)> = ::std::vec::Vec::new();
                for row in ::std::mem::take(&mut self.#inner.#field) {
                    if row.#index.witnesses().is_empty() { continue; }
                    if let Some(old) = rows.iter_mut().find(|old| true #(&& old.#keys == row.#keys)*) {
                        ::ascent::Lattice::join_mut(&mut old.#index, row.#index);
                    } else { rows.push(row); }
                }
                self.#inner.#field = rows;
            }
        }
    });
    let timeout = lowered.program.attributes.iter().any(|a| a.path().is_ident("generate_run_timeout")).then(|| quote! {
        pub fn run_timeout(&mut self, timeout: ::std::time::Duration) -> bool {
            self.#prepare();
            self.#inner.run_timeout(timeout)
        }
    });
    let summary = if inline {
        quote!(pub fn summary(&self) -> &'static str { <#engine #impl_args>::summary() })
    } else {
        quote!(pub fn summary() -> &'static str { <#engine #impl_args>::summary() })
    };
    let default_initializers = if inline { Vec::new() } else { initialize.clone() };
    let program = emit_program(&lowered.program);
    let items = quote! {
        #(#gating)*
        ::ascent::ascent! { #program }
        #(#attrs)*
        #vis struct #name #ty_impl #ty_bounds {
            #inner: #engine #ty_args,
            #started: bool,
        }
        #(#gating)*
        impl #ty_impl ::std::ops::Deref for #name #ty_args #ty_bounds {
            type Target = #engine #ty_args;
            fn deref(&self) -> &Self::Target { &self.#inner }
        }
        #(#gating)*
        impl #ty_impl ::std::ops::DerefMut for #name #ty_args #ty_bounds {
            fn deref_mut(&mut self) -> &mut Self::Target { &mut self.#inner }
        }
        #(#gating)*
        impl #impl_params ::std::default::Default for #name #impl_args #impl_bounds {
            fn default() -> Self {
                let mut #value = Self { #inner: ::std::default::Default::default(), #started: false };
                #(#default_initializers)*
                #value.#normalize();
                #value
            }
        }
        #(#gating)*
        impl #impl_params #name #impl_args #impl_bounds {
            fn #normalize(&mut self) { #(#normalization)* }
            fn #prepare(&mut self) {
                if !self.#started {
                    self.#normalize();
                    self.#started = true;
                }
            }
            pub fn run(&mut self) { self.#prepare(); self.#inner.run(); }
            #timeout
            #summary
        }
    };
    Ok(if inline {
        quote! {{
            #items
            let mut #value: #name #ty_args = ::std::default::Default::default();
            #(#initialize)*
            #value.run();
            #value
        }}
    } else { items })
}
