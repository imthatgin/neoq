use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(IntoBoltType)]
pub fn derive_into_bolttype(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let expanded = match input.data {
        Data::Struct(ref s) => {
            let fields = match &s.fields {
                Fields::Named(named) => &named.named,
                _ => panic!("IntoBoltType does not support structs with unnamed fields"),
            };

            let field_names = fields.iter().map(|f| f.ident.as_ref().unwrap());
            let field_strings = fields.iter().map(|f| f.ident.as_ref().unwrap().to_string());

            quote! {
                impl ::std::convert::Into<neo4rs::BoltType> for #name {
                    fn into(self) -> neo4rs::BoltType {
                        let mut map = neo4rs::BoltMap::new();
                        #(
                            map.put(
                                #field_strings.to_string().into(),
                                self.#field_names.into()
                            );
                        )*
                        neo4rs::BoltType::Map(map)
                    }
                }
            }
        },
        Data::Enum(ref e) => {
            let variants = e.variants.iter().map(|v| &v.ident);
            quote! {
                impl ::std::convert::Into<neo4rs::BoltType> for #name {
                    fn into(self) -> neo4rs::BoltType {
                        match self {
                            #(
                                #name::#variants => neo4rs::BoltType::String(neo4rs::BoltString::new(stringify!(#variants))),
                            )*
                        }
                    }
                }
            }
        }
        _ => panic!("IntoBoltType does not support this type"),
    };

    TokenStream::from(expanded)
}
