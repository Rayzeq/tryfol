use std::ops::{Deref, DerefMut};
use syn::{
    parse_quote_spanned, punctuated::Punctuated, spanned::Spanned, AngleBracketedGenericArguments,
    AssocType, GenericArgument, Ident, ItemTrait, Path, PathArguments, PathSegment, ReturnType,
    Token, TraitBound, TraitItem, TraitItemFn, Type, TypeImplTrait, TypeParamBound, TypePath,
};

pub trait IdentEditor {
    fn with_name(&self, name: &str) -> Self;
}

pub trait TraitEditor {
    fn methods(&self) -> impl Iterator<Item = &TraitItemFn>;
    fn methods_mut(&mut self) -> impl Iterator<Item = &mut TraitItemFn>;

    fn set_name(&mut self, name: Ident);
}

pub trait FunctionEditor {
    fn add_async_send_bound(&mut self);
}

pub trait GenericsEditor {
    fn types(&self) -> impl Iterator<Item = &Type>;
    fn types_mut(&mut self) -> impl Iterator<Item = &mut Type>;

    fn enumerate_types(&mut self) -> impl Iterator<Item = (usize, &mut Type)>;

    fn associated_types(&self) -> impl Iterator<Item = &AssocType>;
    fn associated_types_mut(&mut self) -> impl Iterator<Item = &mut AssocType>;
}

pub trait ParseType {
    fn as_result(&self) -> Option<TypeResult<&Path>>;
    fn as_result_mut(&mut self) -> Option<TypeResult<&mut Path>>;

    fn as_stream(&self) -> Option<TypeStream<&Punctuated<GenericArgument, Token![,]>>>;
    fn as_stream_mut(&mut self) -> Option<TypeStream<&mut Punctuated<GenericArgument, Token![,]>>>;
}

pub struct TypeResult<T: Deref<Target = Path>> {
    path: T,
}

pub struct TypeStream<T: Deref<Target = Punctuated<GenericArgument, Token![,]>>> {
    args: T,
}

impl IdentEditor for Ident {
    fn with_name(&self, name: &str) -> Self {
        Self::new(name, self.span())
    }
}

impl TraitEditor for ItemTrait {
    fn methods(&self) -> impl Iterator<Item = &TraitItemFn> {
        self.items.iter().filter_map(|item| {
            if let TraitItem::Fn(func) = item {
                Some(func)
            } else {
                None
            }
        })
    }

    fn methods_mut(&mut self) -> impl Iterator<Item = &mut TraitItemFn> {
        self.items.iter_mut().filter_map(|item| {
            if let TraitItem::Fn(func) = item {
                Some(func)
            } else {
                None
            }
        })
    }

    fn set_name(&mut self, name: Ident) {
        self.ident = name;
    }
}

impl FunctionEditor for TraitItemFn {
    fn add_async_send_bound(&mut self) {
        if self.sig.asyncness.is_some() {
            let asyncness = self.sig.asyncness;
            match &self.sig.output {
                ReturnType::Default => {
                    self.sig.output = parse_quote_spanned!(asyncness.span()=> => impl ::core::future::Future<Output=()> + ::core::marker::Send );
                }
                ReturnType::Type(arrow, ty) => {
                    self.sig.output = parse_quote_spanned!(asyncness.span()=> #arrow impl ::core::future::Future<Output=#ty> + ::core::marker::Send );
                }
            }

            self.sig.asyncness = None;
        }
    }
}

impl ParseType for Type {
    fn as_result(&self) -> Option<TypeResult<&Path>> {
        if let Self::Path(TypePath { path, .. }) = self {
            if let Some(segment) = path.segments.last() {
                if segment.ident == "Result" {
                    if let PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                        args,
                        ..
                    }) = &segment.arguments
                    {
                        let types_count = args.types().count();
                        if types_count == 1 || types_count == 2 {
                            return Some(TypeResult { path });
                        }
                    }
                }
            }
        }

        None
    }

    fn as_result_mut(&mut self) -> Option<TypeResult<&mut Path>> {
        if let Self::Path(TypePath { path, .. }) = self {
            if let Some(segment) = path.segments.last_mut() {
                if segment.ident == "Result" {
                    if let PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                        args,
                        ..
                    }) = &mut segment.arguments
                    {
                        let types_count = args.types().count();
                        if types_count == 1 || types_count == 2 {
                            return Some(TypeResult { path });
                        }
                    }
                }
            }
        }

        None
    }

    fn as_stream(&self) -> Option<TypeStream<&Punctuated<GenericArgument, Token![,]>>> {
        if let Self::ImplTrait(TypeImplTrait { bounds, .. }) = self {
            for bound in bounds {
                if let TypeParamBound::Trait(TraitBound { path, .. }) = bound {
                    if let Some(PathSegment { ident, arguments }) = path.segments.last() {
                        if ident == "Stream" {
                            if let PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                                args,
                                ..
                            }) = arguments
                            {
                                if args.associated_types().any(|ty| ty.ident == "Item") {
                                    return Some(TypeStream { args });
                                }
                            };
                        }
                    };
                };
            }
        }

        None
    }

    fn as_stream_mut(&mut self) -> Option<TypeStream<&mut Punctuated<GenericArgument, Token![,]>>> {
        if let Self::ImplTrait(TypeImplTrait { bounds, .. }) = self {
            for bound in bounds {
                if let TypeParamBound::Trait(TraitBound { path, .. }) = bound {
                    if let Some(PathSegment { ident, arguments }) = path.segments.last_mut() {
                        if ident == "Stream" {
                            if let PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                                args,
                                ..
                            }) = arguments
                            {
                                if args.associated_types().any(|ty| ty.ident == "Item") {
                                    return Some(TypeStream { args });
                                }
                            };
                        }
                    };
                };
            }
        }

        None
    }
}

impl ParseType for ReturnType {
    fn as_result(&self) -> Option<TypeResult<&Path>> {
        match self {
            Self::Default => None,
            Self::Type(_, ty) => ty.as_result(),
        }
    }

    fn as_result_mut(&mut self) -> Option<TypeResult<&mut Path>> {
        match self {
            Self::Default => None,
            Self::Type(_, ty) => ty.as_result_mut(),
        }
    }

    fn as_stream(&self) -> Option<TypeStream<&Punctuated<GenericArgument, Token![,]>>> {
        match self {
            Self::Default => None,
            Self::Type(_, ty) => ty.as_stream(),
        }
    }

    fn as_stream_mut(&mut self) -> Option<TypeStream<&mut Punctuated<GenericArgument, Token![,]>>> {
        match self {
            Self::Default => None,
            Self::Type(_, ty) => ty.as_stream_mut(),
        }
    }
}

impl GenericsEditor for Punctuated<GenericArgument, Token![,]> {
    fn types(&self) -> impl Iterator<Item = &Type> {
        self.iter().filter_map(|arg| {
            if let GenericArgument::Type(ty) = arg {
                Some(ty)
            } else {
                None
            }
        })
    }

    fn types_mut(&mut self) -> impl Iterator<Item = &mut Type> {
        self.iter_mut().filter_map(|arg| {
            if let GenericArgument::Type(ty) = arg {
                Some(ty)
            } else {
                None
            }
        })
    }

    fn enumerate_types(&mut self) -> impl Iterator<Item = (usize, &mut Type)> {
        self.iter_mut().enumerate().filter_map(|(i, arg)| {
            if let GenericArgument::Type(ty) = arg {
                Some((i, ty))
            } else {
                None
            }
        })
    }

    fn associated_types(&self) -> impl Iterator<Item = &AssocType> {
        self.iter().filter_map(|arg| {
            if let GenericArgument::AssocType(ty) = arg {
                Some(ty)
            } else {
                None
            }
        })
    }

    fn associated_types_mut(&mut self) -> impl Iterator<Item = &mut AssocType> {
        self.iter_mut().filter_map(|arg| {
            if let GenericArgument::AssocType(ty) = arg {
                Some(ty)
            } else {
                None
            }
        })
    }
}

impl<T: Deref<Target = Path>> TypeResult<T> {
    pub fn set_path(&mut self, path: Path)
    where
        T: DerefMut<Target = Path>,
    {
        let Path {
            leading_colon,
            mut segments,
        } = path;
        if self.path.leading_colon.is_none() {
            self.path.leading_colon = leading_colon;
        }

        let old_segments: Vec<_> = self
            .path
            .segments
            .iter()
            .skip_while(|segment| segment.ident != "Result")
            .cloned()
            .collect();

        segments.extend(old_segments);
        self.path.segments = segments.into_iter().collect();
    }

    fn args(&self) -> &Punctuated<GenericArgument, Token![,]> {
        #[expect(
            clippy::unwrap_used,
            reason = "ParseType::as_result guarantees that there is at least one segment"
        )]
        let arguments = &self.path.segments.last().unwrap().arguments;
        let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) = arguments
        else {
            unreachable!("ParseType::as_result guarantees that arguments are bracketed")
        };

        args
    }

    fn args_mut(&mut self) -> &mut Punctuated<GenericArgument, Token![,]>
    where
        T: DerefMut<Target = Path>,
    {
        #[expect(
            clippy::unwrap_used,
            reason = "ParseType::as_result guarantees that there is at least one segment"
        )]
        let arguments = &mut self.path.segments.last_mut().unwrap().arguments;
        let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) = arguments
        else {
            unreachable!("ParseType::as_result guarantees that arguments are bracketed")
        };

        args
    }

    pub fn ok(&self) -> &Type {
        #[expect(
            clippy::unwrap_used,
            reason = "ParseType::as_result guarantees that there is at least one type argument"
        )]
        self.args().types().next().unwrap()
    }

    pub fn ok_mut(&mut self) -> &mut Type
    where
        T: DerefMut<Target = Path>,
    {
        #[expect(
            clippy::unwrap_used,
            reason = "ParseType::as_result guarantees that there is at least one type argument"
        )]
        self.args_mut().types_mut().next().unwrap()
    }

    pub fn err(&self) -> Option<&Type> {
        self.args().types().nth(1)
    }

    pub fn set_err(&mut self, ty: Type)
    where
        T: DerefMut<Target = Path>,
    {
        let err = GenericArgument::Type(ty);
        let index = self.args_mut().enumerate_types().nth(1).map(|x| x.0);
        if let Some(index) = index {
            self.args_mut()[index] = err;
        } else {
            self.args_mut().push(err);
        }
    }
}

impl<T: Deref<Target = Punctuated<GenericArgument, Token![,]>>> TypeStream<T> {
    pub fn item(&self) -> &Type {
        #[expect(
            clippy::unwrap_used,
            reason = "ParseType::as_stream guarantees that there is an associated item named `Item`"
        )]
        &self
            .args
            .associated_types()
            .find(|ty| ty.ident == "Item")
            .unwrap()
            .ty
    }

    pub fn item_mut(&mut self) -> &mut Type
    where
        T: DerefMut<Target = Punctuated<GenericArgument, Token![,]>>,
    {
        #[expect(
            clippy::unwrap_used,
            reason = "ParseType::as_stream guarantees that there is an associated item named `Item`"
        )]
        &mut self
            .args
            .associated_types_mut()
            .find(|ty| ty.ident == "Item")
            .unwrap()
            .ty
    }
}
