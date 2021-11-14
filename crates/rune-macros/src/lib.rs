/*
 *  Copyright (c) 2021 Uskrai
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{parse::*, punctuated::Punctuated, *};

// this is used to indicate whether the type should be registered with module.ty::<Type>()
// `: skip` after type will make it skipped
// ```
struct SkipableType {
  ty: Type,
  is_skip: bool,
}

impl Parse for SkipableType {
  fn parse(input: ParseStream) -> Result<Self> {
    // get the type.
    let ty = input.parse()?;
    // check if : exists
    let is_skip = input.parse::<Option<Token![:]>>()?.is_some();

    // if : exists, check if the next iden is skip.
    if is_skip {
      let ident: Ident = input.parse()?;
      if ident != "skip" {
        // throw error if ident is not skip
        return Err(syn::parse::Error::new(ident.span(), "expected `skip`"));
      }
    }

    Ok(Self { ty, is_skip })
  }
}

impl SkipableType {
  pub fn register(&self, items: &mut Vec<Stmt>) {
    // register type if type is not skipped.
    if !self.is_skip {
      let ty = &self.ty;
      items.push(parse_quote! {
        module.ty::<#ty>()?;
      });
    }
  }
}

/// enum to indicate how the function should be registered
/// associated will call function(&[Type, name], ident);
/// async_associated will call async_function(&[Type, name], ident);
/// inst will call inst_fn(name, Type::ident);
/// async_inst will call async_inst_fn(name, Type::ident);
/// protocol will call inst_fn(Protocol::name, Type::ident);
enum MethodKind {
  Associated,
  AsyncAssociated,
  Inst,
  AsyncInst,
  Protocol,
}

impl Parse for MethodKind {
  fn parse(input: ParseStream) -> Result<Self> {
    let ident: Ident = input.parse()?;

    Ok(match ident.to_string().as_str() {
      "associated" => Self::Associated,
      "async_associated" => Self::AsyncAssociated,
      "inst" => Self::Inst,
      "async_inst" => Self::AsyncInst,
      "protocol" => Self::Protocol,
      _ => {
        return Err(syn::Error::new(
          ident.span(),
          "expected `associated`, `async_associated`, `inst`, `async_inst`, `protocol`",
        ));
      }
    })
  }
}

/// this is a token for method name and the name to register in module
/// if no name is provided with `method_name : name` then `method_name`
/// is used as name
struct Method {
  ident: Ident,
  name: String,
}

impl Parse for Method {
  fn parse(input: ParseStream) -> Result<Self> {
    let ident: Ident = input.parse()?;
    let mut name = ident.to_string();

    if input.peek(Token![:]) {
      input.parse::<Token![:]>()?;
      if input.peek(LitStr) {
        name = input.parse::<LitStr>()?.value();
      } else if input.peek(Ident) {
        name = input.parse::<Ident>()?.to_string();
      }
    }

    Ok(Self { ident, name })
  }
}

struct VectorMethod {
  kind: MethodKind,
  vec: Punctuated<Method, Option<Token![,]>>,
}

impl Parse for VectorMethod {
  fn parse(input: ParseStream) -> Result<Self> {
    let kind = input.parse()?;
    input.parse::<Token![=>]>()?;
    let braced;
    braced!(braced in input);

    let vec = Punctuated::parse_terminated(&braced)?;

    Ok(Self { kind, vec })
  }
}

impl VectorMethod {
  fn register(&self, ty: &Type, items: &mut Vec<Stmt>) {
    for method in &self.vec {
      let ident = &method.ident;
      let name = &method.name;
      match self.kind {
        MethodKind::Inst => items.push(parse_quote! {
          module.inst_fn(#name, #ty::#ident)?;
        }),

        MethodKind::AsyncInst => {
          items.push(parse_quote! {
            module.async_inst_fn(#name, #ty::#ident)?;
          });
        }

        MethodKind::Associated => items.push(parse_quote! {
          module.function(&[stringify!(#ty),#name], #ty::#ident)?;
        }),

        MethodKind::AsyncAssociated => items.push(parse_quote! {
          module.async_function(&[stringify!(#ty), #name], #ty::#ident)?;
        }),

        MethodKind::Protocol => {
          let name = syn::parse_str::<Ident>(name).unwrap();

          items.push(parse_quote! {
            module.inst_fn(::runestick::Protocol::#name, #ty::#ident)?;
          });
        }
      }
    }
  }
}

struct ItemStruct {
  types: Punctuated<SkipableType, Option<Token![,]>>,
  function: Punctuated<VectorMethod, Option<Token![,]>>,
}

impl Parse for ItemStruct {
  fn parse(input: ParseStream) -> Result<Self> {
    let types;
    parenthesized!(types in input);
    let types = Punctuated::parse_terminated(&types)?;
    input.parse::<Token![=>]>()?;
    let function;
    braced!(function in input);
    let function = Punctuated::parse_terminated(&function)?;

    Ok(Self { types, function })
  }
}

impl ItemStruct {
  fn register(&self, items: &mut Vec<Stmt>) {
    for ty in &self.types {
      ty.register(items);

      for it in &self.function {
        it.register(&ty.ty, items);
      }
    }
  }
}

struct VecItemStruct {
  items: Vec<ItemStruct>,
}

impl VecItemStruct {
  fn register(&self, items: &mut Vec<Stmt>) {
    for it in &self.items {
      it.register(items);
    }
  }
}

impl Parse for VecItemStruct {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut items = Vec::new();

    loop {
      let item = input.parse()?;
      items.push(item);
      if !input.is_empty() {
        input.parse::<Token![,]>()?;
      } else {
        break;
      }
    }

    Ok(Self { items })
  }
}

#[proc_macro]
pub fn register_module(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as VecItemStruct);

  let mut vec = Vec::new();

  input.register(&mut vec);

  let output = quote::quote! {
      pub fn load_module_with(mut module: ::runestick::Module)
        -> Result<::runestick::Module, ::runestick::ContextError> {
        #(#vec)*
        Ok(module)
      }
  };

  TokenStream::from(output.into_token_stream())
}
