#[derive(rune::Any)]
pub struct Foo {
    //
}

impl Foo {
    fn bar(self) {
        //
    }
}

mado_rune_macros::register_module! {
  (Foo) => {
    inst => {
      bar
    },
    // bar, associated bar
  }
}
