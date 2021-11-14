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

// use std::fmt::Debug;
// use std::sync::Arc;
//
// use runestick::{Any, Struct, ToValue, Value, VmError};
// use mado_rune_macros::register_module;
// use tokio::sync::{
//   RwLock as TokioRwLock, RwLockReadGuard as TokioRwLockReadGuard,
// };
//
// macro_rules! create_rwlock {
//   ($type:ty, $rwtype:ident, $guardtype:ident) => {
//     #[derive(Any, Debug, Clone)]
//     pub struct $rwtype {
//       lock: Arc<TokioRwLock<Value>>,
//     }
//
//     #[derive(Any)]
//     pub struct $guardtype {
//       guard: TokioRwLockReadGuard<'static, Value>,
//     }
//
//     unsafe impl Send for $guardtype {}
//     unsafe impl Send for $rwtype {}
//     unsafe impl Sync for $rwtype {}
//
//     impl $rwtype {
//       pub fn new(t: $type) -> Result<Self, VmError> {
//         Ok(Self {
//           lock: Arc::new(TokioRwLock::new(t.to_value()?)),
//         })
//       }
//
//       pub async fn read(&'static self) -> $guardtype {
//         $guardtype {
//           guard: self.lock.read().await,
//         }
//       }
//     }
//
//     impl $guardtype {
//       pub fn data(&self) -> Value {
//         self.guard.clone()
//       }
//     }
//
//     pub fn load_module() -> Result<runestick::Module, runestick::ContextError> {
//       register_module! {
//         ($rwtype) => {
//           async_inst => {
//             read
//           }
//         },
//         ($guardtype) => {
//           inst => {
//             data
//           }
//         }
//       }
//
//       load_module_with(runestick::Module::new())
//     }
//   };
// }
//
// create_rwlock!(Struct, RwLockStruct, RwLockReadGuardStruct);
//
// // #[derive(Any, Debug, Clone)]
// // pub struct RwLock<T>
// // where
// //   T: ToValue + 'static,
// // {
// //   lock: Arc<TokioRwLock<T>>,
// // }
// //
// // #[derive(Any, Debug)]
// // pub struct RwLockReadGuard {
// //   guard: TokioRwLockReadGuard<'static, Value>,
// // }
// //
// // impl<T> RwLock<T>
// // where
// //   T: ToValue + 'static + Debug,
// // {
// //   pub fn new(t: T) -> Result<Self, VmError> {
// //     Ok(Self {
// //       lock: Arc::new(TokioRwLock::new(t)),
// //     })
// //   }
// //
// //   pub async fn read(&'static self) -> RwLockReadGuard {
// //     RwLockReadGuard {
// //       guard: self.lock.read().await,
// //     }
// //   }
// //
// //   pub fn to_string_debug(&self, s: &mut String) -> std::fmt::Result {
// //     write!(s, "{:?}", self)
// //   }
// // }
// //
// // impl RwLockReadGuard {
// //   pub fn data(&self) -> &Value {
// //     &self.guard
// //   }
// // }
// //
// // fn load_module() {
// //   register_module! {
// //     (RwLock) => {
// //       async_inst => {
// //         read
// //       }
// //     },
// //     (RwLockReadGuard) => {
// //       inst => {
// //         data
// //       }
// //     }
// //   }
// // }
