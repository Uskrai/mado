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

#[tokio::test]
async fn test_load_module() {
  let scripts = std::fs::read_dir("script").unwrap();

  let mut errors = Vec::new();

  let build_module =
    |path: &std::path::PathBuf| -> Result<_, Box<dyn std::error::Error>> {
      Ok(
        mado_rune::Build::default()
          .with_path(path)?
          .build_for_module()?
          .error_missing_load_module(false)
          .build()?,
      )
    };

  for it in scripts.into_iter() {
    let it = it.unwrap();
    if it.path().is_file() {
      let module = build_module(&it.path());

      match module {
        Ok(_) => {}
        Err(err) => {
          errors.push((it.path(), err));
        }
      }
    }
  }

  if !errors.is_empty() {
    for (path, err) in errors {
      println!("error on {}: {}", path.to_string_lossy(), err);
    }
    panic!("Error");
  }
}

#[test]
fn test_mangadex() {
  // let rt = tokio::runtime::Runtime::new().unwrap();
  //
  // rt.block_on(async move {
  //   let loaded = {
  //     let scripts = std::path::Path::new("script/module/mangadex.rn");
  //
  //     println!("{}", scripts.display());
  //
  //     let mut source = rune::Sources::new();
  //     source.insert(rune::compile::from_path(scripts).unwrap());
  //
  //     loader::load([source].into_iter())
  //       .await
  //       .unwrap()
  //       .swap_remove(0)
  //   };
  //
  //   let url = url::Url::parse("https://mangadex.org/title/bd6d0982-0091-4945-ad70-c028ed3c0917/mushoku-tensei-isekai-ittara-honki-dasu").unwrap();
  //
  //   let create_fut = || {
  //     let loaded = loaded.clone();
  //     let url = url.clone();
  //     tokio::spawn(async move {
  //       println!("{:#?}", std::thread::current().id());
  //       let res = loaded.clone().get_info(url.clone()).await;
  //       println!("{:#?}", std::thread::current().id());
  //       res
  //     })
  //   };
  //
  //   let fut1 = create_fut();
  //   let fut2 = create_fut();
  //   let fut3 = create_fut();
  //   let fut4 = create_fut();
  //   let fut5 = create_fut();
  //   let fut6 = create_fut();
  //   let fut7 = create_fut();
  //   let fut8 = create_fut();
  //   let fut9 = create_fut();
  //
  //   println!("first:{:#?}", std::thread::current().id());
  //   let _ = join!(fut1, fut2, fut3, fut4, fut5, fut6, fut7, fut8, fut9);
  //   println!("last:{:#?}", std::thread::current().id());
  // });
  // println!("{:#?}", val);
}
