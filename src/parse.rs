use syn::{
    Expr, Ident, Lit, LitStr, Meta, NestedMeta,
    parse::{Parse, ParseStream, Result, Error},
    spanned::Spanned,
    punctuated::{Punctuated},
    Token,
    token
};
use proc_macro2::{TokenStream, Span};
use crate::{error::error, Tokenize};
// TODO: Remove this dependency
use quote::quote;
use quote::ToTokens;

#[derive(Default, Debug)]
pub struct ParametrizeData {
    pub args: Vec<Ident>,
    pub cases: Vec<TestCase>,
}

#[derive(Default, Debug)]
/// Parametrize
pub struct ParametrizeInfo {
    pub data: ParametrizeData,
    pub modifier: Modifiers,
}

#[derive(Debug)]
/// A test case instance data. Contains a list of arguments. It is parsed by parametrize
/// attributes.
pub struct TestCase {
    pub args: Punctuated<CaseArg, Token![,]>,
    pub description: Option<Ident>
}

impl TestCase {
    pub fn span_start(&self) -> Span {
        self.args.first().map(|arg| arg.span()).unwrap_or(Span::call_site())
    }
    pub fn span_end(&self) -> Span {
        self.args.last().map(|arg| arg.span()).unwrap_or(Span::call_site())
    }
}

#[derive(Debug, Clone)]
/// A test case's argument as an expression that can be assigned.
pub struct CaseArg {
    expr: Expr,
}

impl CaseArg {
    pub fn new(expr: Expr) -> Self {
        Self { expr }
    }
}

#[cfg(test)]
impl PartialEq for CaseArg {
    fn eq(&self, other: &Self) -> bool {
        self.expr == other.expr
    }
}

impl ToTokens for CaseArg {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.expr.to_tokens(tokens);
    }
}

impl From<Expr> for CaseArg {
    fn from(expr: Expr) -> Self {
        CaseArg::new(expr)
    }
}

impl Parse for TestCase {
    fn parse(input: ParseStream) -> Result<Self> {
        let case: Ident = input.parse()?;
        if case == "case" {
            let content;
            let _ = syn::parenthesized!(content in input);
            let args = content.parse_terminated(CaseArg::parse)?;
            Ok(TestCase { args, description: None })
        } else {
            Err(Error::new(case.span(), "expected a test case"))
        }
    }
}

fn is_arbitrary_rust_code(ident: &Ident) -> bool {
    ["Unwrap", "r"].iter().any(|&n| ident == n)
}

fn nested_meta_literal_str(nested: &NestedMeta) -> Option<&LitStr> {
    match nested {
        NestedMeta::Literal(Lit::Str(lit)) => Some(lit),
        _ => None
    }
}

fn compile_lit_str(lit: &LitStr) -> Result<TokenStream> {
    lit.parse::<Expr>()
        .map(|e| quote! { #e })
        .or_else(|e| Err(Error::new(
            lit.span(),
            &format!("Cannot parse '{}' due {}", lit.value(), e),
        ))
        )
}

impl Parse for CaseArg {
    fn parse(input: ParseStream) -> Result<Self> {
        let nested: NestedMeta = input.parse()?;
        let tokens = match nested {
            NestedMeta::Literal(_) | NestedMeta::Meta(Meta::Word(_)) =>
                nested.clone().into_tokens(),
            NestedMeta::Meta(Meta::List(ref arg)) if is_arbitrary_rust_code(&arg.ident) => {
                arg.nested.first()
                    .map(|m| *m.value())
                    .and_then(nested_meta_literal_str)
                    .map(compile_lit_str)
                    .unwrap_or_else(
                        || Ok(error(&format!("Invalid {} argument", arg.ident), nested.span(), nested.span()))
                    )?
            }
            NestedMeta::Meta(Meta::List(arg)) =>
                error(&format!("Invalid case argument: `{}`", arg.ident), arg.span(), arg.span()),
            NestedMeta::Meta(nested) =>
                error(&format!("Unexpected case argument: {:?}", nested), nested.span(), nested.span())
        };
        Ok(CaseArg::new(syn::parse2(tokens)?))
    }
}

#[derive(Default, Debug)]
pub struct Modifiers {
    pub modifiers: Vec<RsTestAttribute>
}

impl Parse for Modifiers {
    fn parse(input: ParseStream) -> Result<Self> {
        let vars = Punctuated::<RsTestAttribute, Token![::]>::parse_terminated(input)?;
        Ok(Modifiers {
            modifiers: vars.into_iter().collect(),
        })
    }
}

#[derive(Debug)]
pub enum RsTestAttribute {
    Attr(Ident),
    Tagged(Ident, Vec<Ident>),
}

fn no_literal_nested(nested: NestedMeta) -> Result<Meta> {
    match nested {
        NestedMeta::Meta(m) => Ok(m),
        NestedMeta::Literal(l) => Err(Error::new(l.span(), "Unexpected literal"))
    }
}

fn just_word_meta(meta: Meta) -> Result<Ident> {
    match meta {
        Meta::Word(ident) => Ok(ident),
        other => Err(Error::new(other.span(), "Should be an ident"))
    }
}

impl Parse for RsTestAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        let meta = no_literal_nested(NestedMeta::parse(input)?)?;
        use Meta::*;
        match meta {
            Word(ident) => Ok(RsTestAttribute::Attr(ident)),
            List(l) =>
                Ok(RsTestAttribute::Tagged(l.ident,
                                           l.nested.into_iter()
                                               .map(no_literal_nested)
                                               .collect::<Result<Vec<Meta>>>()?
                                               .into_iter().map(just_word_meta)
                                               .collect::<Result<Vec<Ident>>>()?,
                )),
            NameValue(nv) => Err(Error::new(nv.span(), "Invalid attribute"))
        }
    }
}

impl Parse for ParametrizeData {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = vec![];
        loop {
            if input.peek2(token::Paren) {
                break;
            }
            if let Ok(arg) = input.parse() {
                args.push(arg);
                if input.parse::<Token![,]>().is_err() {
                    break;
                }
            } else {
                break;
            }
        }

        let cases: Vec<_> =
            Punctuated::<TestCase, Token![,]>::parse_separated_nonempty(input)?
                .into_iter().collect();
        Ok(ParametrizeData {
            args,
            cases,
        })
    }
}

impl Parse for ParametrizeInfo {
    fn parse(input: ParseStream) -> Result<Self> {
        // Parse a `ParametrizeData` and then look for `Modifiers` after `::` token if any.
        // Example
        //  |-    u,a,d
        //  |-    case(42, r("A{}"), Unwrap("D{}"))
        //  |  ::trace::notrace(a))
        //  |    ^^^^^^^^^^^^^^^^^^ Modifiers
        //  |______________________ ParametrizeData
        Ok(
            ParametrizeInfo {
                data: input.parse()?,
                modifier: input.parse::<Token![::]>()
                    .or_else(|_| Ok(Default::default()))
                    .and_then(|_| input.parse())?,
            }
        )
    }
}

