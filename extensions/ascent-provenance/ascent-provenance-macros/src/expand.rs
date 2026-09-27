use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::parse::{ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Error, Expr, ExprMacro, Ident, Result, Token};

use crate::syntax::*;
use crate::syntax_utils::*;

fn rule_desugar_disjunction_nodes(rule: RuleNode) -> Vec<RuleNode> {
   fn bitem_desugar(bitem: &BodyItemNode) -> Vec<Vec<BodyItemNode>> {
      match bitem {
         BodyItemNode::Generator(_) => vec![vec![bitem.clone()]],
         BodyItemNode::Clause(_) => vec![vec![bitem.clone()]],
         BodyItemNode::Cond(_) => vec![vec![bitem.clone()]],
         BodyItemNode::Agg(_) => vec![vec![bitem.clone()]],
         BodyItemNode::Negation(_) => vec![vec![bitem.clone()]],
         BodyItemNode::Disjunction(d) => {
            let mut res = vec![];
            for disjunt in d.disjuncts.iter() {
               for conjunction in bitems_desugar(&disjunt.iter().cloned().collect_vec()) {
                  res.push(conjunction);
               }
            }
            res
         },
         BodyItemNode::MacroInvocation(m) => panic!("unexpected macro invocation: {:?}", m.mac.path),
      }
   }
   fn bitems_desugar(bitems: &[BodyItemNode]) -> Vec<Vec<BodyItemNode>> {
      let mut res = vec![];
      if !bitems.is_empty() {
         let sub_res = bitems_desugar(&bitems[0..bitems.len() - 1]);
         let last_desugared = bitem_desugar(&bitems[bitems.len() - 1]);
         for sub_res_item in sub_res.into_iter() {
            for last_item in last_desugared.iter() {
               let mut res_item = sub_res_item.clone();
               res_item.extend(last_item.clone());
               res.push(res_item);
            }
         }
      } else {
         res.push(vec![]);
      }

      res
   }

   let mut res = vec![];
   for conjunction in bitems_desugar(&rule.body_items) {
      res.push(RuleNode { body_items: conjunction, head_clauses: rule.head_clauses.clone() })
   }
   res
}

pub(crate) fn body_item_get_bound_vars(bi: &BodyItemNode) -> Vec<Ident> {
   match bi {
      BodyItemNode::Generator(gen) => pattern_get_vars(&gen.pattern),
      BodyItemNode::Agg(agg) => pattern_get_vars(&agg.pat),
      BodyItemNode::Clause(cl) => cl.args.iter().flat_map(|arg| arg.get_vars()).collect(),
      BodyItemNode::Negation(_cl) => vec![],
      BodyItemNode::Disjunction(disj) =>
         disj.disjuncts.iter().flat_map(|conj| conj.iter().flat_map(body_item_get_bound_vars)).collect(),
      BodyItemNode::Cond(cl) => cl.bound_vars(),
      BodyItemNode::MacroInvocation(_) => vec![],
   }
}

fn body_item_visit_bound_vars_mut(bi: &mut BodyItemNode, visitor: &mut dyn FnMut(&mut Ident)) {
   match bi {
      BodyItemNode::Generator(gen) => pattern_visit_vars_mut(&mut gen.pattern, visitor),
      BodyItemNode::Agg(agg) => pattern_visit_vars_mut(&mut agg.pat, visitor),
      BodyItemNode::Clause(cl) =>
         for arg in cl.args.iter_mut() {
            match arg {
               BodyClauseArg::Pat(p) => pattern_visit_vars_mut(&mut p.pattern, visitor),
               BodyClauseArg::Expr(e) =>
                  if let Some(ident) = expr_to_ident_mut(e) {
                     visitor(ident)
                  },
            }
         },
      BodyItemNode::Negation(_cl) => (),
      BodyItemNode::Disjunction(disj) =>
         for conj in disj.disjuncts.iter_mut() {
            for bi in conj.iter_mut() {
               body_item_visit_bound_vars_mut(bi, visitor)
            }
         },
      BodyItemNode::Cond(cl) => match cl {
         CondClause::IfLet(cl) => pattern_visit_vars_mut(&mut cl.pattern, visitor),
         CondClause::If(_cl) => (),
         CondClause::Let(cl) => pattern_visit_vars_mut(&mut cl.pattern, visitor),
      },
      BodyItemNode::MacroInvocation(_) => (),
   }
}

fn body_item_visit_exprs_free_vars_mut(
   bi: &mut BodyItemNode, visitor: &mut dyn FnMut(&mut Ident), visit_macro_idents: bool,
) {
   let mut visit = |expr: &mut Expr| {
      expr_visit_free_vars_mut(expr, visitor);
      if visit_macro_idents {
         expr_visit_idents_in_macros_mut(expr, visitor);
      }
   };
   match bi {
      BodyItemNode::Generator(gen) => visit(&mut gen.expr),
      BodyItemNode::Agg(agg) => {
         for arg in agg.rel_args.iter_mut() {
            visit(arg)
         }
         if let AggregatorNode::Expr(e) = &mut agg.aggregator {
            visit(e)
         }
      },
      BodyItemNode::Clause(cl) =>
         for arg in cl.args.iter_mut() {
            if let BodyClauseArg::Expr(e) = arg {
               visit(e);
            }
         },
      BodyItemNode::Negation(cl) =>
         for arg in cl.args.iter_mut() {
            visit(arg);
         },
      BodyItemNode::Disjunction(disj) =>
         for conj in disj.disjuncts.iter_mut() {
            for bi in conj.iter_mut() {
               body_item_visit_exprs_free_vars_mut(bi, visitor, visit_macro_idents);
            }
         },
      BodyItemNode::Cond(cl) => match cl {
         CondClause::IfLet(cl) => visit(&mut cl.exp),
         CondClause::If(cl) => visit(&mut cl.cond),
         CondClause::Let(cl) => visit(&mut cl.exp),
      },
      BodyItemNode::MacroInvocation(m) => {
         update(&mut m.mac.tokens, |ts| token_stream_replace_ident(ts, visitor));
      },
   }
}

#[derive(Clone)]
struct GenSym(HashMap<String, u32>, fn(&str) -> String);
impl GenSym {
   pub fn next(&mut self, ident: &str) -> String {
      match self.0.get_mut(ident) {
         Some(n) => {
            *n += 1;
            format!("{}{}", self.1(ident), *n - 1)
         },
         None => {
            self.0.insert(ident.into(), 1);
            self.1(ident)
         },
      }
   }
   pub fn new(transformer: fn(&str) -> String) -> Self { Self(Default::default(), transformer) }
}

impl Default for GenSym {
   fn default() -> Self { Self(Default::default(), |x| format!("{}_", x)) }
}

fn body_items_rename_macro_originated_vars(
   bis: &mut [&mut BodyItemNode], macro_def: &MacroDefNode, gensym: &mut GenSym,
) {
   let bi_vars = bis.iter().flat_map(|bi| body_item_get_bound_vars(bi)).collect_vec();
   let mut mac_body_idents = token_stream_idents(macro_def.body.clone());
   mac_body_idents.retain(|ident| bi_vars.contains(ident));

   let macro_originated_vars = bi_vars
      .iter()
      .filter(|v| mac_body_idents.iter().any(|ident| spans_eq(&v.span(), &ident.span())))
      .cloned()
      .collect::<HashSet<_>>();

   let var_mappings = macro_originated_vars.iter().map(|v| (v, gensym.next(&v.to_string()))).collect::<HashMap<_, _>>();
   let mut visitor = |ident: &mut Ident| {
      if let Some(replacement) = var_mappings.get(ident) {
         if mac_body_idents.iter().any(|mac_ident| spans_eq(&mac_ident.span(), &ident.span())) {
            *ident = Ident::new(replacement, ident.span())
         }
      }
   };
   for bi in bis.iter_mut() {
      body_item_visit_bound_vars_mut(bi, &mut visitor);
      body_item_visit_exprs_free_vars_mut(bi, &mut visitor, true);
   }
}

fn invoke_macro(invocation: &ExprMacro, definition: &MacroDefNode) -> Result<TokenStream> {
   let tokens = invocation.mac.tokens.clone();

   fn parse_args(definition: &MacroDefNode, args: ParseStream, span: Span) -> Result<HashMap<Ident, TokenStream>> {
      let mut ident_replacement = HashMap::new();

      for pair in definition.params.pairs() {
         if args.is_empty() {
            return Err(Error::new(span, "expected more arguments"));
         }
         let (param, comma) = pair.into_tuple();
         let arg = match param.kind {
            MacroParamKind::Expr(_) => args.parse::<Ident>()?.into_token_stream(),
            MacroParamKind::Ident(_) => args.parse::<Expr>()?.into_token_stream(),
         };

         ident_replacement.insert(param.name.clone(), arg);
         if comma.is_some() {
            if args.is_empty() {
               return Err(Error::new(span, "expected more arguments"));
            }
            args.parse::<Token![,]>()?;
         }
      }

      Ok(ident_replacement)
   }

   let args_parser = |inp: ParseStream| parse_args(definition, inp, invocation.mac.span());
   let args_parsed = Parser::parse2(args_parser, tokens)?;

   let replaced_body = token_stream_replace_macro_idents(definition.body.clone(), &args_parsed);
   Ok(replaced_body)
}

fn rule_expand_macro_invocations(rule: RuleNode, macros: &HashMap<Ident, &MacroDefNode>) -> Result<RuleNode> {
   const RECURSIVE_MACRO_ERROR: &'static str = "recursively defined Ascent macro";
   fn body_item_expand_macros(
      bi: BodyItemNode, macros: &HashMap<Ident, &MacroDefNode>, gensym: &mut GenSym, depth: i16, span: Option<Span>,
   ) -> Result<Punctuated<BodyItemNode, Token![,]>> {
      if depth <= 0 {
         return Err(Error::new(span.unwrap_or_else(Span::call_site), RECURSIVE_MACRO_ERROR))
      }
      match bi {
         BodyItemNode::MacroInvocation(m) => {
            let mac_def =
               macros.get(m.mac.path.get_ident().unwrap()).ok_or_else(|| Error::new(m.span(), "undefined macro"))?;
            let macro_invoked = invoke_macro(&m, mac_def)?;
            let expanded_bis = Parser::parse2(Punctuated::<BodyItemNode, Token![,]>::parse_terminated, macro_invoked)?;
            let mut recursively_expanded = punctuated_try_map(expanded_bis, |ebi| {
               body_item_expand_macros(ebi, macros, gensym, depth - 1, Some(m.span()))
            })?
            .pipe(flatten_punctuated);
            body_items_rename_macro_originated_vars(
               &mut recursively_expanded.iter_mut().collect_vec(),
               mac_def,
               gensym,
            );
            Ok(recursively_expanded)
         },
         BodyItemNode::Disjunction(disj) => {
            let new_disj: Punctuated<Result<_>, _> = punctuated_map(disj.disjuncts, |bis| {
               let new_bis = punctuated_map(bis, |bi| {
                  body_item_expand_macros(bi, macros, gensym, depth - 1, Some(disj.paren.span.join()))
               });
               Ok(flatten_punctuated(punctuated_try_unwrap(new_bis)?))
            });

            Ok(punctuated_singleton(BodyItemNode::Disjunction(DisjunctionNode {
               disjuncts: punctuated_try_unwrap(new_disj)?,
               ..disj
            })))
         },
         _ => Ok(punctuated_singleton(bi)),
      }
   }

   fn head_item_expand_macros(
      hi: HeadItemNode, macros: &HashMap<Ident, &MacroDefNode>, depth: i16, span: Option<Span>,
   ) -> Result<Punctuated<HeadItemNode, Token![,]>> {
      if depth <= 0 {
         return Err(Error::new(span.unwrap_or_else(Span::call_site), RECURSIVE_MACRO_ERROR))
      }
      match hi {
         HeadItemNode::MacroInvocation(m) => {
            let mac_def =
               macros.get(m.mac.path.get_ident().unwrap()).ok_or_else(|| Error::new(m.span(), "undefined macro"))?;
            let macro_invoked = invoke_macro(&m, mac_def)?;
            let expanded_his = Parser::parse2(Punctuated::<HeadItemNode, Token![,]>::parse_terminated, macro_invoked)?;

            Ok(punctuated_map(expanded_his, |ehi| head_item_expand_macros(ehi, macros, depth - 1, Some(m.span())))
               .pipe(punctuated_try_unwrap)?
               .pipe(flatten_punctuated))
         },
         HeadItemNode::HeadClause(_) => Ok(punctuated_singleton(hi)),
      }
   }

   let mut gensym = GenSym::new(|s| format!("__{}_", s));

   let new_body_items = rule
      .body_items
      .into_iter()
      .map(|bi| body_item_expand_macros(bi, macros, &mut gensym, 100, None))
      .collect::<Result<Vec<_>>>()?
      .into_iter()
      .flatten()
      .collect_vec();

   let new_head_items = punctuated_map(rule.head_clauses, |hi| head_item_expand_macros(hi, macros, 100, None))
      .pipe(punctuated_try_unwrap)?
      .pipe(flatten_punctuated);

   Ok(RuleNode { body_items: new_body_items, head_clauses: new_head_items })
}

pub(crate) fn expand_program(prog: &mut AscentProgram) -> Result<()> {
   let macros = prog.macros.iter().map(|m| (m.name.clone(), m)).collect::<HashMap<_, _>>();
   let expanded = std::mem::take(&mut prog.rules)
      .into_iter()
      .map(|r| rule_expand_macro_invocations(r, &macros))
      .collect::<Result<Vec<_>>>()?;
   prog.rules = expanded.into_iter().flat_map(rule_desugar_disjunction_nodes).collect();
   prog.macros.clear();
   Ok(())
}
