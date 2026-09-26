use std::collections::HashSet;

use proc_macro2::{Group, Ident, TokenStream, TokenTree};
use quote::ToTokens;
#[cfg(test)]
use syn::parse2;
use syn::visit_mut::VisitMut;
use syn::{Block, Expr, ExprMacro, Pat, Path, Stmt};

pub fn pattern_get_vars(pat: &Pat) -> Vec<Ident> {
   let mut res = vec![];
   match pat {
      Pat::Ident(pat_ident) => {
         res.push(pat_ident.ident.clone());
         if let Some(subpat) = &pat_ident.subpat {
            res.extend(pattern_get_vars(&subpat.1))
         }
      },
      Pat::Lit(_) => {},
      Pat::Macro(_) => {},
      Pat::Or(or_pat) => {
         let cases_vars = or_pat.cases.iter().map(pattern_get_vars).map(into_set);
         let intersection = cases_vars.reduce(|case_vars, accu| collect_set(case_vars.intersection(&accu).cloned()));
         if let Some(intersection) = intersection {
            res.extend(intersection);
         }
      },
      Pat::Path(_) => {},
      Pat::Range(_) => {},
      Pat::Reference(ref_pat) => res.extend(pattern_get_vars(&ref_pat.pat)),
      Pat::Rest(_) => {},
      Pat::Slice(slice_pat) =>
         for sub_pat in slice_pat.elems.iter() {
            res.extend(pattern_get_vars(sub_pat));
         },
      Pat::Struct(struct_pat) =>
         for field_pat in struct_pat.fields.iter() {
            res.extend(pattern_get_vars(&field_pat.pat));
         },
      Pat::Tuple(tuple_pat) =>
         for elem_pat in tuple_pat.elems.iter() {
            res.extend(pattern_get_vars(elem_pat));
         },
      Pat::TupleStruct(tuple_strcut_pat) =>
         for elem_pat in tuple_strcut_pat.elems.iter() {
            res.extend(pattern_get_vars(elem_pat));
         },
      Pat::Type(type_pat) => {
         res.extend(pattern_get_vars(&type_pat.pat));
      },
      Pat::Verbatim(_) => {},
      Pat::Wild(_) => {},
      _ => {},
   }
   // println!("pattern vars {} : {}", pat.to_token_stream(), res.iter().map(|ident| ident.to_string()).join(", "));
   res
}

pub fn pattern_visit_vars_mut(pat: &mut Pat, visitor: &mut dyn FnMut(&mut Ident)) {
   macro_rules! visit {
      ($e: expr) => {
         pattern_visit_vars_mut($e, visitor)
      };
   }
   match pat {
      Pat::Ident(pat_ident) => {
         visitor(&mut pat_ident.ident);
         if let Some(subpat) = &mut pat_ident.subpat {
            visit!(&mut subpat.1);
         }
      },
      Pat::Lit(_) => {},
      Pat::Macro(_) => {},
      Pat::Or(or_pat) =>
         for case in or_pat.cases.iter_mut() {
            visit!(case)
         },
      Pat::Path(_) => {},
      Pat::Range(_) => {},
      Pat::Reference(ref_pat) => visit!(&mut ref_pat.pat),
      Pat::Rest(_) => {},
      Pat::Slice(slice_pat) =>
         for sub_pat in slice_pat.elems.iter_mut() {
            visit!(sub_pat);
         },
      Pat::Struct(struct_pat) =>
         for field_pat in struct_pat.fields.iter_mut() {
            visit!(&mut field_pat.pat);
         },
      Pat::Tuple(tuple_pat) =>
         for elem_pat in tuple_pat.elems.iter_mut() {
            visit!(elem_pat);
         },
      Pat::TupleStruct(tuple_strcut_pat) =>
         for elem_pat in tuple_strcut_pat.elems.iter_mut() {
            visit!(elem_pat);
         },
      Pat::Type(type_pat) => {
         visit!(&mut type_pat.pat);
      },
      Pat::Verbatim(_) => {},
      Pat::Wild(_) => {},
      _ => {},
   }
}

#[test]
fn test_pattern_get_vars() {
   use syn::parse::Parser;

   let pattern = quote! {
      SomePair(x, (y, z))
   };
   let pat = Pat::parse_single.parse2(pattern).unwrap();
   assert_eq!(
      collect_set(["x", "y", "z"].iter().map(ToString::to_string)),
      pattern_get_vars(&pat).into_iter().map(|id| id.to_string()).collect()
   );
}

/// if the expression is a let expression (for example in `if let Some(foo) = bar {..}`),
/// returns the variables bound by the let expression
pub fn expr_get_let_bound_vars(expr: &Expr) -> Vec<Ident> {
   match expr {
      Expr::Let(l) => pattern_get_vars(&l.pat),
      _ => vec![],
   }
}

pub fn stmt_visit_free_vars_mut(stmt: &mut Stmt, visitor: &mut dyn FnMut(&mut Ident)) {
   match stmt {
      Stmt::Local(l) =>
         if let Some(init) = &mut l.init {
            expr_visit_free_vars_mut(&mut init.expr, visitor);
            if let Some(diverge) = &mut init.diverge {
               expr_visit_free_vars_mut(&mut diverge.1, visitor);
            }
         },
      Stmt::Item(_) => {},
      Stmt::Expr(e, _) => expr_visit_free_vars_mut(e, visitor),
      Stmt::Macro(m) => {
         eprintln!(
            "WARNING: cannot determine free variables of macro invocations. macro invocation:\n{}",
            m.to_token_stream()
         );
      },
   }
}

pub fn block_visit_free_vars_mut(block: &mut Block, visitor: &mut dyn FnMut(&mut Ident)) {
   let mut bound_vars = HashSet::new();
   for stmt in block.stmts.iter_mut() {
      let stmt_bound_vars = match stmt {
         Stmt::Local(local) => pattern_get_vars(&local.pat),
         _ => Vec::new(),
      };
      // A local binding is not in scope in its initializer or let-else block.
      stmt_visit_free_vars_mut(stmt, &mut |ident| {
         if !bound_vars.contains(ident) {
            visitor(ident)
         }
      });
      bound_vars.extend(stmt_bound_vars);
   }
}

/// visits free variables in the expr
pub fn expr_visit_free_vars_mut(expr: &mut Expr, visitor: &mut dyn FnMut(&mut Ident)) {
   macro_rules! visit {
      ($e: expr) => { expr_visit_free_vars_mut(&mut $e, visitor)};
   }
   macro_rules! visitor_except {
      ($excluded: expr) => {
         &mut |ident| {if ! $excluded.contains(ident) {visitor(ident)}}
      };
   }
   macro_rules! visit_except {
      ($e: expr, $excluded: expr) => { expr_visit_free_vars_mut($e, visitor_except!($excluded))};
   }
   match expr {
      Expr::Array(arr) =>
         for elem in arr.elems.iter_mut() {
            expr_visit_free_vars_mut(elem, visitor);
         },
      Expr::Assign(assign) => {
         visit!(assign.left);
         visit!(assign.right)
      },
      Expr::Async(a) => block_visit_free_vars_mut(&mut a.block, visitor),
      Expr::Await(a) => visit!(a.base),
      Expr::Binary(b) => {
         visit!(b.left);
         visit!(b.right)
      },
      Expr::Block(b) => block_visit_free_vars_mut(&mut b.block, visitor),
      Expr::Break(b) =>
         if let Some(b_e) = &mut b.expr {
            expr_visit_free_vars_mut(b_e, visitor)
         },
      Expr::Call(c) => {
         visit!(c.func);
         for arg in c.args.iter_mut() {
            expr_visit_free_vars_mut(arg, visitor)
         }
      },
      Expr::Cast(c) => visit!(c.expr),
      Expr::Closure(c) => {
         let input_vars: HashSet<_> = c.inputs.iter().flat_map(pattern_get_vars).collect();
         visit_except!(&mut c.body, input_vars);
      },
      Expr::Continue(_c) => {},
      Expr::Field(f) => visit!(f.base),
      Expr::ForLoop(f) => {
         let pat_vars: HashSet<_> = pattern_get_vars(&f.pat).into_iter().collect();
         visit!(f.expr);
         block_visit_free_vars_mut(&mut f.body, visitor_except!(pat_vars));
      },
      Expr::Group(g) => visit!(g.expr),
      Expr::If(e) => {
         let bound_vars = expr_get_let_bound_vars(&e.cond).into_iter().collect::<HashSet<_>>();
         visit!(e.cond);
         block_visit_free_vars_mut(&mut e.then_branch, visitor_except!(bound_vars));
         if let Some(eb) = &mut e.else_branch {
            visit!(eb.1)
         }
      },
      Expr::Index(i) => {
         visit!(i.expr);
         visit!(i.index)
      },
      Expr::Let(l) => visit!(l.expr),
      Expr::Lit(_) => {},
      Expr::Loop(l) => block_visit_free_vars_mut(&mut l.body, visitor),
      Expr::Macro(_m) => {
         eprintln!(
            "WARNING: cannot determine free variables of macro invocations. macro invocation:\n{}",
            expr.to_token_stream()
         )
      },
      Expr::Match(m) => {
         visit!(m.expr);
         for arm in m.arms.iter_mut() {
            if let Some(g) = &mut arm.guard {
               visit!(g.1);
            }
            let arm_vars = pattern_get_vars(&arm.pat).into_iter().collect::<HashSet<_>>();
            visit_except!(&mut arm.body, arm_vars);
         }
      },
      Expr::MethodCall(c) => {
         visit!(c.receiver);
         for arg in c.args.iter_mut() {
            expr_visit_free_vars_mut(arg, visitor)
         }
      },
      Expr::Paren(p) => visit!(p.expr),
      Expr::Path(p) =>
         if let Some(ident) = path_get_ident_mut(&mut p.path) {
            visitor(ident)
         },
      Expr::Range(r) => {
         if let Some(start) = &mut r.start {
            expr_visit_free_vars_mut(start, visitor)
         };
         if let Some(end) = &mut r.end {
            expr_visit_free_vars_mut(end, visitor)
         };
      },
      Expr::Reference(r) => visit!(r.expr),
      Expr::Repeat(r) => {
         visit!(r.expr);
         visit!(r.len)
      },
      Expr::Return(r) =>
         if let Some(e) = &mut r.expr {
            expr_visit_free_vars_mut(e, visitor)
         },
      Expr::Struct(s) => {
         for f in s.fields.iter_mut() {
            visit!(f.expr)
         }
         if let Some(rest) = &mut s.rest {
            expr_visit_free_vars_mut(rest.as_mut(), visitor)
         }
      },
      Expr::Try(t) => visit!(t.expr),
      Expr::TryBlock(t) => block_visit_free_vars_mut(&mut t.block, visitor),
      Expr::Tuple(t) =>
         for e in t.elems.iter_mut() {
            expr_visit_free_vars_mut(e, visitor)
         },
      Expr::Unary(u) => visit!(u.expr),
      Expr::Unsafe(u) => block_visit_free_vars_mut(&mut u.block, visitor),
      Expr::Verbatim(_) => {},
      Expr::While(w) => {
         let bound_vars = expr_get_let_bound_vars(&w.cond).into_iter().collect::<HashSet<_>>();
         visit!(w.cond);
         block_visit_free_vars_mut(&mut w.body, visitor_except!(bound_vars))
      },
      Expr::Yield(y) =>
         if let Some(e) = &mut y.expr {
            expr_visit_free_vars_mut(e.as_mut(), visitor)
         },
      _ => {},
   }
}

/// like `Path::get_ident(&self)`, but `mut`
pub fn path_get_ident_mut(path: &mut Path) -> Option<&mut Ident> {
   if path.segments.len() != 1 || path.leading_colon.is_some() {
      return None
   }
   let res = path.segments.first_mut()?;
   if res.arguments.is_empty() { Some(&mut res.ident) } else { None }
}

#[test]
fn test_expr_visit_free_vars_mut() {
   let test_cases = [
      (
         quote! {
            {
               let res = 0;
               for i in [0..10] {
                  let x = i + a;
                  res += x / {|m, (n, o)| m + n - o}(2, (b, 42))
               }
               res
            }
         },
         vec!["a", "b"],
      ),
      (
         quote! {
            |x1: u32, x2: u32| {
               if y > x1 {
                  if let Some(z) = foo(x1)  {
                     z + w
                  } else {
                     t = 42;
                     x2
                  }
               }
            }
         },
         vec!["y", "foo", "w", "t"],
      ),
   ];

   for (expr, expected) in test_cases {
      let mut expr = parse2(expr).unwrap();
      let mut result = HashSet::new();
      expr_visit_free_vars_mut(&mut expr, &mut |ident| {
         result.insert(ident.to_string());
      });
      let expected = expected.into_iter().map(|v| v.to_string()).collect::<HashSet<_>>();
      assert_eq!(result, expected)
   }
}

#[test]
fn mutable_visitor_preserves_statement_binding_scopes() {
   let mut expr = parse2(quote! {
      {
         let (x, y) = (x, seed);
         let Some(z) = candidate else { return fallback(z); };
         { let x = x + y; x + z + extra }
         x + y + z + extra
      }
   })
   .unwrap();
   expr_visit_free_vars_mut(&mut expr, &mut |ident| {
      *ident = Ident::new(&format!("free_{ident}"), ident.span());
   });
   let expected = quote! {
      {
         let (x, y) = (free_x, free_seed);
         let Some(z) = free_candidate else { return free_fallback(free_z); };
         { let x = x + y; x + z + free_extra }
         x + y + z + free_extra
      }
   };
   assert_eq!(expr.to_token_stream().to_string(), expected.to_string());
}

pub fn token_stream_replace_ident(ts: TokenStream, visitor: &mut dyn FnMut(&mut Ident)) -> TokenStream {
   fn token_tree_replace_ident(mut tt: TokenTree, visitor: &mut dyn FnMut(&mut Ident)) -> TokenTree {
      match tt {
         TokenTree::Group(grp) => {
            let updated_ts = token_stream_replace_ident(grp.stream(), visitor);
            let new_grp = Group::new(grp.delimiter(), updated_ts);
            TokenTree::Group(new_grp)
         },
         TokenTree::Ident(ref mut ident) => {
            visitor(ident);
            tt
         },
         TokenTree::Punct(_) => tt,
         TokenTree::Literal(_) => tt,
      }
   }

   let mut new_tts = vec![];
   for tt in ts.into_iter() {
      new_tts.push(token_tree_replace_ident(tt, visitor));
   }
   TokenStream::from_iter(new_tts)
}

pub fn token_stream_visit_idents(ts: TokenStream, visitor: &mut impl FnMut(&Ident)) {
   fn token_tree_visit_idents(mut tt: TokenTree, visitor: &mut impl FnMut(&Ident)) {
      match tt {
         TokenTree::Group(grp) => {
            token_stream_visit_idents(grp.stream(), visitor);
         },
         TokenTree::Ident(ref mut ident) => visitor(ident),
         TokenTree::Punct(_) => (),
         TokenTree::Literal(_) => (),
      }
   }

   for tt in ts.into_iter() {
      token_tree_visit_idents(tt, visitor);
   }
}

pub fn token_stream_idents(ts: TokenStream) -> Vec<Ident> {
   let mut res = vec![];
   token_stream_visit_idents(ts, &mut |ident| res.push(ident.clone()));
   res
}

pub fn expr_visit_macros_mut(expr: &mut Expr, visitor: &mut dyn FnMut(&mut ExprMacro)) {
   struct Visitor<'a>(&'a mut dyn FnMut(&mut ExprMacro));
   impl syn::visit_mut::VisitMut for Visitor<'_> {
      fn visit_expr_macro_mut(&mut self, node: &mut ExprMacro) { (self.0)(node) }
   }
   Visitor(visitor).visit_expr_mut(expr)
}

pub fn expr_visit_idents_in_macros_mut(expr: &mut Expr, visitor: &mut dyn FnMut(&mut Ident)) {
   let mut mac_visitor = |mac: &mut ExprMacro| {
      update(&mut mac.mac.tokens, |ts| token_stream_replace_ident(ts, visitor));
   };
   expr_visit_macros_mut(expr, &mut mac_visitor)
}

use std::collections::HashMap;

use proc_macro2::Span;
use syn::punctuated::Punctuated;

pub fn collect_set<T: Eq + std::hash::Hash>(iter: impl Iterator<Item = T>) -> HashSet<T> { iter.collect() }

pub fn into_set<T: Eq + std::hash::Hash>(iter: impl IntoIterator<Item = T>) -> HashSet<T> { iter.into_iter().collect() }

pub fn punctuated_map<T, P, U>(punc: Punctuated<T, P>, mut f: impl FnMut(T) -> U) -> Punctuated<U, P> {
   let mut res = Punctuated::new();
   for pair in punc.into_pairs() {
      let (t, p) = pair.into_tuple();
      res.push_value(f(t));
      if let Some(p) = p {
         res.push_punct(p)
      }
   }
   res
}

pub fn punctuated_try_map<T, P, U, E>(
   punc: Punctuated<T, P>, mut f: impl FnMut(T) -> Result<U, E>,
) -> Result<Punctuated<U, P>, E> {
   let mut res = Punctuated::new();
   for pair in punc.into_pairs() {
      let (t, p) = pair.into_tuple();
      res.push_value(f(t)?);
      if let Some(p) = p {
         res.push_punct(p)
      }
   }
   Ok(res)
}

pub fn flatten_punctuated<T, P>(punc: Punctuated<Punctuated<T, P>, P>) -> Punctuated<T, P> {
   let mut res = Punctuated::new();
   for inner_punc in punc.into_pairs() {
      let (inner_punc, p) = inner_punc.into_tuple();
      let inner_punc_len = inner_punc.len();
      for (ind, item) in inner_punc.into_pairs().enumerate() {
         let (t, p) = item.into_tuple();
         res.push_value(t);
         if ind != inner_punc_len - 1 {
            res.push_punct(p.unwrap())
         }
      }
      if let Some(p) = p {
         res.push_punct(p)
      }
   }
   res
}

pub fn punctuated_try_unwrap<T, P, E>(punc: Punctuated<Result<T, E>, P>) -> Result<Punctuated<T, P>, E> {
   let mut res = Punctuated::new();
   for pair in punc.into_pairs() {
      let (t, p) = pair.into_tuple();
      res.push_value(t?);
      if let Some(p) = p {
         res.push_punct(p)
      }
   }
   Ok(res)
}

pub fn punctuated_singleton<T, P>(item: T) -> Punctuated<T, P> {
   let mut res = Punctuated::new();
   res.push_value(item);
   res
}

pub fn expr_to_ident(expr: &Expr) -> Option<Ident> {
   match expr {
      Expr::Path(p) => p.path.get_ident().cloned(),
      _ => None,
   }
}
pub fn expr_to_ident_mut(expr: &mut Expr) -> Option<&mut Ident> {
   match expr {
      Expr::Path(p) => path_get_ident_mut(&mut p.path),
      _ => None,
   }
}

pub fn token_stream_replace_macro_idents(
   input: TokenStream, ident_replacements: &HashMap<Ident, TokenStream>,
) -> TokenStream {
   fn ts_replace(ts: TokenStream, ident_replacements: &HashMap<Ident, TokenStream>, res: &mut Vec<TokenTree>) {
      let mut last_dollar = None;
      for tt in ts {
         if let Some(dollar) = last_dollar.take() {
            let is_match = match &tt {
               TokenTree::Ident(after_dollar_ident) => ident_replacements.get(after_dollar_ident),
               _ => None,
            };
            if let Some(replacement) = is_match {
               res.extend(replacement.clone());
               continue;
            } else {
               res.push(dollar);
            }
         }
         let is_dollar = match &tt {
            TokenTree::Punct(punct) => punct.as_char() == '$',
            _ => false,
         };
         if is_dollar {
            last_dollar = Some(tt);
         } else {
            match tt {
               TokenTree::Group(grp) => {
                  let replaced = token_stream_replace_macro_idents(grp.stream(), ident_replacements);
                  let updated_group = Group::new(grp.delimiter(), replaced);
                  res.push(TokenTree::Group(updated_group));
               },
               _ => res.push(tt),
            }
         }
      }
      if let Some(dollar) = last_dollar {
         res.push(dollar);
      }
   }

   let mut res = vec![];
   ts_replace(input, ident_replacements, &mut res);

   res.into_iter().collect()
}

pub fn spans_eq(span1: &Span, span2: &Span) -> bool { format!("{:?}", span1) == format!("{:?}", span2) }

// I don't know why I'm like this
pub trait Piper: Sized {
   /// applies `f` to `self`, i.e., `f(self)`
   fn pipe<Res>(self, f: impl FnOnce(Self) -> Res) -> Res;
}

impl<T> Piper for T
where T: Sized
{
   #[inline(always)]
   fn pipe<Res>(self, f: impl FnOnce(Self) -> Res) -> Res { f(self) }
}

pub fn join_spans(spans: impl IntoIterator<Item = Span>) -> Span {
   let mut spans = spans.into_iter();
   let fst = spans.next().unwrap_or(Span::call_site());
   spans.try_fold(fst, |acc, next| acc.join(next)).unwrap_or(fst)
}

pub fn update<T: Default>(value: &mut T, f: impl FnOnce(T) -> T) { *value = f(std::mem::take(value)); }
