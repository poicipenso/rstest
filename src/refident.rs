/// Provide `RefIdent` and `MaybeIdent` traits that give a shortcut to extract identity reference
/// (`syn::Ident` struct).

use proc_macro2::Ident;
use syn::{FnArg, PatType, Pat};

pub trait RefIdent {
    /// Return the reference to ident if any
    fn ident(&self) -> &Ident;
}

pub trait MaybeIdent {
    /// Return the reference to ident if any
    fn maybe_ident(&self) -> Option<&Ident>;
}

impl <I: RefIdent> MaybeIdent for I {
    fn maybe_ident(&self) -> Option<&Ident> {
        Some(self.ident())
    }
}

impl RefIdent for Ident {
    fn ident(&self) -> &Ident {
        self
    }
}

impl<'a> RefIdent for &'a Ident {
    fn ident(&self) -> &Ident {
        *self
    }
}

impl MaybeIdent for FnArg {
    fn maybe_ident(&self) -> Option<&Ident> {
        match self {
            FnArg::Typed(PatType { pat, .. }) =>
                match pat.as_ref() {
                    Pat::Ident(ident) => Some(&ident.ident),
                    _ => None
                },
            _ => None
        }
    }
}

impl MaybeIdent for syn::Type {
    fn maybe_ident(&self) -> Option<&Ident> {
        match self {
            syn::Type::Path(tp) if tp.qself.is_none()=> {
                tp.path.get_ident()
            }
            _ => None
        }
    }
}
