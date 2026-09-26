use proc_macro2::TokenStream;

use crate::lower::AnnotatedRelation;

/// Coalesce logical keys in first-seen order before stock Ascent indexes them.
pub(crate) fn input(annotation: &AnnotatedRelation, expression: TokenStream) -> TokenStream {
   let types = &annotation.key_types;
   let value_type = &annotation.annotation_type;
   let index = syn::Index::from(types.len());
   let keys: Vec<_> = (0..types.len()).map(syn::Index::from).collect();
   quote! {{
       // Evaluate before introducing other locals; caller captures stay intact.
       let input: ::std::vec::Vec<(#(#types,)* #value_type,)> = #expression;
       let mut rows: ::std::vec::Vec<(#(#types,)* #value_type,)> = ::std::vec::Vec::with_capacity(input.len());
       let mut positions: ::std::collections::HashMap<(#(#types,)*), usize> =
           ::std::collections::HashMap::with_capacity(input.len());
       for row in input {
           if row.#index.witnesses().is_empty() { continue; }
           let key = (#(::std::clone::Clone::clone(&row.#keys),)*);
           match positions.entry(key) {
               ::std::collections::hash_map::Entry::Occupied(position) => {
                   ::ascent::Lattice::join_mut(&mut rows[*position.get()].#index, row.#index);
               }
               ::std::collections::hash_map::Entry::Vacant(position) => {
                   position.insert(rows.len());
                   rows.push(row);
               }
           }
       }
       rows
   }}
}
