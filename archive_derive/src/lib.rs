extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::{
    quote,
    quote_spanned,
};
use syn::{
    AttrStyle,
    Data,
    DeriveInput,
    Error,
    Fields,
    Ident,
    Index,
    Meta,
    NestedMeta,
    parse_macro_input,
    spanned::Spanned,
};

#[proc_macro_derive(Archive)]
pub fn archive_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let archive_impl = derive_archive_impl(&input);

    proc_macro::TokenStream::from(archive_impl)
}

fn derive_archive_impl(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;

    let generic_params = input.generics.params.iter().map(|p| quote! { #p });
    let generic_params = quote! { #(#generic_params,)* };

    let generic_args = input.generics.type_params().map(|p| {
        let name = &p.ident;
        quote_spanned! { p.ident.span() => #name }
    });
    let generic_args = quote! { #(#generic_args,)* };

    let generic_predicates = match input.generics.where_clause {
        Some(ref clause) => {
            let predicates = clause.predicates.iter().map(|p| quote! { #p });
            quote! { #(#predicates,)* }
        },
        None => quote! {},
    };

    let archive_impl = match input.data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let field_wheres = fields.named.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: Archive }
                    });
                    let field_wheres = quote! { #(#field_wheres,)* };

                    let resolver_fields = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #name: archive::Resolver<#ty> }
                    });

                    let resolver_values = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        quote_spanned! { f.span() => #name: self.#name.archive(writer)? }
                    });

                    let archived_fields = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #name: archive::Archived<#ty> }
                    });

                    let archived_values = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        quote_spanned! { f.span() => #name: self.#name.resolve(pos + offset_of!(Archived<#generic_args>, #name), &value.#name) }
                    });

                    quote! {
                        struct Resolver<#generic_params>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            #(#resolver_fields,)*
                        }

                        impl<#generic_params> Resolve<#name<#generic_args>> for Resolver<#generic_args>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            type Archived = Archived<#generic_args>;

                            fn resolve(self, pos: usize, value: &#name<#generic_args>) -> Self::Archived {
                                Self::Archived {
                                    #(#archived_values,)*
                                }
                            }
                        }

                        struct Archived<#generic_params>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            #(#archived_fields,)*
                        }

                        impl<#generic_params> Archive for #name<#generic_args>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            type Archived = Archived<#generic_args>;
                            type Resolver = Resolver<#generic_args>;

                            fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                                Ok(Self::Resolver {
                                    #(#resolver_values,)*
                                })
                            }
                        }
                    }
                },
                Fields::Unnamed(ref fields) => {
                    let field_wheres = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: Archive }
                    });
                    let field_wheres = quote! { #(#field_wheres,)* };

                    let resolver_fields = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => archive::Resolver<#ty> }
                    });

                    let resolver_values = fields.unnamed.iter().enumerate().map(|(i, f)| {
                        let index = Index::from(i);
                        quote_spanned! { f.span() => self.#index.archive(writer)? }
                    });

                    let archived_fields = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => archive::Archived<#ty> }
                    });

                    let archived_values = fields.unnamed.iter().enumerate().map(|(i, f)| {
                        let index = Index::from(i);
                        quote_spanned! { f.span() => self.#index.resolve(pos + offset_of!(Archived<#generic_args>, #index), &value.#index) }
                    });

                    quote! {
                        struct Resolver<#generic_params>(#(#resolver_fields,)*)
                        where
                            #generic_predicates
                            #field_wheres;

                        impl<#generic_params> Resolve<#name<#generic_args>> for Resolver<#generic_args>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            type Archived = Archived<#generic_args>;

                            fn resolve(self, pos: usize, value: &#name<#generic_args>) -> Self::Archived {
                                Archived::<#generic_args>(
                                    #(#archived_values,)*
                                )
                            }
                        }

                        struct Archived<#generic_params>(#(#archived_fields,)*)
                        where
                            #generic_predicates
                            #field_wheres;

                        impl<#generic_params> Archive for #name<#generic_args>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            type Archived = Archived<#generic_args>;
                            type Resolver = Resolver<#generic_args>;

                            fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                                Ok(Resolver::<#generic_args>(
                                    #(#resolver_values,)*
                                ))
                            }
                        }
                    }
                },
                Fields::Unit => {
                    quote! {
                        struct Resolver;

                        impl<#generic_params> Resolve<#name<#generic_args>> for Resolver
                        where
                            #generic_predicates
                        {
                            type Archived = #name<#generic_args>;

                            fn resolve(self, _pos: usize, _value: &#name<#generic_args>) -> Self::Archived {
                                #name::<#generic_args>
                            }
                        }

                        impl<#generic_params> Archive for #name<#generic_args>
                        where
                            #generic_predicates
                        {
                            type Archived = #name<#generic_args>;
                            type Resolver = Resolver;

                            fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                                Ok(Resolver)
                            }
                        }
                    }
                }
            }
        },
        Data::Enum(ref data) => {
            let field_wheres = data.variants.iter().map(|v| {
                match v.fields {
                    Fields::Named(ref fields) => {
                        let field_wheres = fields.named.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() =>  #ty: Archive }
                        });
                        quote! { #(#field_wheres,)* }
                    },
                    Fields::Unnamed(ref fields) => {
                        let field_wheres = fields.unnamed.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() => #ty: Archive }
                        });
                        quote! { #(#field_wheres,)* }
                    },
                    Fields::Unit => quote! {},
                }
            });
            let field_wheres = quote! { #(#field_wheres)* };

            let resolver_variants = data.variants.iter().map(|v| {
                let variant = &v.ident;
                match v.fields {
                    Fields::Named(ref fields) => {
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let ty = &f.ty;
                            quote_spanned! { f.span() => #name: archive::Resolver<#ty> }
                        });
                        quote_spanned! { variant.span() =>
                            #variant {
                                #(#fields,)*
                            }
                        }
                    },
                    Fields::Unnamed(ref fields) => {
                        let fields = fields.unnamed.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() => archive::Resolver<#ty> }
                        });
                        quote_spanned! { variant.span() =>
                            #variant(#(#fields,)*)
                        }
                    },
                    Fields::Unit => quote_spanned! { variant.span() => #variant },
                }
            });

            let resolve_arms = data.variants.iter().map(|v| {
                let variant = &v.ident;
                let archived_variant_name = Ident::new(&format!("ArchivedVariant{}", variant.to_string()), v.span());
                match v.fields {
                    Fields::Named(ref fields) => {
                        let self_bindings = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let binding = Ident::new(&format!("self_{}", name.as_ref().unwrap().to_string()), name.span());
                            quote_spanned! { name.span() => #name: #binding }
                        });
                        let value_bindings = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let binding = Ident::new(&format!("value_{}", name.as_ref().unwrap().to_string()), name.span());
                            quote_spanned! { binding.span() => #name: #binding }
                        });
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let self_binding = Ident::new(&format!("self_{}", name.as_ref().unwrap().to_string()), name.span());
                            let value_name = Ident::new(&format!("value_{}", name.as_ref().unwrap().to_string()), name.span());
                            quote! {
                                #name: #self_binding.resolve(pos + offset_of!(#archived_variant_name<#generic_args>, #name), #value_name)
                            }
                        });
                        quote_spanned! { name.span() =>
                            Self::#variant { #(#self_bindings,)* } => {
                                if let #name::#variant { #(#value_bindings,)* } = value { Archived::#variant { #(#fields,)* } } else { panic!("enum resolver variant does not match value variant") }
                            }
                        }
                    },
                    Fields::Unnamed(ref fields) => {
                        let self_bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("self_{}", i), f.span());
                            quote_spanned! { f.span() => #name }
                        });
                        let value_bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("value_{}", i), f.span());
                            quote_spanned! { f.span() => #name }
                        });
                        let fields = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let index = Index::from(i + 1);
                            let self_binding = Ident::new(&format!("self_{}", i), f.span());
                            let value_binding = Ident::new(&format!("value_{}", i), f.span());
                            quote! {
                                #self_binding.resolve(pos + offset_of!(#archived_variant_name<#generic_args>, #index), #value_binding)
                            }
                        });
                        quote_spanned! { name.span() =>
                            Self::#variant( #(#self_bindings,)* ) => {
                                if let #name::#variant(#(#value_bindings,)*) = value { Archived::#variant(#(#fields,)*) } else { panic!("enum resolver variant does not match value variant") }
                            }
                        }
                    },
                    Fields::Unit => quote_spanned! { name.span() => Self::#variant => Archived::#variant },
                }
            });

            let archived_repr = match data.variants.len() {
                0..=255 => quote! { u8 },
                256..=65_535 => quote! { u16 },
                65_536..=4_294_967_295 => quote! { u32 },
                4_294_967_296..=18_446_744_073_709_551_615 => quote! { u64 },
                _ => quote! { u128 },
            };

            let archived_variants = data.variants.iter().map(|v| {
                let variant = &v.ident;
                match v.fields {
                    Fields::Named(ref fields) => {
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let ty = &f.ty;
                            quote_spanned! { f.span() => #name: archive::Archived<#ty> }
                        });
                        quote_spanned! { variant.span() =>
                            #variant {
                                #(#fields,)*
                            }
                        }
                    },
                    Fields::Unnamed(ref fields) => {
                        let fields = fields.unnamed.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() => archive::Archived<#ty> }
                        });
                        quote_spanned! { variant.span() =>
                            #variant(#(#fields,)*)
                        }
                    },
                    Fields::Unit => quote_spanned! { variant.span() => #variant },
                }
            });

            let archived_variant_tags = data.variants.iter().map(|v| {
                let variant = &v.ident;
                quote_spanned! { variant.span() => #variant }
            });

            let archived_variant_structs = data.variants.iter().map(|v| {
                let variant = &v.ident;
                let archived_variant_name = Ident::new(&format!("ArchivedVariant{}", variant.to_string()), v.span());
                match v.fields {
                    Fields::Named(ref fields) => {
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let ty = &f.ty;
                            quote_spanned! { f.span() => #name: archive::Archived<#ty> }
                        });
                        quote_spanned! { name.span() =>
                            #[repr(C)]
                            struct #archived_variant_name<#generic_params>
                            where
                                #generic_predicates
                                #field_wheres
                            {
                                __tag: ArchivedTag,
                                #(#fields,)*
                                __phantom: PhantomData<(#generic_args)>,
                            }
                        }
                    },
                    Fields::Unnamed(ref fields) => {
                        let fields = fields.unnamed.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() => archive::Archived<#ty> }
                        });
                        quote_spanned! { name.span() =>
                            #[repr(C)]
                            struct #archived_variant_name<#generic_params>(ArchivedTag, #(#fields,)* PhantomData<(#generic_args)>)
                            where
                                #generic_predicates
                                #field_wheres;
                        }
                    },
                    Fields::Unit => quote! {},
                }
            });

            let archive_arms = data.variants.iter().map(|v| {
                let variant = &v.ident;
                match v.fields {
                    Fields::Named(ref fields) => {
                        let bindings = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            quote_spanned! { name.span() => #name }
                        });
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            quote! {
                                #name: #name.archive(writer)?
                            }
                        });
                        quote_spanned! { name.span() =>
                            Self::#variant { #(#bindings,)* } => Resolver::#variant {
                                #(#fields,)*
                            }
                        }
                    },
                    Fields::Unnamed(ref fields) => {
                        let bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("_{}", i), f.span());
                            quote_spanned! { f.span() => #name }
                        });
                        let fields = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let binding = Ident::new(&format!("_{}", i), f.span());
                            quote! {
                                #binding.archive(writer)?
                            }
                        });
                        quote_spanned! { name.span() =>
                            Self::#variant( #(#bindings,)* ) => Resolver::#variant(#(#fields,)*)
                        }
                    },
                    Fields::Unit => quote_spanned! { name.span() => Self::#variant => Resolver::#variant },
                }
            });

            quote! {
                enum Resolver<#generic_params>
                where
                    #generic_predicates
                    #field_wheres
                {
                    #(#resolver_variants,)*
                }

                impl<#generic_params> Resolve<#name<#generic_args>> for Resolver<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {
                    type Archived = Archived<#generic_args>;

                    fn resolve(self, pos: usize, value: &#name<#generic_args>) -> Self::Archived {
                        match self {
                            #(#resolve_arms,)*
                        }
                    }
                }

                #[repr(#archived_repr)]
                enum Archived<#generic_params>
                where
                    #generic_predicates
                    #field_wheres
                {
                    #(#archived_variants,)*
                }

                #[repr(#archived_repr)]
                enum ArchivedTag {
                    #(#archived_variant_tags,)*
                }

                #(#archived_variant_structs)*

                impl<#generic_params> Archive for #name<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {
                    type Archived = Archived<#generic_args>;
                    type Resolver = Resolver<#generic_args>;

                    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                        Ok(match self {
                            #(#archive_arms,)*
                        })
                    }
                }
            }
        },
        Data::Union(_) => Error::new(input.span(), "Archive cannot be derived for unions").to_compile_error(),
    };

    quote! {
        const _: () = {
            use core::marker::PhantomData;
            use archive::{
                Archive,
                offset_of,
                Resolve,
                Write,
            };
            #archive_impl
        };
    }
}

#[proc_macro_derive(ArchiveCopy, attributes(archive))]
pub fn archive_copy_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let impl_archive_copy = derive_archive_copy_impl(&input);

    let def = quote! {
        const _: () = {
            use archive::{
                Archive,
                ArchiveCopy,
                CopyResolver,
                Write,
            };

            #impl_archive_copy
        };
    };

    proc_macro::TokenStream::from(def)
}

fn derive_archive_copy_impl(input: &DeriveInput) -> TokenStream {
    let repr_set = input.attrs.iter().filter_map(|a| {
        match a.style {
            AttrStyle::Outer => match a.parse_meta() {
                Ok(meta) => match meta {
                    Meta::List(meta) => if meta.path.is_ident("archive") {
                        let repr_set = meta.nested.iter().any(|n| {
                            match n {
                                NestedMeta::Meta(meta) => match meta {
                                    Meta::Path(path) => {
                                        path.is_ident("repr_set")
                                    },
                                    _ => false,
                                },
                                _ => false,
                            }
                        });
                        if repr_set {
                            Some(Ok(()))
                        } else {
                            None
                        }
                    } else {
                        None
                    },
                    _ => Some(Err(Error::new(meta.span(), "unsupported attribute type, expected path").to_compile_error()))
                },
                _ => Some(Err(Error::new(a.span(), "unable to parse attribute").to_compile_error())),
            },
            _ => Some(Err(Error::new(a.span(), "attributes must be outer").to_compile_error()))
        }
    }).next();

    let name = &input.ident;

    let generic_params = input.generics.params.iter().map(|p| quote! { #p });
    let generic_params = quote! { #(#generic_params,)* };

    let generic_args = input.generics.type_params().map(|p| {
        let name = &p.ident;
        quote_spanned! { p.ident.span() => #name }
    });
    let generic_args = quote! { #(#generic_args,)* };

    let generic_predicates = match input.generics.where_clause {
        Some(ref clause) => {
            let predicates = clause.predicates.iter().map(|p| quote! { #p });
            quote! { #(#predicates,)* }
        },
        None => quote! {},
    };

    match input.data {
        Data::Struct(ref data) => {
            let field_wheres = match data.fields {
                Fields::Named(ref fields) => {
                    let field_wheres = fields.named.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: ArchiveCopy }
                    });

                    quote! { #(#field_wheres,)* }
                },
                Fields::Unnamed(ref fields) => {
                    let field_wheres = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: ArchiveCopy }
                    });

                    quote! { #(#field_wheres,)* }
                },
                Fields::Unit => quote! {},
            };

            quote! {
                unsafe impl<#generic_params> ArchiveCopy for #name<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {}

                impl<#generic_params> Archive for #name<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {
                    type Archived = Self;
                    type Resolver = CopyResolver;

                    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                        Ok(CopyResolver)
                    }
                }
            }
        },
        Data::Enum(ref data) => {
            match repr_set {
                Some(Ok(())) => (),
                Some(Err(error)) => return error,
                None => return Error::new(input.span(), "enum may be an invalid repr, make sure the enum has #[repr(u*)] or #[repr(i*)] then add #[archive(repr_set)]").to_compile_error(),
            }

            let field_wheres = data.variants.iter().map(|v| {
                match v.fields {
                    Fields::Named(ref fields) => {
                        let field_wheres = fields.named.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() => #ty: ArchiveCopy }
                        });
                        quote! { #(#field_wheres,)* }
                    },
                    Fields::Unnamed(ref fields) => {
                        let field_wheres = fields.unnamed.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() => #ty: ArchiveCopy }
                        });
                        quote! { #(#field_wheres,)* }
                    },
                    Fields::Unit => quote! {},
                }
            });
            let field_wheres = quote! { #(#field_wheres)* };

            quote! {
                unsafe impl<#generic_params> ArchiveCopy for #name<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {}

                impl<#generic_params> Archive for #name<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {
                    type Archived = Self;
                    type Resolver = CopyResolver;

                    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                        Ok(CopyResolver)
                    }
                }
            }
        },
        Data::Union(_) => Error::new(input.span(), "Archive cannot be derived for unions").to_compile_error(),
    }
}
