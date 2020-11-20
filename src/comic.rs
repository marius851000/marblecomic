use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fs::{read_dir, File};
use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Comic {
    pub id: u64,
    pub comic_name: Option<String>,
    pub description: Option<String>,
    pub keywords: HashMap<String, Vec<String>>,
    pub translations: Vec<(String, u64)>,
    pub found: bool,
}

#[derive(Default)]
pub struct ComicDatabase {
    comics: HashMap<u64, (PathBuf, Comic)>,
    keywords: HashMap<String, HashMap<String, Vec<u64>>>,
}

#[derive(Error, Debug)]
pub enum ComicDatabaseLoadError {
    #[error("failed to list sub content of {1}")]
    CantReadDirectory(#[source] io::Error, PathBuf),
    #[error("failed to read an entry of the content of the director {1}")]
    CantReadDirEntry(#[source] io::Error, PathBuf),
    #[error("failed to open file at {1}")]
    CantOpenFile(#[source] io::Error, PathBuf),
    #[error("failed to deserialize a comic data file at {1}")]
    CantDeserializeComic(#[source] serde_json::Error, PathBuf),
}

impl ComicDatabase {
    pub fn add_comic(&mut self, path: PathBuf, comic: Comic) {
        for (keyword_category, values) in &comic.keywords {
            if let Some(keyword_hashmap) = self.keywords.get_mut(keyword_category) {
                for section_name in values {
                    if let Some(section_vec) = keyword_hashmap.get_mut(section_name) {
                        section_vec.push(comic.id);
                    } else {
                        keyword_hashmap.insert(section_name.to_string(), vec![comic.id]);
                    }
                }
            } else {
                let mut new_category_map: HashMap<String, Vec<u64>> = HashMap::new();
                for section_name in values {
                    if let Some(section_vec) = new_category_map.get_mut(section_name) {
                        section_vec.push(comic.id);
                    } else {
                        new_category_map.insert(section_name.to_string(), vec![comic.id]);
                    }
                }
                self.keywords
                    .insert(keyword_category.clone(), new_category_map);
            }
        }
        self.comics.insert(comic.id, (path, comic));
    }

    pub fn load_from_dir(&mut self, folder: PathBuf) -> Result<(), ComicDatabaseLoadError> {
        let paths = read_dir(&folder)
            .map_err(|err| ComicDatabaseLoadError::CantReadDirectory(err, folder.clone()))?;
        for path in paths {
            let mut data_path = path
                .map_err(|err| ComicDatabaseLoadError::CantReadDirEntry(err, folder.clone()))?
                .path();
            let folder_path = data_path.clone();
            data_path.push("data.json");

            if data_path.exists() {
                println!("subfolder name: {:?}", data_path);
                let data_file = File::open(&data_path)
                    .map_err(|err| ComicDatabaseLoadError::CantOpenFile(err, data_path.clone()))?;
                let comic: Comic = serde_json::from_reader(data_file).map_err(|err| {
                    ComicDatabaseLoadError::CantDeserializeComic(err, data_path.clone())
                })?;

                if comic.found {
                    self.add_comic(folder_path, comic);
                };
            };
        }
        Ok(())
    }

    pub fn comics(&self) -> &HashMap<u64, (PathBuf, Comic)> {
        &self.comics
    }

    pub fn get_comic(&self, id: u64) -> Option<&Comic> {
        self.comics.get(&id).map(|pair| &pair.1)
    }

    //TODO: rewrite so name are saved
    //TODO: cache
    pub fn get_comic_navigation(&self, id: u64) -> Vec<Vec<Option<PathBuf>>> {
        let mut result = Vec::new();
        println!("{:?}", &self.comics.get(&id).unwrap().0);
        let paths = read_dir(&self.comics.get(&id).unwrap().0).unwrap(); //TODO: get rid of unwrap
        for path in paths {
            let path = path.unwrap().path(); //TODO: don't unwrap
            let file_name = path.file_name().unwrap().to_str().unwrap(); //TODO: don't unwrap
            if file_name == "data.json" {
                continue;
            };
            if file_name.split(".").last().unwrap() == "tmp" {
                continue;
            }; //TODO: don't unwrap
            let file_name_without_extension = file_name.split(".").next().unwrap();

            let (part, page) = {
                let mut splited = file_name_without_extension.split("-");
                let first_part = splited.next().unwrap().parse::<u64>().unwrap();
                let second_part = splited.next().unwrap().parse::<u64>().unwrap();
                //TODO: handle the case with multiple document by page
                (first_part, second_part)
            };

            while result.len() <= part as usize {
                result.push(Vec::new())
            }

            while result[part as usize].len() <= page as usize {
                result[part as usize].push(None)
            }

            result[part as usize][page as usize] = Some(path.into());
        }

        result
    }

    pub fn keywords(&self) -> &HashMap<String, HashMap<String, Vec<u64>>> {
        &self.keywords
    }
}
