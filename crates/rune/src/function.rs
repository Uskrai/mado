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

use std::sync::Arc;

use runestick::SyncFunction;

#[derive(Clone)]
pub struct DebugSyncFunction {
  inner: Arc<SyncFunction>,
}

impl std::ops::Deref for DebugSyncFunction {
  type Target = Arc<SyncFunction>;
  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl std::fmt::Debug for DebugSyncFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let ptr = &*self.inner as *const SyncFunction;
    f.debug_struct("SyncFunction").field("inner", &ptr).finish()
  }
}

impl From<Arc<SyncFunction>> for DebugSyncFunction {
  fn from(inner: Arc<SyncFunction>) -> Self {
    Self { inner }
  }
}
