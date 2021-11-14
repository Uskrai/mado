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

macro_rules! with_dollar_signs {
  ($($body:tt)*) => {
    macro_rules! __with_dollar_sign { $($body)* }
    __with_dollar_sign!($)
  }
}

macro_rules! register_module {
  ($module:ident) => {
    $crate::macros::with_dollar_signs! {
      ($d:tt) => {
        #[allow(unused_macros)]
        macro_rules! register_type {
          ($d type:ty : $d skip:literal) => {};
          ($d type:ty) => {
            $module.ty::<$d type>()?;
          }
        }

        #[allow(unused_macros)]
        macro_rules! function {
          ($d ($d name:ident),+) => {
            $d(
              $module.function([stringify!($name)], $name)?;
            )+
          }
        }

        #[allow(unused_macros)]
        macro_rules! register_class {
          ($d($d type:ty $d(: $d skip:literal )? ),*) => {
            $d(
              register_type!($d type $d(: $d skip)?);
            )*

            $crate::macros::with_dollar_signs! {
              ($c:tt) => {
                #[allow(unused_macros)]
                macro_rules! associated_fn {
                  ($c($c name:ident), +) => {
                    $d(
                      $c(
                        $module.function(&[stringify!($type), stringify!($name)], <$type>::$name)?;
                      )+
                    )+
                  }
                }
                #[allow(unused_macros)]
                macro_rules! inst_fn {
                  ($c($c name:ident), +) => {
                    $d(
                      $c(
                        $module.inst_fn(stringify!($name), <$type>::$c name)?;
                      )+
                    )*
                  }
                }

                #[allow(unused_macros)]
                macro_rules! async_inst_fn {
                  ($c($c name:ident),+) => {
                    $d(
                      $c(
                        $module.async_inst_fn(stringify!($name), <$d type>::$name)?;
                      )+
                    )*
                  }
                }

                #[allow(unused_macros)]
                macro_rules! protocol_fn {
                  ($protocol:expr, $name:ident) => {
                    $d(
                      $module.inst_fn($protocol,<$d type>::$name)?;
                    )*
                  }
                }

                #[allow(unused_macros)]
                macro_rules! async_protocol_fn {
                  ($protocol:expr, $name:ident) => {
                    $d(
                      $mmodule.async_inst_fn($protocol, <$d type>::$name)?;
                    )*
                  }
                }
              }
            }

          }
        }
      }
    }
  };
}

pub(super) use register_module;
pub(super) use with_dollar_signs;
