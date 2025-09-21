use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(IntoBoltType)]
pub fn derive_into_bolttype(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let fields = match input.data {
        Data::Struct(ref s) => match &s.fields {
            Fields::Named(named) => &named.named,
            _ => panic!("IntoBoltType only supports structs with named fields"),
        },
        _ => panic!("IntoBoltType only supports structs"),
    };

    let field_names = fields.iter().map(|f| f.ident.as_ref().unwrap());
    let field_strings = fields.iter().map(|f| f.ident.as_ref().unwrap().to_string());

    let expanded = quote! {
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
    };

    TokenStream::from(expanded)
}
