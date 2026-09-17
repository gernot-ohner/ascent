use derive_syn_parse::Parse;
use itertools::Either;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Attribute, Error, Expr, Generics, Ident, ImplGenerics, Pat, Path, Result, Token, Type, TypeGenerics, Visibility, WhereClause, braced, parenthesized};
use crate::syntax_utils::{pattern_get_vars, expr_to_ident};

mod kw {
   use derive_syn_parse::Parse;
   use proc_macro2::Span;
   use syn::Token;

   use crate::syntax_utils::join_spans;

   syn::custom_keyword!(relation);
   syn::custom_keyword!(lattice);
   #[allow(dead_code)] // for unused fields of LongLeftArrow
   #[derive(Parse)]
   pub struct LongLeftArrow(Token![<], Token![-], Token![-]);
   #[allow(unused)]
   impl LongLeftArrow {
      pub fn span(&self) -> Span { join_spans([self.0.span, self.1.span, self.2.span]) }
   }
   syn::custom_keyword!(agg);
   syn::custom_keyword!(ident);
   syn::custom_keyword!(expr);

   syn::custom_keyword!(include_source);
}

#[derive(Clone, Debug)]
pub(crate) struct Signatures {
   pub(crate) declaration: TypeSignature,
   pub(crate) implementation: Option<ImplSignature>,
}

impl Signatures {
   pub fn split_ty_generics_for_impl(&self) -> (ImplGenerics<'_>, TypeGenerics<'_>, Option<&'_ WhereClause>) {
      self.declaration.generics.split_for_impl()
   }

   pub fn split_impl_generics_for_impl(&self) -> (ImplGenerics<'_>, TypeGenerics<'_>, Option<&'_ WhereClause>) {
      let Some(signature) = &self.implementation else {
         return self.split_ty_generics_for_impl();
      };

      let (impl_generics, _, _) = signature.impl_generics.split_for_impl();
      let (_, ty_generics, where_clause) = signature.generics.split_for_impl();

      (impl_generics, ty_generics, where_clause)
   }
}

impl Parse for Signatures {
   fn parse(input: ParseStream) -> Result<Self> {
      let declaration = TypeSignature::parse(input)?;
      let implementation = if input.peek(Token![impl]) { Some(ImplSignature::parse(input)?) } else { None };
      Ok(Signatures { declaration, implementation })
   }
}

#[derive(Clone, Parse, Debug)]
pub struct TypeSignature {
   // We don't actually use the Parse impl to parse attrs.
   #[call(Attribute::parse_outer)]
   pub attrs: Vec<Attribute>,
   pub visibility: Visibility,
   pub _struct_kw: Token![struct],
   pub ident: Ident,
   #[call(parse_generics_with_where_clause)]
   pub generics: Generics,
   pub _semi: Token![;],
}

#[derive(Clone, Parse, Debug)]
pub struct ImplSignature {
   pub _impl_kw: Token![impl],
   pub impl_generics: Generics,
   pub ident: Ident,
   #[call(parse_generics_with_where_clause)]
   pub generics: Generics,
   pub _semi: Token![;],
}

/// Parse impl on Generics does not parse WhereClauses, hence this function
fn parse_generics_with_where_clause(input: ParseStream) -> Result<Generics> {
   let mut res = Generics::parse(input)?;
   if input.peek(Token![where]) {
      res.where_clause = Some(input.parse()?);
   }
   Ok(res)
}

#[derive(PartialEq, Eq, Clone)]
pub struct RelationNode {
   pub attrs: Vec<Attribute>,
   pub name: Ident,
   pub field_types: Punctuated<Type, Token![,]>,
   pub initialization: Option<Expr>,
   pub _semi_colon: Token![;],
   pub is_lattice: bool,
}

impl Parse for RelationNode {
   fn parse(input: ParseStream) -> Result<Self> {
      let is_lattice = input.peek(kw::lattice);
      if is_lattice {
         input.parse::<kw::lattice>()?;
      } else {
         input.parse::<kw::relation>()?;
      }
      let name: Ident = input.parse()?;
      let content;
      parenthesized!(content in input);
      let field_types = content.parse_terminated(Type::parse, Token![,])?;
      let initialization = if input.peek(Token![=]) {
         input.parse::<Token![=]>()?;
         Some(input.parse::<Expr>()?)
      } else {
         None
      };

      let _semi_colon = input.parse::<Token![;]>()?;
      if is_lattice && field_types.empty_or_trailing() {
         return Err(input.error("empty lattice is not allowed"));
      }
      Ok(RelationNode {
         attrs: vec![],
         name,
         field_types,
         _semi_colon,
         is_lattice,
         initialization,
      })
   }
}

#[derive(Parse, Clone)]
pub enum BodyItemNode {
   #[peek(Token![for], name = "generative clause")]
   Generator(GeneratorNode),
   #[peek(kw::agg, name = "aggregate clause")]
   Agg(AggClauseNode),
   #[peek_with(peek_macro_invocation, name = "macro invocation")]
   MacroInvocation(syn::ExprMacro),
   #[peek(Ident, name = "body clause")]
   Clause(BodyClauseNode),
   #[peek(Token![!], name = "negation clause")]
   Negation(NegationClauseNode),
   #[peek(syn::token::Paren, name = "disjunction node")]
   Disjunction(DisjunctionNode),
   #[peek_with(peek_if_or_let, name = "if condition or let binding")]
   Cond(CondClause),
}

fn peek_macro_invocation(parse_stream: ParseStream) -> bool {
   parse_stream.peek(Ident) && parse_stream.peek2(Token![!])
}

fn peek_if_or_let(parse_stream: ParseStream) -> bool { parse_stream.peek(Token![if]) || parse_stream.peek(Token![let]) }

#[derive(Parse, Clone)]
pub(crate) enum DisjunctionToken {
   #[allow(unused)]
   #[peek(Token![||], name = "||")]
   OrOr(Token![||]),
   #[allow(unused)]
   #[peek(Token![|], name = "|")]
   Or(Token![|]),
}

#[derive(Clone)]
pub struct DisjunctionNode {
   pub(crate) paren: syn::token::Paren,
   pub(crate) disjuncts: Punctuated<Punctuated<BodyItemNode, Token![,]>, DisjunctionToken>,
}

impl Parse for DisjunctionNode {
   fn parse(input: ParseStream) -> Result<Self> {
      let content;
      let paren = parenthesized!(content in input);
      let res: Punctuated<Punctuated<BodyItemNode, Token![,]>, DisjunctionToken> =
         Punctuated::<Punctuated<BodyItemNode, Token![,]>, DisjunctionToken>::parse_terminated_with(
            &content,
            Punctuated::<BodyItemNode, Token![,]>::parse_separated_nonempty,
         )?;
      if res.pairs().any(|pair| matches!(pair.punct(), Some(DisjunctionToken::OrOr(_)))) {
         eprintln!("WARNING: In Ascent rules, use `|` as the disjunction token instead of `||`")
      }
      Ok(DisjunctionNode { paren, disjuncts: res })
   }
}

#[derive(Parse, Clone)]
pub struct GeneratorNode {
   pub _for_keyword: Token![for],
   #[call(Pat::parse_multi)]
   pub pattern: Pat,
   pub _in_keyword: Token![in],
   pub expr: Expr,
}

#[derive(Clone)]
pub struct BodyClauseNode {
   pub rel: Ident,
   pub args: Punctuated<BodyClauseArg, Token![,]>,
   pub cond_clauses: Vec<CondClause>,
}

#[derive(Parse, Clone, PartialEq, Eq, Debug)]
pub enum BodyClauseArg {
   #[peek(Token![?], name = "Pattern arg")]
   Pat(ClauseArgPattern),
   #[peek_with({ |_| true }, name = "Expression arg")]
   Expr(Expr),
}

impl BodyClauseArg {
   pub fn get_vars(&self) -> Vec<Ident> {
      match self {
         BodyClauseArg::Pat(p) => pattern_get_vars(&p.pattern),
         BodyClauseArg::Expr(e) => expr_to_ident(e).into_iter().collect(),
      }
   }
}
impl ToTokens for BodyClauseArg {
   fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
      match self {
         BodyClauseArg::Pat(pat) => {
            pat.huh_token.to_tokens(tokens);
            pat.pattern.to_tokens(tokens);
         },
         BodyClauseArg::Expr(exp) => exp.to_tokens(tokens),
      }
   }
}

#[derive(Parse, Clone, PartialEq, Eq, Debug)]
pub struct ClauseArgPattern {
   pub huh_token: Token![?],
   #[call(Pat::parse_multi)]
   pub pattern: Pat,
}

#[derive(Parse, Clone, PartialEq, Eq, Hash, Debug)]
pub struct IfLetClause {
   pub if_keyword: Token![if],
   pub let_keyword: Token![let],
   #[call(Pat::parse_multi)]
   pub pattern: Pat,
   pub eq_symbol: Token![=],
   pub exp: syn::Expr,
}

#[derive(Parse, Clone, PartialEq, Eq, Hash, Debug)]
pub struct IfClause {
   pub if_keyword: Token![if],
   pub cond: Expr,
}

#[derive(Parse, Clone, PartialEq, Eq, Hash, Debug)]
pub struct LetClause {
   pub let_keyword: Token![let],
   #[call(Pat::parse_multi)]
   pub pattern: Pat,
   pub eq_symbol: Token![=],
   pub exp: syn::Expr,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum CondClause {
   IfLet(IfLetClause),
   If(IfClause),
   Let(LetClause),
}

impl CondClause {
   pub fn bound_vars(&self) -> Vec<Ident> {
      match self {
         CondClause::IfLet(cl) => pattern_get_vars(&cl.pattern),
         CondClause::If(_) => vec![],
         CondClause::Let(cl) => pattern_get_vars(&cl.pattern),
      }
   }


}
impl Parse for CondClause {
   fn parse(input: ParseStream) -> Result<Self> {
      if input.peek(Token![if]) {
         if input.peek2(Token![let]) {
            let cl: IfLetClause = input.parse()?;
            Ok(Self::IfLet(cl))
         } else {
            let cl: IfClause = input.parse()?;
            Ok(Self::If(cl))
         }
      } else if input.peek(Token![let]) {
         let cl: LetClause = input.parse()?;
         Ok(Self::Let(cl))
      } else {
         Err(input.error("expected either if clause or if let clause"))
      }
   }
}

// impl ToTokens for BodyClauseNode {
//    fn to_tokens(&self, tokens: &mut quote::__private::TokenStream) {
//       self.rel.to_tokens(tokens);
//       self.args.to_tokens(tokens);
//    }
// }

impl Parse for BodyClauseNode {
   fn parse(input: ParseStream) -> Result<Self> {
      let rel: Ident = input.parse()?;
      let args_content;
      parenthesized!(args_content in input);
      let args = args_content.parse_terminated(BodyClauseArg::parse, Token![,])?;
      let mut cond_clauses = vec![];
      while let Ok(cl) = input.parse() {
         cond_clauses.push(cl);
      }
      Ok(BodyClauseNode { rel, args, cond_clauses })
   }
}

#[derive(Parse, Clone)]
pub struct NegationClauseNode {
   _neg_token: Token![!],
   pub rel: Ident,
   #[paren]
   _rel_arg_paren: syn::token::Paren,
   #[inside(_rel_arg_paren)]
   #[call(Punctuated::parse_terminated)]
   pub args: Punctuated<Expr, Token![,]>,
}

#[derive(Clone, Parse)]
pub enum HeadItemNode {
   #[peek_with(peek_macro_invocation, name = "macro invocation")]
   MacroInvocation(syn::ExprMacro),
   #[peek(Ident, name = "head clause")]
   HeadClause(HeadClauseNode),
}


#[derive(Clone)]
pub struct HeadClauseNode {
   pub rel: Ident,
   pub args: Punctuated<Expr, Token![,]>,
}
impl ToTokens for HeadClauseNode {
   fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
      let rel = &self.rel;
      let args = &self.args;
      tokens.extend(quote! { #rel(#args) });
   }
}

impl Parse for HeadClauseNode {
   fn parse(input: ParseStream) -> Result<Self> {
      let rel: Ident = input.parse()?;
      let args_content;
      parenthesized!(args_content in input);
      let args = args_content.parse_terminated(Expr::parse, Token![,])?;
      Ok(HeadClauseNode { rel, args })
   }
}

#[derive(Clone, Parse)]
pub struct AggClauseNode {
   pub agg_kw: kw::agg,
   #[call(Pat::parse_multi)]
   pub pat: Pat,
   pub _eq_token: Token![=],
   pub aggregator: AggregatorNode,
   #[paren]
   pub _agg_arg_paren: syn::token::Paren,
   #[inside(_agg_arg_paren)]
   #[call(Punctuated::parse_terminated)]
   pub bound_args: Punctuated<Ident, Token![,]>,
   pub _in_kw: Token![in],
   pub rel: Ident,
   #[paren]
   _rel_arg_paren: syn::token::Paren,
   #[inside(_rel_arg_paren)]
   #[call(Punctuated::parse_terminated)]
   pub rel_args: Punctuated<Expr, Token![,]>,
}

#[derive(Clone)]
pub enum AggregatorNode {
   Path(syn::Path),
   Expr(Expr),
}

impl Parse for AggregatorNode {
   fn parse(input: ParseStream) -> Result<Self> {
      if input.peek(syn::token::Paren) {
         let inside_parens;
         parenthesized!(inside_parens in input);
         Ok(AggregatorNode::Expr(inside_parens.parse()?))
      } else {
         Ok(AggregatorNode::Path(input.parse()?))
      }
   }
}

pub struct RuleNode {
   pub head_clauses: Punctuated<HeadItemNode, Token![,]>,
   pub body_items: Vec<BodyItemNode>, // Punctuated<BodyItemNode, Token![,]>,
}

impl Parse for RuleNode {
   fn parse(input: ParseStream) -> Result<Self> {
      let head_clauses = if input.peek(syn::token::Brace) {
         let content;
         braced!(content in input);
         Punctuated::<HeadItemNode, Token![,]>::parse_terminated(&content)?
      } else {
         Punctuated::<HeadItemNode, Token![,]>::parse_separated_nonempty(input)?
      };

      if input.peek(Token![;]) {
         input.parse::<Token![;]>()?;
         Ok(RuleNode { head_clauses, body_items: vec![] /*Punctuated::default()*/ })
      } else {
         input.parse::<kw::LongLeftArrow>()?;
         let body_items = Punctuated::<BodyItemNode, Token![,]>::parse_separated_nonempty(input)?;
         input.parse::<Token![;]>()?;
         Ok(RuleNode { head_clauses, body_items: body_items.into_iter().collect() })
      }
   }
}

#[derive(Parse)]
pub struct MacroDefParam {
   _dollar: Token![$],
   pub(crate) name: Ident,
   _colon: Token![:],
   pub(crate) kind: MacroParamKind,
}

#[derive(Parse)]
#[allow(unused)]
pub enum MacroParamKind {
   #[peek(kw::ident, name = "ident")]
   Expr(Ident),
   #[peek(kw::expr, name = "expr")]
   Ident(Ident),
}

#[derive(Parse)]
pub struct MacroDefNode {
   _mac: Token![macro],
   pub(crate) name: Ident,
   #[paren]
   _arg_paren: syn::token::Paren,
   #[inside(_arg_paren)]
   #[call(Punctuated::parse_terminated)]
   pub(crate) params: Punctuated<MacroDefParam, Token![,]>,
   #[brace]
   _body_brace: syn::token::Brace,
   #[inside(_body_brace)]
   pub(crate) body: TokenStream,
}

#[derive(Parse)]
pub struct IncludeSourceNode {
   pub _include_source_kw: kw::include_source,
   _bang: Token![!],
   #[paren]
   _arg_paren: syn::token::Paren,
   #[inside(_arg_paren)]
   #[call(syn::Path::parse_mod_style)]
   path: syn::Path,
   _semi: Token![;],
}

// #[derive(Clone)]
pub(crate) struct AscentProgram {
   pub rules: Vec<RuleNode>,
   pub relations: Vec<RelationNode>,
   pub signatures: Option<Signatures>,
   pub attributes: Vec<syn::Attribute>,
   pub macros: Vec<MacroDefNode>,
}

/// The output that should be emitted when an `include_source!()` is encountered
pub(crate) struct IncludeSourceMacroCall {
   /// the encountered `include_source!()`
   pub include_node: IncludeSourceNode,
   pub before_tokens: TokenStream,
   pub after_tokens: TokenStream,
   pub ascent_macro_name: Path,
}

impl IncludeSourceMacroCall {
   /// The output that should be emitted
   pub fn macro_call_output(&self) -> TokenStream {
      let Self { include_node, before_tokens, after_tokens, ascent_macro_name } = self;
      let include_macro_callback = &include_node.path;
      quote_spanned! {include_macro_callback.span()=>
         #include_macro_callback! { {#ascent_macro_name}, {#before_tokens}, {#after_tokens} }
      }
   }
}

pub(crate) fn parse_ascent_program(
   input: ParseStream, ascent_macro_name: Path,
) -> Result<Either<AscentProgram, IncludeSourceMacroCall>> {
   let input_clone = input.cursor();
   let attributes = Attribute::parse_inner(input)?;
   let mut struct_attrs = Attribute::parse_outer(input)?;
   let signatures = if input.peek(Token![pub]) || input.peek(Token![struct]) {
      let mut signatures = Signatures::parse(input)?;
      signatures.declaration.attrs = std::mem::take(&mut struct_attrs);
      Some(signatures)
   } else {
      None
   };
   let mut rules = vec![];
   let mut relations = vec![];
   let mut macros = vec![];
   while !input.is_empty() {
      let attrs =
         if !struct_attrs.is_empty() { std::mem::take(&mut struct_attrs) } else { Attribute::parse_outer(input)? };
      if input.peek(kw::relation) || input.peek(kw::lattice) {
         let mut relation_node = RelationNode::parse(input)?;
         relation_node.attrs = attrs;
         relations.push(relation_node);
      } else if input.peek(Token![macro]) {
         if !attrs.is_empty() {
            return Err(Error::new(attrs[0].span(), "unexpected attribute(s)"));
         }
         macros.push(MacroDefNode::parse(input)?);
      } else if input.peek(kw::include_source) {
         if !attrs.is_empty() {
            return Err(Error::new(attrs[0].span(), "unexpected attribute(s)"));
         }
         // Cursor identity works even when several tokens share a call-site span.
         let mut cursor = input_clone;
         let mut before_tokens = TokenStream::new();
         while cursor != input.cursor() {
            let (token, next) = cursor.token_tree().ok_or_else(|| input.error("invalid source boundary"))?;
            before_tokens.extend([token]);
            cursor = next;
         }
         let include_node = IncludeSourceNode::parse(input)?;
         let after_tokens: TokenStream = input.parse()?;
         let include_source_macro_call =
            IncludeSourceMacroCall { include_node, before_tokens, after_tokens, ascent_macro_name };
         return Ok(Either::Right(include_source_macro_call));
      } else {
         if !attrs.is_empty() {
            return Err(Error::new(attrs[0].span(), "unexpected attribute(s)"));
         }
         rules.push(RuleNode::parse(input)?);
      }
   }
   Ok(Either::Left(AscentProgram { rules, relations, signatures, attributes, macros }))
}


pub(crate) enum Parsed {
    Program(AscentProgram),
    Include(TokenStream),
}

pub(crate) fn emit_program(program: &AscentProgram) -> TokenStream {
    let attrs = &program.attributes;
    let signature = program.signatures.as_ref().map(|s| {
        let d = &s.declaration;
        let (attrs, vis, name, generics, bounds) =
            (&d.attrs, &d.visibility, &d.ident, &d.generics, &d.generics.where_clause);
        let implementation = s.implementation.as_ref().map(|i| {
            let (ig, name, tg, bounds) = (&i.impl_generics, &i.ident, &i.generics, &i.generics.where_clause);
            quote! { impl #ig #name #tg #bounds; }
        });
        quote! { #(#attrs)* #vis struct #name #generics #bounds; #implementation }
    });
    let relations = program.relations.iter().map(|r| {
        let (attrs, name, types) = (&r.attrs, &r.name, &r.field_types);
        let kind = if r.is_lattice { quote!(lattice) } else { quote!(relation) };
        let init = r.initialization.as_ref().map(|e| quote!(= #e));
        quote! { #(#attrs)* #kind #name(#types) #init; }
    });
    let macros = program.macros.iter().map(|m| {
        let (name, body) = (&m.name, &m.body);
        let params = m.params.iter().map(|p| {
            let name = &p.name;
            let kind = match &p.kind { MacroParamKind::Expr(k) | MacroParamKind::Ident(k) => k };
            quote! { $ #name : #kind }
        });
        quote! { macro #name(#(#params),*) { #body } }
    });
    let rules = program.rules.iter().map(|r| {
        let heads = r.head_clauses.iter().map(|h| match h {
            HeadItemNode::HeadClause(c) => c.to_token_stream(),
            HeadItemNode::MacroInvocation(m) => m.to_token_stream(),
        });
        if r.body_items.is_empty() {
            quote! { {#(#heads),*}; }
        } else {
            let body = r.body_items.iter().map(emit_body);
            quote! { {#(#heads),*} <-- #(#body),*; }
        }
    });
    quote! { #(#attrs)* #signature #(#relations)* #(#macros)* #(#rules)* }
}

fn emit_condition(c: &CondClause) -> TokenStream {
    match c {
        CondClause::If(c) => { let e = &c.cond; quote!(if #e) }
        CondClause::IfLet(c) => { let (p,e) = (&c.pattern,&c.exp); quote!(if let #p = #e) }
        CondClause::Let(c) => { let (p,e) = (&c.pattern,&c.exp); quote!(let #p = #e) }
    }
}

fn emit_body(b: &BodyItemNode) -> TokenStream {
    match b {
        BodyItemNode::Clause(c) => {
            let (name,args) = (&c.rel,&c.args);
            let conditions = c.cond_clauses.iter().map(emit_condition);
            quote!(#name(#args) #(#conditions)*)
        }
        BodyItemNode::Generator(g) => {
            let (p,e) = (&g.pattern,&g.expr); quote!(for #p in #e)
        }
        BodyItemNode::Cond(c) => emit_condition(c),
        BodyItemNode::Negation(n) => { let (r,a) = (&n.rel,&n.args); quote!(!#r(#a)) }
        BodyItemNode::Agg(a) => {
            let (p,b,r,args) = (&a.pat,&a.bound_args,&a.rel,&a.rel_args);
            let f = match &a.aggregator {
                AggregatorNode::Path(p) => quote!(#p),
                AggregatorNode::Expr(e) => quote!((#e)),
            };
            quote!(agg #p = #f(#b) in #r(#args))
        }
        BodyItemNode::MacroInvocation(m) => m.to_token_stream(),
        BodyItemNode::Disjunction(d) => {
            let branches = d.disjuncts.iter().map(|items| {
                let items = items.iter().map(emit_body); quote!(#(#items),*)
            });
            quote!((#(#branches)|*))
        }
    }
}
pub(crate) fn parse_program(input: TokenStream, callback: Path) -> Result<Parsed> {
    (|input: ParseStream| match parse_ascent_program(input, callback.clone())? {
        Either::Left(program) => Ok(Parsed::Program(program)),
        Either::Right(include) => Ok(Parsed::Include(include.macro_call_output())),
    }).parse2(input)
}
