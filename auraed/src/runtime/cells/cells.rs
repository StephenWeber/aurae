/* -------------------------------------------------------------------------- *\
 *             Apache 2.0 License Copyright © 2022 The Aurae Authors          *
 *                                                                            *
 *                +--------------------------------------------+              *
 *                |   █████╗ ██╗   ██╗██████╗  █████╗ ███████╗ |              *
 *                |  ██╔══██╗██║   ██║██╔══██╗██╔══██╗██╔════╝ |              *
 *                |  ███████║██║   ██║██████╔╝███████║█████╗   |              *
 *                |  ██╔══██║██║   ██║██╔══██╗██╔══██║██╔══╝   |              *
 *                |  ██║  ██║╚██████╔╝██║  ██║██║  ██║███████╗ |              *
 *                |  ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝ |              *
 *                +--------------------------------------------+              *
 *                                                                            *
 *                         Distributed Systems Runtime                        *
 *                                                                            *
 * -------------------------------------------------------------------------- *
 *                                                                            *
 *   Licensed under the Apache License, Version 2.0 (the "License");          *
 *   you may not use this file except in compliance with the License.         *
 *   You may obtain a copy of the License at                                  *
 *                                                                            *
 *       http://www.apache.org/licenses/LICENSE-2.0                           *
 *                                                                            *
 *   Unless required by applicable law or agreed to in writing, software      *
 *   distributed under the License is distributed on an "AS IS" BASIS,        *
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. *
 *   See the License for the specific language governing permissions and      *
 *   limitations under the License.                                           *
 *                                                                            *
\* -------------------------------------------------------------------------- */

use super::{Cell, CellName, CellsError, Result};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, MutexGuard};

type Cache = HashMap<CellName, Cell>;

/// Cells is the in-memory store for the list of cells created with Aurae.
#[derive(Debug, Default, Clone)]
pub(crate) struct Cells {
    cache: Arc<Mutex<Cache>>,
}

// TODO: add to the impl
// - Get Cgroup from cell_name
// - Get Cgroup from executable_name
// - Get Cgroup from pid
// - Get Cgroup and pids from executable_name

impl Cells {
    /// Add the [Cell] to the cache with key [CellName].
    /// Returns an error if a duplicate [CellName] already exists in the cache.
    pub async fn allocate<T: Into<Cell>>(
        &self,
        cell_name: CellName,
        cell: T,
    ) -> Result<()> {
        let mut cache = self.cache.lock().await;
        let _ = allocate(&mut cache, cell_name, cell)?;
        Ok(())
    }

    /// See [allocate]
    pub async fn allocate_then<T: Into<Cell>, F, R>(
        &self,
        cell_name: CellName,
        cell: T,
        f: F,
    ) -> Result<R>
    where
        F: Fn(&Cell) -> Result<R>,
    {
        let mut cache = self.cache.lock().await;
        let cell = allocate(&mut cache, cell_name, cell)?;
        f(cell)
    }

    pub async fn get<F, R>(&self, cell_name: &CellName, f: F) -> Result<R>
    where
        F: Fn(&Cell) -> Result<R>,
    {
        let mut cache = self.cache.lock().await;
        if let Some(cell) = cache.get(cell_name) {
            let res = f(cell);
            if matches!(res, Err(CellsError::CellUnallocated { .. })) {
                let _ = cache.remove(cell_name);
            }
            res
        } else {
            Err(CellsError::CellNotFound { cell_name: cell_name.clone() })
        }
    }

    pub async fn get_mut<F, R>(&self, cell_name: &CellName, f: F) -> Result<R>
    where
        F: FnOnce(&mut Cell) -> Result<R>,
    {
        let mut cache = self.cache.lock().await;
        get_mut(&mut cache, cell_name, f)
    }

    /// Returns an error if the [CellName] does not exist in the cache.
    pub async fn free(&self, cell_name: &CellName) -> Result<()> {
        let mut cache = self.cache.lock().await;
        get_mut(&mut cache, cell_name, |cell| cell.free())?;
        let _ = cache.remove(cell_name).ok_or_else(|| {
            CellsError::CellNotFound { cell_name: cell_name.clone() }
        })?;
        Ok(())
    }
}

fn allocate<'a, T: Into<Cell>>(
    cache: &'a mut MutexGuard<Cache>,
    cell_name: CellName,
    cell: T,
) -> Result<&'a Cell> {
    // TODO: replace with this when it becomes stable
    // cache.try_insert(cell_name.clone(), cgroup)

    // Check if there was already a cgroup in the table with this cell name as a key.
    if cache.contains_key(&cell_name) {
        return Err(CellsError::CellExists { cell_name });
    }

    // `or_insert` will always insert as we've already assured ourselves that the key does not exist.
    let cell = cache.entry(cell_name).or_insert_with(|| cell.into());
    cell.allocate();
    Ok(cell)
}

fn get_mut<F, R>(
    cache: &mut MutexGuard<Cache>,
    cell_name: &CellName,
    f: F,
) -> Result<R>
where
    F: FnOnce(&mut Cell) -> Result<R>,
{
    if let Some(cell) = cache.get_mut(cell_name) {
        let res = f(cell);
        if matches!(res, Err(CellsError::CellUnallocated { .. })) {
            let _ = cache.remove(cell_name);
        }
        res
    } else {
        Err(CellsError::CellNotFound { cell_name: cell_name.clone() })
    }
}

#[cfg(test)]
mod tests {
    // TODO (future-highway): These tests need to be updated.
    // use cgroups_rs::{cgroup_builder::CgroupBuilder, hierarchies};
    //
    // use super::*;

    // #[test]
    // fn test_insert() {
    //     let table = CellsTable::default();
    //     let cgroup = CgroupBuilder::new("test-cell")
    //         .build(Box::new(hierarchies::V2::new()));
    //     {
    //         let cache = table.cache.lock().expect("lock table");
    //         assert!(cache.is_empty());
    //     }
    //     table.insert("test".into(), cgroup).expect("inserted in table");
    //     {
    //         let mut cache = table.cache.lock().expect("lock table");
    //         assert!(cache.contains_key(&"test".into()));
    //         cache.clear();
    //     }
    // }
    //
    // #[test]
    // fn test_duplicate_insert_is_error() {
    //     let table = CellsTable::default();
    //     let cgroup = CgroupBuilder::new("test-cell")
    //         .build(Box::new(hierarchies::V2::new()));
    //     {
    //         let cache = table.cache.lock().expect("lock table");
    //         assert!(cache.is_empty());
    //     }
    //     table.insert("test".into(), cgroup.clone()).expect("inserted in table");
    //     assert!(table.insert("test".into(), cgroup).is_err());
    //     {
    //         let mut cache = table.cache.lock().expect("lock table");
    //         cache.clear();
    //     }
    // }
    //
    // #[test]
    // fn test_get() {
    //     let table = CellsTable::default();
    //     let cgroup = CgroupBuilder::new("test-cell")
    //         .build(Box::new(hierarchies::V2::new()));
    //     {
    //         let cache = table.cache.lock().expect("lock table");
    //         assert!(cache.is_empty());
    //     }
    //     table.insert("test".into(), cgroup).expect("inserted in table");
    //     assert!(table
    //         .get(&"test".into())
    //         .expect("getting from cache")
    //         .is_some());
    //     {
    //         let mut cache = table.cache.lock().expect("lock table");
    //         cache.clear();
    //     }
    // }
    //
    // #[test]
    // fn test_get_missing_returns_none() {
    //     let table = CellsTable::default();
    //     {
    //         let cache = table.cache.lock().expect("lock table");
    //         assert!(cache.is_empty());
    //     }
    //
    //     assert!(table
    //         .get(&"test".into())
    //         .expect("getting from cache")
    //         .is_none());
    // }
    //
    // #[test]
    // fn test_remove() {
    //     let table = CellsTable::default();
    //     let cgroup = CgroupBuilder::new("test-cell")
    //         .build(Box::new(hierarchies::V2::new()));
    //     {
    //         let cache = table.cache.lock().expect("lock table");
    //         assert!(cache.is_empty());
    //     }
    //     table.insert("test".into(), cgroup).expect("inserted in table");
    //     let _ = table.remove(&"test".into()).expect("removed from table");
    //     {
    //         let mut cache = table.cache.lock().expect("lock table");
    //         assert!(cache.is_empty());
    //         cache.clear();
    //     }
    // }
    //
    // #[test]
    // fn test_remove_missing_is_error() {
    //     let table = CellsTable::default();
    //     {
    //         let cache = table.cache.lock().expect("lock table");
    //         assert!(cache.is_empty());
    //     }
    //     assert!(table.remove(&"test".into()).is_err());
    //     {
    //         let mut cache = table.cache.lock().expect("lock table");
    //         cache.clear();
    //     }
    // }
}