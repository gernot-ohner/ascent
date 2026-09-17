use std::collections::HashSet;

use itertools::Itertools;
use proc_macro2::{Ident, Span};
use syn::spanned::Spanned;
use syn::{Attribute, Error, Expr, Path, Type, parse_quote, parse_quote_spanned};

use crate::syntax::{AscentProgram, BodyClauseArg, BodyItemNode, HeadItemNode, RelationNode};
use crate::expand::body_item_get_bound_vars;

const PROVENANCE_ATTR: &str = "provenance";

pub(crate) fn lower(mut prog: AscentProgram) -> syn::Result<LoweredProgram> {
   let mut boolean = false;
   for attr in prog.attributes.iter().filter(|attr| attr.path().is_ident(PROVENANCE_ATTR)) {
      if boolean {
         return Err(Error::new_spanned(attr, "multiple program-level `provenance` attributes specified"));
      }
      if !matches!(attr.parse_args::<Ident>(), Ok(mode) if mode == "boolean") {
         return Err(Error::new_spanned(attr, "expected `#![provenance(boolean)]`"));
      }
      boolean = true;
   }
   prog.attributes.retain(|attr| !attr.path().is_ident(PROVENANCE_ATTR));
   let provenance_type: Path =
      if boolean { parse_quote!(::ascent_provenance::BooleanProvenance) } else { parse_quote!(::ascent_provenance::WhyProvenance) };
   let mut annotation_types = Vec::<(usize, Type)>::new();

   for (index, relation) in prog.relations.iter_mut().enumerate() {
      let provenance_attrs =
         relation.attrs.iter().filter(|attr| attr.meta.path().is_ident(PROVENANCE_ATTR)).cloned().collect_vec();
      if provenance_attrs.len() > 1 {
         return Err(Error::new_spanned(&provenance_attrs[1], "multiple `provenance` attributes specified"));
      }
      let Some(attr) = provenance_attrs.first() else { continue };
      if relation.is_lattice {
         return Err(Error::new_spanned(attr, "`provenance` cannot be applied to a user-declared `lattice`"));
      }
      if relation.attrs.iter().any(|a| a.path().is_ident("ds")) {
         return Err(Error::new_spanned(attr, "annotated relations cannot use custom data structure providers"));
      }
      let token_type = parse_provenance_type(attr)?;

      relation.attrs.retain(|attr| !attr.meta.path().is_ident(PROVENANCE_ATTR));
      annotation_types.push((index, parse_quote_spanned! {attr.span()=> #provenance_type<#token_type>}));
      relation.is_lattice = true;

   }

   if annotation_types.is_empty() {
      return Ok(LoweredProgram { program: prog, annotated: vec![] });
   }

   let tracked = annotation_types.iter().map(|(i,_)| prog.relations[*i].name.to_string()).collect::<HashSet<_>>();

   // Keep declarations at logical arity until every rule has used the existing validator.
   let mut rules = std::mem::take(&mut prog.rules);
   for rule in &mut rules {
      let mut annotated_head = None;
      for head in &rule.head_clauses {
         let HeadItemNode::HeadClause(head) = head else { unreachable!("rule macros must be expanded") };
         if tracked.contains(&prog_get_relation(&prog, &head.rel, head.args.len())?.name.to_string()) {
            annotated_head = Some(head.rel.span());
         }
      }

      if let Some(span) = annotated_head {
         if rule.body_items.is_empty() {
            return Err(Error::new(
               span, "empty-body fact rules cannot have an annotated head; initialize the relation with tagged rows",
            ));
         }
         if let Some(item) =
            rule.body_items.iter().find(|item| matches!(item, BodyItemNode::Agg(_) | BodyItemNode::Negation(_)))
         {
            return Err(Error::new(
               body_item_span(item),
               "rules with an annotated head cannot use aggregation or negation",
            ));
         }
      }

      let mut forbidden_names = rule
         .body_items
         .iter()
         .flat_map(body_item_get_bound_vars)
         .map(|ident| ident.to_string())
         .collect::<HashSet<_>>();
      for body_item in &rule.body_items {
         if let BodyItemNode::Clause(clause) = body_item {
            forbidden_names.extend(
               clause
                  .cond_clauses
                  .iter()
                  .flat_map(|cond_clause| cond_clause.bound_vars())
                  .map(|ident| ident.to_string()),
            );
         }
      }
      let mut annotation_counter = 0;
      let mut body_annotations = Vec::new();
      for body_item in rule.body_items.iter_mut() {
         match body_item {
            BodyItemNode::Clause(clause) if tracked.contains(&prog_get_relation(&prog, &clause.rel, clause.args.len())?.name.to_string()) => {
               let annotation_expr = if annotated_head.is_some() {
                  let annotation = fresh_annotation_ident(&forbidden_names, &mut annotation_counter);
                  body_annotations.push(annotation.clone());
                  parse_quote_spanned! {clause.rel.span()=> #annotation}
               } else {
                  parse_quote_spanned! {clause.rel.span()=> _}
               };
               clause.args.push(BodyClauseArg::Expr(annotation_expr));
            },
            BodyItemNode::Agg(aggregate)
               if tracked.contains(&prog_get_relation(&prog, &aggregate.rel, aggregate.rel_args.len())?.name.to_string()) =>
            {
               aggregate.rel_args.push(parse_quote_spanned! {aggregate.rel.span()=> _});
            },
            BodyItemNode::Negation(negation)
               if tracked.contains(&prog_get_relation(&prog, &negation.rel, negation.args.len())?.name.to_string()) =>
            {
               negation.args.push(parse_quote_spanned! {negation.rel.span()=> _});
            },
            _ => {},
         }
      }

      for head_item in rule.head_clauses.iter_mut() {
         let HeadItemNode::HeadClause(head) = head_item else { unreachable!("rule macros must be expanded") };
         if !tracked.contains(&prog_get_relation(&prog, &head.rel, head.args.len())?.name.to_string()) {
            continue;
         }
         let product = provenance_product(&body_annotations, head.rel.span(), &provenance_type);
         head.args.push(product);
      }
   }

   prog.rules = rules;
   let mut annotated = Vec::new();
   for (index, annotation_type) in annotation_types {
      let relation = &mut prog.relations[index];
      annotated.push(AnnotatedRelation {
         name: relation.name.clone(),
         key_types: relation.field_types.iter().cloned().collect(),
         annotation_type: annotation_type.clone(),
      });
      relation.field_types.push(annotation_type);
   }
   Ok(LoweredProgram { program: prog, annotated })
}

fn fresh_annotation_ident(forbidden_names: &HashSet<String>, counter: &mut usize) -> Ident {
   loop {
      let name = format!("__ascent_provenance_{}", *counter);
      *counter += 1;
      if !forbidden_names.contains(&name) {
         return Ident::new(&name, Span::mixed_site());
      }
   }
}

fn parse_provenance_type(attr: &Attribute) -> syn::Result<Type> {
   let list = attr.meta.require_list().map_err(|_| Error::new_spanned(attr, "expected `#[provenance(TokenType)]`"))?;
   syn::parse2::<Type>(list.tokens.clone())
      .map_err(|_| Error::new_spanned(attr, "expected exactly one token type in `#[provenance(TokenType)]`"))
}

fn provenance_product(annotations: &[Ident], span: Span, provenance_type: &Path) -> Expr {
   let Some((first, rest)) = annotations.split_first() else {
      return parse_quote_spanned! {span=> #provenance_type::__one()};
   };
   let mut product: Expr = parse_quote_spanned! {span=> (*#first).clone()};
   for annotation in rest {
      product = parse_quote_spanned! {span=>
         #provenance_type::__product(&(#product), #annotation)
      };
   }
   product
}

fn body_item_span(item: &BodyItemNode) -> Span {
   match item {
      BodyItemNode::Agg(aggregate) => aggregate.agg_kw.span,
      BodyItemNode::Negation(negation) => negation.rel.span(),
      _ => unreachable!(),
   }
}

pub(crate) struct AnnotatedRelation {
    pub name: Ident,
    pub key_types: Vec<Type>,
    pub annotation_type: Type,
}
pub(crate) struct LoweredProgram {
    pub program: AscentProgram,
    pub annotated: Vec<AnnotatedRelation>,
}

// Logical lookup only: no HIR, indexes, or dependency analysis.
fn prog_get_relation<'a>(prog: &'a AscentProgram, name: &Ident, arity: usize) -> syn::Result<&'a RelationNode> {
    let relation = prog.relations.iter().rev().find(|r| name == &r.name)
        .ok_or_else(|| Error::new(name.span(), format!("relation `{name}` is not defined")))?;
    if relation.field_types.len() != arity {
        return Err(Error::new(name.span(), format!(
            "wrong arity for relation `{name}` (expected {}, found {arity})", relation.field_types.len())));
    }
    Ok(relation)
}
