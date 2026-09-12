use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use proc_macro2::{Ident, Span};
use syn::spanned::Spanned;
use syn::{Attribute, Error, Expr, Type, parse_quote_spanned};

use crate::ascent_syntax::{AscentProgram, BodyClauseArg, BodyItemNode, HeadItemNode, body_item_get_bound_vars};

const PROVENANCE_ATTR: &str = "provenance";

struct RelationInfo {
   logical_arity: usize,
   provenance: bool,
}

pub(crate) fn lower_why_provenance(prog: &mut AscentProgram, is_parallel: bool) -> syn::Result<()> {
   let mut errors = None;
   let mut relation_info = HashMap::new();

   for relation in prog.relations.iter_mut() {
      let provenance_attrs =
         relation.attrs.iter().filter(|attr| attr.meta.path().is_ident(PROVENANCE_ATTR)).cloned().collect_vec();
      if provenance_attrs.len() > 1 {
         combine_error(
            &mut errors,
            Error::new_spanned(&provenance_attrs[1], "multiple `provenance` attributes specified"),
         );
         continue;
      }
      let Some(attr) = provenance_attrs.first() else { continue };
      if relation.is_lattice {
         combine_error(
            &mut errors,
            Error::new_spanned(attr, "`provenance` cannot be applied to a user-declared `lattice`"),
         );
         continue;
      }
      if is_parallel {
         combine_error(&mut errors, Error::new_spanned(attr, "`provenance` is only supported by serial Ascent"));
         continue;
      }
      if let Some(ds_attr) = relation.attrs.iter().find(|attr| attr.meta.path().is_ident("ds")) {
         combine_error(
            &mut errors,
            Error::new_spanned(ds_attr, "annotated relations cannot have custom data structure providers"),
         );
         continue;
      }
      let token_type = match parse_provenance_type(attr) {
         Ok(token_type) => token_type,
         Err(error) => {
            combine_error(&mut errors, error);
            continue;
         },
      };

      relation.attrs.retain(|attr| !attr.meta.path().is_ident(PROVENANCE_ATTR));
      relation.field_types.push(parse_quote_spanned! {attr.span()=> ::ascent::WhyProvenance<#token_type>});
      relation.is_lattice = true;
      relation.is_provenance = true;
   }

   for relation in prog.relations.iter().rev() {
      relation_info.entry(relation.name.to_string()).or_insert_with(|| RelationInfo {
         logical_arity: relation.field_types.len() - usize::from(relation.is_provenance),
         provenance: relation.is_provenance,
      });
   }

   if let Some(error) = errors {
      return Err(error);
   }

   for rule in prog.rules.iter_mut() {
      validate_rule_arities(rule, &relation_info)?;
      let annotated_head = rule.head_clauses.iter().any(|head| {
         let HeadItemNode::HeadClause(head) = head else { unreachable!("rule macros must be expanded") };
         relation_info[&head.rel.to_string()].provenance
      });

      if annotated_head && rule.body_items.is_empty() {
         let span = rule.head_clauses.first().map(body_head_span).unwrap_or_else(Span::call_site);
         return Err(Error::new(
            span, "empty-body fact rules cannot have an annotated head; initialize the relation with tagged rows",
         ));
      }
      if annotated_head {
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
            BodyItemNode::Clause(clause) if relation_info[&clause.rel.to_string()].provenance => {
               let annotation_expr = if annotated_head {
                  let annotation = fresh_annotation_ident(&forbidden_names, &mut annotation_counter);
                  body_annotations.push(annotation.clone());
                  parse_quote_spanned! {clause.rel.span()=> #annotation}
               } else {
                  parse_quote_spanned! {clause.rel.span()=> _}
               };
               clause.args.push(BodyClauseArg::Expr(annotation_expr));
            },
            BodyItemNode::Agg(aggregate) if relation_info[&aggregate.rel.to_string()].provenance => {
               aggregate.rel_args.push(parse_quote_spanned! {aggregate.rel.span()=> _});
            },
            BodyItemNode::Negation(negation) if relation_info[&negation.rel.to_string()].provenance => {
               negation.args.push(parse_quote_spanned! {negation.rel.span()=> _});
            },
            _ => {},
         }
      }

      for head_item in rule.head_clauses.iter_mut() {
         let HeadItemNode::HeadClause(head) = head_item else { unreachable!("rule macros must be expanded") };
         if !relation_info[&head.rel.to_string()].provenance {
            continue;
         }
         let product = provenance_product(&body_annotations, head.rel.span());
         head.args.push(product);
      }
   }

   Ok(())
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

fn body_head_span(item: &HeadItemNode) -> Span {
   match item {
      HeadItemNode::HeadClause(head) => head.rel.span(),
      HeadItemNode::MacroInvocation(invocation) => invocation.span(),
   }
}

fn parse_provenance_type(attr: &Attribute) -> syn::Result<Type> {
   let list = attr.meta.require_list().map_err(|_| Error::new_spanned(attr, "expected `#[provenance(TokenType)]`"))?;
   syn::parse2::<Type>(list.tokens.clone())
      .map_err(|_| Error::new_spanned(attr, "expected exactly one token type in `#[provenance(TokenType)]`"))
}

fn validate_rule_arities(
   rule: &crate::ascent_syntax::RuleNode, relation_info: &HashMap<String, RelationInfo>,
) -> syn::Result<()> {
   for head_item in rule.head_clauses.iter() {
      let HeadItemNode::HeadClause(head) = head_item else { unreachable!("rule macros must be expanded") };
      validate_arity(&head.rel, head.args.len(), relation_info)?;
   }
   for body_item in rule.body_items.iter() {
      match body_item {
         BodyItemNode::Clause(clause) => validate_arity(&clause.rel, clause.args.len(), relation_info)?,
         BodyItemNode::Agg(aggregate) => validate_arity(&aggregate.rel, aggregate.rel_args.len(), relation_info)?,
         BodyItemNode::Negation(negation) => validate_arity(&negation.rel, negation.args.len(), relation_info)?,
         BodyItemNode::Generator(_) | BodyItemNode::Cond(_) => {},
         BodyItemNode::Disjunction(_) | BodyItemNode::MacroInvocation(_) => {
            unreachable!("macros and disjunctions must be expanded before provenance lowering")
         },
      }
   }
   Ok(())
}

fn validate_arity(rel: &Ident, found: usize, relation_info: &HashMap<String, RelationInfo>) -> syn::Result<()> {
   let Some(info) = relation_info.get(&rel.to_string()) else {
      return Err(Error::new(rel.span(), format!("relation `{rel}` is not defined")));
   };
   if info.logical_arity != found {
      return Err(Error::new(
         rel.span(),
         format!(
            "wrong arity for relation `{rel}` (expected {expected}, found {found})",
            expected = info.logical_arity,
         ),
      ));
   }
   Ok(())
}

fn provenance_product(annotations: &[Ident], span: Span) -> Expr {
   let Some((first, rest)) = annotations.split_first() else {
      return parse_quote_spanned! {span=> ::ascent::internal::why_provenance_one()};
   };
   let mut product: Expr = parse_quote_spanned! {span=> (*#first).clone()};
   for annotation in rest {
      product = parse_quote_spanned! {span=>
         ::ascent::internal::why_provenance_product(&(#product), #annotation)
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

fn combine_error(errors: &mut Option<Error>, error: Error) {
   if let Some(errors) = errors {
      errors.combine(error);
   } else {
      *errors = Some(error);
   }
}
