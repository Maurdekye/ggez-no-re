use std::{
    cell::{Ref, RefCell, RefMut},
    error::Error,
    fs::File,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    rc::Rc,
};

use log::{debug, error, warn};
use serde::{Serialize, de::DeserializeOwned};

#[derive(Debug)]
pub struct PersistentRefMut<'a, T>
where
    T: Serialize,
{
    data: RefMut<'a, T>,
    save_path: &'a Path,
}

impl<T> Deref for PersistentRefMut<'_, T>
where
    T: Serialize,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for PersistentRefMut<'_, T>
where
    T: Serialize,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T> Drop for PersistentRefMut<'_, T>
where
    T: Serialize,
{
    fn drop(&mut self) {
        debug!("saving to {}", self.save_path.display());
        let result: Result<(), Box<dyn Error>> = try {
            let mut file = File::create(self.save_path)?;
            serde_json::to_writer_pretty(&mut file, &*self.data)?;
        };
        if let Err(err) = result {
            error!("Error saving persistent data: {err}");
        }
    }
}

#[derive(Debug, Clone)]
pub struct PersistenceManager<T> {
    data: Rc<RefCell<T>>,
    save_path: PathBuf,
}

impl<T> PersistenceManager<T> {
    pub fn new(save_path: &Path) -> PersistenceManager<T>
    where
        T: Default + Serialize + DeserializeOwned,
    {
        let result: Result<T, Box<dyn Error>> = try {
            let mut file = File::open(save_path)?;
            serde_json::from_reader(&mut file)?
        };
        let data = match result {
            Ok(data) => data,
            Err(err) => {
                warn!("Error loading persistent data: {err}");
                warn!("Loading defaults");
                let data = T::default();
                let result: Result<(), Box<dyn Error>> = try {
                    let mut file = File::create(save_path)?;
                    serde_json::to_writer_pretty(&mut file, &data)?;
                };
                if let Err(err) = result {
                    error!("Error saving default persistent data: {err}");
                }
                data
            }
        };
        let data = Rc::new(RefCell::new(data));
        let save_path = save_path.to_path_buf();
        PersistenceManager { data, save_path }
    }

    pub fn borrow(&self) -> Ref<'_, T> {
        self.data.borrow()
    }

    pub fn borrow_mut(&mut self) -> PersistentRefMut<'_, T>
    where
        T: Serialize,
    {
        PersistentRefMut {
            data: self.data.borrow_mut(),
            save_path: &self.save_path,
        }
    }
}
