use proc_macro::TokenStream;
use quote::quote;
use syn::{self, spanned::Spanned, Data, DeriveInput, Fields};

#[proc_macro_derive(ImGui)]
pub fn derive_imgui(input: TokenStream) -> TokenStream {
    match syn::parse::<DeriveInput>(input).and_then(|input| impl_derive_imgui(input)) {
        Ok(result) => result,
        Err(err) => err.into_compile_error().into(),
    }
}

fn impl_derive_imgui(ast: syn::DeriveInput) -> syn::Result<TokenStream> {
    let name = &ast.ident;
    let data = match &ast.data {
        Data::Struct(data) => data,
        _ => return Err(syn::Error::new(ast.span(), "Expected struct"))
    };

    let fields = match &data.fields {
        Fields::Named(fields) => fields.named.iter().collect(),
        Fields::Unnamed(fields) => return Err(syn::Error::new(fields.span(), "Structs with unnamed fields are not supported")),
        Fields::Unit => Vec::new(),
    };

    let fields = fields.iter().filter_map(|field| {
        match &field.ident {
            Some(ident) => {
                let ident_str = ident.to_string();
                Some(quote! {
                    crate::imgui_helpers::ImGui::draw_gui_with_settings(&mut self.#ident, #ident_str, gui, settings);
                })
            },
            None => None
        }
    });

    let out = quote! {
        impl ImGui for #name {
            fn draw_gui_with_settings(&mut self, label: &str, gui: &imgui::Ui, settings: &crate::imgui_helpers::ImGuiSettings) {
                if gui.collapsing_header(label, imgui::TreeNodeFlags::empty()) {
                    gui.indent();
                    #(#fields);*
                    gui.unindent();
                }
            }
        }
    };
    
    Ok(out.into())
}

// Based on https://stackoverflow.com/a/41638362
#[proc_macro_derive(EnumCount)]
pub fn derive_enum_count(input: TokenStream) -> TokenStream {
    match syn::parse::<DeriveInput>(input).and_then(|input| impl_derive_enum_count(input)) {
        Ok(result) => result,
        Err(err) => err.into_compile_error().into(),
    }
}

fn impl_derive_enum_count(ast: syn::DeriveInput) -> syn::Result<TokenStream> {
    let name = &ast.ident;
    let len = match ast.data {
        syn::Data::Enum(item) => item.variants.len(),
        _ => return Err(syn::Error::new(ast.span(), "Only enums are supported")),
    };
    let out = quote! {
        impl #name {
            pub const COUNT: usize = #len;
        }
    };
    Ok(out.into())
}
