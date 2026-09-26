//! Keep the user's named program as the meaning of `Self` after engine renaming.
use quote::ToTokens;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, Item, Macro, Path, Token};

use crate::lower::LoweredProgram;
use crate::syntax::{AggregatorNode, BodyClauseArg, BodyItemNode, CondClause, HeadItemNode};

pub(crate) fn preserve_self(lowered: &mut LoweredProgram, public_type: Path) -> syn::Result<()> {
    let mut visitor = PublicSelf {
        public_type,
        error: None,
    };
    for relation in &mut lowered.program.relations {
        for ty in &mut relation.field_types {
            visitor.visit_type_mut(ty);
        }
        // Initializers execute in the wrapper, where Self already has its meaning.
    }
    for annotation in &mut lowered.annotated {
        for ty in &mut annotation.key_types {
            visitor.visit_type_mut(ty);
        }
        visitor.visit_type_mut(&mut annotation.annotation_type);
    }
    for rule in &mut lowered.program.rules {
        for head in &mut rule.head_clauses {
            if let HeadItemNode::HeadClause(head) = head {
                for expr in &mut head.args {
                    visitor.visit_expr_mut(expr);
                }
            }
        }
        for body in &mut rule.body_items {
            visitor.body(body);
        }
    }
    match visitor.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

struct PublicSelf {
    public_type: Path,
    error: Option<syn::Error>,
}

impl PublicSelf {
    fn condition(&mut self, condition: &mut CondClause) {
        match condition {
            CondClause::If(cl) => self.visit_expr_mut(&mut cl.cond),
            CondClause::IfLet(cl) => {
                self.visit_pat_mut(&mut cl.pattern);
                self.visit_expr_mut(&mut cl.exp);
            }
            CondClause::Let(cl) => {
                self.visit_pat_mut(&mut cl.pattern);
                self.visit_expr_mut(&mut cl.exp);
            }
        }
    }

    fn body(&mut self, body: &mut BodyItemNode) {
        match body {
            BodyItemNode::Generator(cl) => {
                self.visit_pat_mut(&mut cl.pattern);
                self.visit_expr_mut(&mut cl.expr);
            }
            BodyItemNode::Clause(cl) => {
                for arg in &mut cl.args {
                    match arg {
                        BodyClauseArg::Expr(expr) => self.visit_expr_mut(expr),
                        BodyClauseArg::Pat(pat) => self.visit_pat_mut(&mut pat.pattern),
                    }
                }
                for condition in &mut cl.cond_clauses {
                    self.condition(condition);
                }
            }
            BodyItemNode::Cond(cl) => self.condition(cl),
            BodyItemNode::Negation(cl) => {
                for expr in &mut cl.args {
                    self.visit_expr_mut(expr);
                }
            }
            BodyItemNode::Agg(cl) => {
                self.visit_pat_mut(&mut cl.pat);
                for expr in &mut cl.rel_args {
                    self.visit_expr_mut(expr);
                }
                match &mut cl.aggregator {
                    AggregatorNode::Path(path) => self.visit_path_mut(path),
                    AggregatorNode::Expr(expr) => self.visit_expr_mut(expr),
                }
            }
            BodyItemNode::Disjunction(_) | BodyItemNode::MacroInvocation(_) => {
                unreachable!("Ascent macros and disjunctions have already expanded")
            }
        }
    }
}

impl VisitMut for PublicSelf {
    fn visit_path_mut(&mut self, path: &mut Path) {
        visit_mut::visit_path_mut(self, path);
        if path.leading_colon.is_none() && path.segments.first().is_some_and(|s| s.ident == "Self")
        {
            let mut replacement = self.public_type.clone();
            replacement
                .segments
                .extend(path.segments.iter().skip(1).cloned());
            *path = replacement;
        }
    }

    // Nested items have their own Self scope, or cannot capture the outer Self.
    fn visit_item_mut(&mut self, _: &mut Item) {}

    fn visit_macro_mut(&mut self, mac: &mut Macro) {
        if !crate::syntax_utils::token_stream_idents(mac.tokens.clone())
            .iter()
            .any(|i| i == "Self")
        {
            return;
        }
        let standard_path = mac.path.segments.len() == 1
            || (mac.path.segments.len() == 2
                && matches!(
                    mac.path.segments[0].ident.to_string().as_str(),
                    "std" | "core"
                ));
        let name = mac.path.segments.last().unwrap().ident.to_string();
        // stringify consumes literal tokens, not a Rust type or expression.
        if standard_path && name == "stringify" {
            return;
        }
        let expression_macro = standard_path
            && matches!(
                name.as_str(),
                "vec"
                    | "assert"
                    | "assert_eq"
                    | "assert_ne"
                    | "debug_assert"
                    | "debug_assert_eq"
                    | "debug_assert_ne"
                    | "format"
                    | "format_args"
                    | "print"
                    | "println"
                    | "eprint"
                    | "eprintln"
                    | "write"
                    | "writeln"
                    | "panic"
                    | "dbg"
            );
        if !expression_macro {
            self.error = Some(syn::Error::new_spanned(
                mac,
                "cannot preserve Self in this macro's custom syntax; use the explicit program type",
            ));
            return;
        }
        // Parsing succeeds for token-based custom macros too. Only known
        // expression macros may use this path; never infer a macro's grammar.
        if let Ok(mut expressions) =
            Punctuated::<Expr, Token![,]>::parse_terminated.parse2(mac.tokens.clone())
        {
            for expression in &mut expressions {
                self.visit_expr_mut(expression);
            }
            mac.tokens = expressions.into_token_stream();
        } else if name == "vec" {
            let tokens = &mac.tokens;
            if let Ok(mut repeat) = syn::parse2::<syn::ExprRepeat>(quote!([#tokens])) {
                self.visit_expr_repeat_mut(&mut repeat);
                let (expr, len) = (&repeat.expr, &repeat.len);
                mac.tokens = quote!(#expr; #len);
                return;
            }
            self.error = Some(syn::Error::new_spanned(
                mac,
                "cannot preserve Self in this macro; use the explicit program type",
            ));
        } else {
            self.error = Some(syn::Error::new_spanned(
                mac,
                "cannot preserve Self in this macro's custom syntax; use the explicit program type",
            ));
        }
    }
}
