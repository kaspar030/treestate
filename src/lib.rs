use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::metadata;
use std::hash::Hash;
use std::io::prelude::*;
use std::path::PathBuf;
use std::time::SystemTime;

use rayon::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct FileState {
    len: u64,
    modified: SystemTime,
}

impl State<PathBuf> for FileState {
    fn from(item: &PathBuf) -> Option<Self> {
        metadata(item)
            .map(|attr| FileState {
                len: attr.len(),
                modified: attr.modified().unwrap(),
            })
            .ok()
    }
}

pub trait State<T>: PartialEq + Sized {
    fn from(item: &T) -> Option<Self>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TreeState<T, U>
where
    T: State<U>,
    U: Clone + Hash + Eq + Send + Sync,
{
    state: HashMap<U, T>,
}

impl<'a, T: State<U>, U: 'a> TreeState<T, U>
where
    T: Serialize + DeserializeOwned + Debug + Send + Sync,
    U: Clone + Eq + Hash + Serialize + DeserializeOwned + Send + Sync,
{
    pub fn new<I>(items: I) -> TreeState<T, U>
    where
        I: IntoIterator<Item = &'a U>,
    {
        TreeState {
            state: items
                .into_iter()
                .filter_map(|item| T::from(item).map(|state| (item.clone(), state)))
                .collect::<HashMap<_, _>>(),
        }
    }

    pub fn has_changed(&self) -> bool {
        self.state
            .par_iter()
            .find_any(|(item, state)| T::from(item).map_or(true, |x| x != **state))
            .is_some()
    }

    pub fn dump<W>(&self, w: W) -> bincode::Result<()>
    where
        W: Write,
    {
        bincode::serialize_into(w, self)
    }

    pub fn load<R>(r: R) -> bincode::Result<Self>
    where
        R: Read,
    {
        bincode::deserialize_from::<R, Self>(r)
    }

    pub fn ignore(&mut self, item: &U) {
        self.state.remove(item);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn basic() {}
}
