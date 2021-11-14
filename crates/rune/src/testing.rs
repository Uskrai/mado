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

use rune::{
  ast::{
    generated::ColonColon, CloseBrace, Ident, Kind, Lit, OpenBrace, Path,
    PathSegment, Token,
  },
  macros::stringify,
  quote, Parser, TokenStream, T,
};
use runestick::{ContextError, Module, Result, Span};

pub fn stringify_path(path: &rune::ast::Path) -> String {
  let mut res = rune::macros::stringify(&path.first);

  for (c, p) in &path.rest {
    res += &rune::macros::stringify(c);
    res += &rune::macros::stringify(p);
  }

  res
}

pub fn create_empty_token(kind: Kind) -> Token {
  Token {
    span: Span::empty(),
    kind,
  }
}

pub fn ident_to_path_segment(string: &str) -> PathSegment {
  match string {
    "super" => PathSegment::Super(rune::ast::generated::Super {
      token: create_empty_token(Kind::Super),
    }),
    "crate" => PathSegment::Crate(rune::ast::generated::Crate {
      token: create_empty_token(Kind::Crate),
    }),
    _ => PathSegment::Ident(Ident::new(string)),
  }
}

pub fn ident_to_path(string: &str) -> Path {
  let create_colon_colon = || ColonColon {
    token: Token {
      span: Span::empty(),
      kind: Kind::ColonColon,
    },
  };
  let id = None;

  let mut split = string.split("::").filter(|v| !v.is_empty());

  let first = ident_to_path_segment(split.next().unwrap());
  let global = if string.starts_with("::") {
    Some(create_colon_colon())
  } else {
    None
  };

  let rest = split
    .map(|v| (create_colon_colon(), ident_to_path_segment(v)))
    .collect();
  let trailing = None;

  Path {
    id,
    first,
    global,
    rest,
    trailing,
  }

  // let first = PathSegment::Ident(Ident::new(*split.peek));

  //
}

pub fn expand_braced(parent: &str, braced: &TokenStream) -> Result<Vec<Path>> {
  let mut parser = Parser::from_token_stream(braced);
  parser.parse::<OpenBrace>()?;
  let mut output = Vec::new();

  while !parser.is_eof()? {
    if parser.parse::<Option<CloseBrace>>()?.is_some() {
      break;
    }

    let ident = stringify(&parser.parse::<Ident>()?);
    parser.parse::<Option<T![,]>>()?;

    let name = format!("crate::script::tests::{}::{}", parent, ident);

    output.push(ident_to_path(&name));
  }

  Ok(output)
}

pub fn register_from_parser(mut parser: Parser) -> Result<TokenStream> {
  let mut mod_vec = Vec::new();

  while !parser.is_eof()? {
    let op = parser.parse::<rune::ast::Ident>()?;

    parser.parse::<rune::ast::generated::Rocket>()?;
    let braced = parser.parse::<rune::ast::Braced<Ident, T![,]>>()?;
    let braced = expand_braced(
      &stringify(&op).to_string(),
      &TokenStream::from_to_tokens(braced),
    )?;
    mod_vec.push((stringify(&op), braced));
  }
  parser.eof()?;
  let mut import_mod = quote!();
  let mut test_list = quote!();
  for (k, v) in mod_vec {
    for it in v {
      let lit = Lit::new(stringify_path(&it));
      test_list = quote!(#test_list #{
        path: #lit,
        location: #it
      },);
    }
    let k = Ident::new(&k);
    import_mod = quote!(#import_mod pub mod #k;);
  }
  test_list = quote!(
    pub fn test_list() {
      [ #test_list ]
    }
  );
  let import_mod = import_mod.into_token_stream();

  let tests_mod = import_mod.clone();
  let tests_mod = quote!(pub mod tests { #tests_mod });

  let module_mod = import_mod;
  let module_mod = quote!(pub mod module { #module_mod });

  let output = quote!(#tests_mod #module_mod #test_list);

  let token = output.into_token_stream();
  Ok(token)
}

pub fn register(stream: &TokenStream) -> Result<TokenStream> {
  register_from_parser(Parser::from_token_stream(stream))
}

pub fn load_module() -> Result<Module, ContextError> {
  let mut module = Module::with_crate_item("mado", &["testing"]);
  module.macro_(&["register"], register)?;

  Ok(module)
}
