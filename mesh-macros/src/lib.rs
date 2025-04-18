use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error2::proc_macro_error;
use quote::ToTokens;
use syn::{
    braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Ident, Path, Result, Token,
};

#[proc_macro_derive(Builder, attributes(skip))]
pub fn builder(input: TokenStream) -> TokenStream {
    let ouput = builder_impl(input);
    ouput
}

fn builder_impl(input: TokenStream) -> TokenStream {
    let strct = syn::parse_macro_input!(input as syn::ItemStruct);

    let strct_name = &strct.ident;
    let name = quote::format_ident!("{}Builder", strct.ident);
    let vis = strct.vis;
    let generics = strct.generics;
    let fields = strct
        .fields
        .iter()
        .filter(|field| {
            field
                .attrs
                .iter()
                .find(|attr| {
                    if let Some(name) = attr.path().get_ident() {
                        if name == "skip" {
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
                .is_none()
        })
        .collect::<Vec<_>>();
    let impl_block = generate_impl_block(&name, &vis, &generics, &fields);
    let build_func = {
        let assignments = fields.iter().filter_map(|field| {
            let name = field.ident.as_ref()?;
            Some(quote::quote! {
                #name: self.#name
            })
        });
        quote::quote! {
            impl #generics #name #generics {
                #vis fn build(mut self) -> util::Shared<#strct_name> {
                    util::Shared::new(#strct_name {
                        #(#assignments),*
                        , ..Default::default()
                    })
                }
            }
        }
    };
    let builder_fn = {
        quote::quote! {
            impl #generics #strct_name #generics {
                #vis fn builder() -> #name {
                    #name::default()
                }
            }
        }
    };
    quote::quote! {
        #builder_fn
        #[derive(Default)]
        #vis struct #name #generics {
            #(#fields),*
        }
        #impl_block
        #build_func
    }
    .into()
}

fn generate_impl_block(
    name: &syn::Ident,
    vis: &syn::Visibility,
    generics: &syn::Generics,
    fields: &[&syn::Field],
) -> TokenStream2 {
    let functions = fields.iter().filter_map(|field| {
        let name = field.ident.as_ref()?;
        let ty = &field.ty;
        Some(quote::quote! {
            #vis fn #name(mut self, #name: #ty) -> Self {
                self.#name = #name;
                self
            }
        })
    });

    quote::quote! {
        impl #generics #name #generics {
            #(#functions)*
        }
    }
}

#[derive(Debug)]
struct Param {
    inner: syn::ExprCall,
}

impl Parse for Param {
    fn parse(input: ParseStream) -> Result<Self> {
        let func = input.parse::<syn::ExprCall>()?;
        Ok(Self { inner: func })
    }
}

#[derive(Debug)]
enum UiItem {
    Component(UiTree),
    Value(Ident),
}

impl Parse for UiItem {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(syn::token::Brace) {
            let content;
            syn::braced!(content in input);
            Ok(Self::Value(content.parse()?))
        } else if input.peek(syn::Ident) {
            Ok(Self::Component(input.parse()?))
        } else {
            unreachable!()
            //proc_macro_error2::abort! {}
        }
    }
}

impl ToTokens for UiItem {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Self::Component(tree) => tree.to_tokens(tokens),
            Self::Value(ident) => ident.to_tokens(tokens),
        }
    }
}

#[derive(Debug)]
struct UiTree {
    item: Path,
    params: Vec<Param>,
    items: Vec<UiItem>,
}

impl Parse for UiTree {
    fn parse(input: ParseStream) -> Result<Self> {
        let item = input.parse()?;
        let params = if input.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            let punctuated = Punctuated::<Param, Token![,]>::parse_terminated(&content)?;
            punctuated
                .into_pairs()
                .map(|pair| pair.into_value())
                .collect()
        } else {
            Vec::new()
        };
        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            let punctuated = Punctuated::<UiItem, Token![,]>::parse_terminated(&content)?;
            let items = punctuated
                .into_pairs()
                .map(|pair| pair.into_value())
                .collect();
            return Ok(Self {
                item,
                params,
                items,
            });
        }

        Ok(Self {
            item,
            params: Vec::new(),
            items: Vec::new(),
        })
    }
}

impl ToTokens for UiTree {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.item;
        let children = self
            .items
            .iter()
            .map(|item| {
                quote::quote! {
                    #item
                }
            })
            .collect::<Vec<_>>();
        let params = self
            .params
            .iter()
            .map(|param| {
                let name = &param.inner.func;
                let args = &param.inner.args.to_token_stream();
                quote::quote! {
                    #name(#args)
                }
            })
            .collect::<Vec<_>>();
        let new_tokens = quote::quote! {
            {
                let this = {
                    #name::builder()
                    #(.#params)*
                    .build()
                };
                #(this.append(#children);)*
                this
            }
        };
        tokens.extend(new_tokens);
    }
}

#[proc_macro]
#[proc_macro_error]
pub fn view(input: TokenStream) -> TokenStream {
    let tree = match syn::parse::<UiTree>(input) {
        Ok(tree) => tree,
        Err(e) => {
            proc_macro_error2::abort! {
                e
            }
        }
    };
    let out = tree.to_token_stream();
    out.into()
}
