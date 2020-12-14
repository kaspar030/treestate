use std::fmt::Debug;
use std::fs::metadata;
use std::hash::Hash;
use std::io::prelude::*;
use std::path::PathBuf;
use std::time::SystemTime;

use bincode;
use blake3;
use hashbrown::HashMap;
use rayon::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct FileState {
    len: u64,
    modified: SystemTime,
    hash: [u8; blake3::OUT_LEN],
}

impl FileState {
    fn blake3_from_file(file: &PathBuf) -> blake3::Hash {
        let data = std::fs::read(file).unwrap();
        blake3::hash(&data)
    }
}

impl State<PathBuf> for FileState {
    fn from(item: &PathBuf) -> Option<Self> {
        if let Ok(attr) = metadata(item) {
            Some(FileState {
                len: attr.len(),
                modified: attr.modified().unwrap(),
                hash: Self::blake3_from_file(item).into(),
            })
        } else {
            None
        }
    }

    fn has_changed(&self, other: &PathBuf) -> bool {
        if let Ok(attr) = metadata(other) {
            if self.len == attr.len() && self.modified == attr.modified().unwrap() {
                false
            } else {
                let other_hash: [u8; blake3::OUT_LEN] = Self::blake3_from_file(other).into();
                self.hash != other_hash
            }
        } else {
            true
        }
    }
}

pub trait State<T>: PartialEq + Sized {
    fn from(item: &T) -> Option<Self>;
    fn has_changed(&self, other: &T) -> bool;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TreeState<T, U>
where
    T: State<U>,
    U: Clone + Eq + Hash + Send + Sync,
{
    state: HashMap<U, T>,
}

impl<'a, 'de, T: State<U>, U: 'a> TreeState<T, U>
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
                .filter_map(|item| {
                    if let Some(state) = T::from(&item) {
                        Some((item.clone(), state))
                    } else {
                        None
                    }
                })
                .collect::<HashMap<_, _>>(),
        }
    }

    pub fn from<I>(items: I) -> TreeState<T, U>
    where
        I: IntoIterator<Item = (U, T)>,
    {
        TreeState {
            state: items.into_iter().collect::<HashMap<U, T>>(),
        }
    }

    pub fn has_changed(&self) -> bool {
        self.state
            .par_iter()
            .find_any(|(item, state)| state.has_changed(item))
            .is_some()
    }

    pub fn dump<W>(&self, w: &mut W) -> Result<(), std::io::Error>
    where
        W: Write,
    {
        let data: Vec<u8> = bincode::serialize(self).unwrap();

        w.write(&data[..])?;
        Ok(())
    }

    pub fn load<R>(r: R) -> bincode::Result<Self>
    where
        R: Read,
    {
        bincode::deserialize_from::<R, Self>(r)
    }

    pub fn load_vec(data: &Vec<u8>) -> bincode::Result<Self> {
        bincode::deserialize(data)
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
