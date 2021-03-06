use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::Comic;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TrackerReadError {
    #[error("can't decode a json value while trying to load reading progress")]
    DecodeError(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum TrackerSaveError {
    #[error("can't create the file to be saved to ({0})")]
    CantCreateFile(#[source] io::Error, PathBuf),
    #[error("can't serialize the data structure")]
    SerializeError(#[from] serde_json::Error),
    #[error("can't write to the file {0}")]
    CantWriteFile(#[source] io::Error, PathBuf),
}

#[derive(Default)]
pub struct Tracker {
    pub data: Mutex<HashMap<usize, (usize, usize)>>, //TODO: use dashmap
}

impl Tracker {
    pub fn new_from_reader<R: Read>(reader: R) -> Result<Self, TrackerReadError> {
        let data = Mutex::new(serde_json::from_reader(reader)?);
        Ok(Self { data })
    }

    pub fn get_progress(&self, comic: &Comic) -> (usize, usize) {
        let data = self.data
            .lock()
            .unwrap();
        if let Some(progress) = data.get(&comic.id) {
            return progress.clone()
        };
        for translated_id in comic.translations.iter().map(|(_, id)| id) {
            if let Some(progress) = data.get(&translated_id) {
                return progress.clone();
            }
        }
        (0, 0)
    }

    pub fn set_progress(&self, comic_id: usize, chapter_id: usize, image_id: usize) {
        self.data
            .lock()
            .unwrap()
            .insert(comic_id, (chapter_id, image_id));
    }

    pub fn list_comic_with_progress(&self) -> Vec<usize> {
        self.data.lock().unwrap().iter().map(|(k, _)| *k).collect()
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), TrackerSaveError> {
        //do not use a serde_json::to_writer, as an error in this case will result to the tracker file being empty
        let value_vec = serde_json::to_vec_pretty(&*self.data.lock().unwrap())?;
        let mut writer = File::create(path)
            .map_err(|err| TrackerSaveError::CantCreateFile(err, path.clone()))?;
        writer
            .write_all(&value_vec)
            .map_err(|err| TrackerSaveError::CantWriteFile(err, path.clone()))?;
        Ok(())
    }
}
