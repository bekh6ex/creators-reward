use proc_macro::TokenStream;

use quote::quote;
use syn::Meta;

#[proc_macro_attribute]
pub fn tagged_id(attr: TokenStream, input: TokenStream) -> TokenStream {
    let ast = match syn::parse::<syn::DeriveInput>(input.clone()) {
        Ok(ast) => ast,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };
    let meta = match syn::parse::<Meta>(attr) {
        Ok(meta) => meta,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };
    let name_value = match meta.require_name_value() {
        Ok(name_value) => name_value.clone(),
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };
    assert!(name_value.path.is_ident("tag"));

    let name = ast.ident.clone();

    let orig = proc_macro2::TokenStream::from(input);

    // TODO: Add check if tag is already assigned to some other type

    let expanded = quote! {
        // ====== start ======
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
        #orig

        impl TaggedId for #name {
            fn new() -> Self {
                Self::new()
            }
        }

        impl #name {
            pub fn new() -> Self {
                #name(uuid::Uuid::new_v4())
            }
        }

        impl serde::Serialize for #name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::ser::Serializer,
            {
                serializer.serialize_str(&format!("#tag:{}", self.0))
            }
        }

        impl<'de> serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct Vis;

                impl<'de> serde::de::Visitor<'de> for Vis {
                    type Value = Uuid;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("#tag:<uuid>")
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        use serde::de;
                        use std::str::FromStr;

                        if let Some(s) = v.strip_prefix("#tag:") {
                            Ok(
                                Uuid::from_str(s)
                                    .map_err(|e| serde::de::Error::custom(e.to_string()))?,
                            )
                        } else {
                            Err(de::Error::unknown_variant(
                                v,
                                &["#tag:<uuid>"],
                            ))
                        }
                    }
                }

                Ok(Self(deserializer.deserialize_string(Vis)?))
            }
        }
        // ====== end ======
    };

    expanded.into()
}
