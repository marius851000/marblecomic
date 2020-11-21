use serde::{Deserialize, Serialize};
use vec_map::VecMap;

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fs::{read_dir, File};
use std::io;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::sync::Mutex;

use thiserror::Error;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Comic {
    pub id: usize,
    pub comic_name: Option<String>,
    pub description: Option<String>,
    pub keywords: HashMap<String, Vec<String>>,
    pub translations: Vec<(String, usize)>,
    pub found: bool,
}

#[derive(Default)]
pub struct ComicDatabase {
    comics: VecMap<(PathBuf, Comic)>,
    keywords: HashMap<String, HashMap<String, Vec<usize>>>,
    navigation_cache: Mutex<VecMap<Vec<Vec<Option<PathBuf>>>>>,
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

#[derive(Error, Debug)]
pub enum GetComicNavigationError {
    #[error("failed to list sub content of {1}")]
    CantReadDirectory(#[source] io::Error, PathBuf),
    #[error("failed to read an entry of the content of the director {1}")]
    CantReadDirEntry(#[source] io::Error, PathBuf),
    #[error("this comic ({0}) doesn't exist")]
    ComicDontExist(usize),
    #[error("the file at {0} doesn't have a file name, but one is required")]
    FileWithNoName(PathBuf),
    #[error("can't convert an OsStr to a String")]
    CantConvertOsToString(OsString),
    #[error("the file at {0} doesn't have a stem (file name without extension)")]
    FileWithNoStem(PathBuf),
    #[error("can't get the {0} value of {1} when splited by '-' (count start at 0)")]
    CantGetSplitedDash(u32, String),
    #[error("can't convert the value {1} from {2} to an usize")]
    CantConvertStringFromPathToInt(ParseIntError, String, PathBuf),
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
                let mut new_category_map: HashMap<String, Vec<usize>> = HashMap::new();
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
        };

        if !self.keywords.contains_key("translation") {
            self.keywords.insert("translation".into(), HashMap::new());
        };
        let trans_hashmap = self.keywords.get_mut("translation").unwrap(); //TODO: check for a get_key_or_create
        for (trans_lang, trans_comic_id) in &comic.translations {
            if *trans_comic_id == comic.id {
                if let Some(lang_vec) = trans_hashmap.get_mut(trans_lang) {
                    lang_vec.push(comic.id);
                } else {
                    trans_hashmap.insert(trans_lang.to_string(), vec![comic.id]);
                }
            }
        };
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

    pub fn comics(&self) -> &VecMap<(PathBuf, Comic)> {
        &self.comics
    }

    pub fn get_comic(&self, id: usize) -> Option<&Comic> {
        self.comics.get(id).map(|pair| &pair.1)
    }

    //TODO: get the section name
    pub fn get_comic_navigation(
        &self,
        id: usize,
    ) -> Result<Vec<Vec<Option<PathBuf>>>, GetComicNavigationError> {
        let mut navigation_cache_lock = self.navigation_cache.lock().unwrap();

        if let Some(cached) = navigation_cache_lock.get(id) {
            return Ok((*cached).clone());
        };

        fn osstr_to_str(osstr: &OsStr) -> Result<&str, GetComicNavigationError> {
            osstr.to_str().map_or(
                Err(GetComicNavigationError::CantConvertOsToString(
                    osstr.to_os_string(),
                )),
                |x| Ok(x),
            )
        };

        let mut result = Vec::new();
        let comic_directory = &self
            .comics
            .get(id)
            .map_or(Err(GetComicNavigationError::ComicDontExist(id)), |x| Ok(x))?
            .0;

        let paths = read_dir(comic_directory).map_err(|err| {
            GetComicNavigationError::CantReadDirectory(err, comic_directory.clone())
        })?;
        for path in paths {
            let path = path
                .map_err(|err| {
                    GetComicNavigationError::CantReadDirEntry(err, comic_directory.clone())
                })?
                .path();
            let file_name = osstr_to_str(path.file_name().map_or(
                Err(GetComicNavigationError::FileWithNoName(path.clone())),
                |x| Ok(x),
            )?)?;

            if file_name == "data.json" {
                continue;
            };

            if let Some(Some("tmp")) = path.extension().map(|x| x.to_str()) {
                continue;
            };

            let file_stem = osstr_to_str(path.file_stem().map_or(
                Err(GetComicNavigationError::FileWithNoStem(path.clone())),
                |x| Ok(x),
            )?)?;

            let (part, page) = {
                let mut splited = file_stem.split("-");
                let first_part_string = splited.next().map_or(
                    Err(GetComicNavigationError::CantGetSplitedDash(
                        0,
                        file_stem.to_string(),
                    )),
                    |x| Ok(x),
                )?;
                let first_part = first_part_string.parse::<usize>().map_err(|err| {
                    GetComicNavigationError::CantConvertStringFromPathToInt(
                        err,
                        first_part_string.to_string(),
                        path.clone(),
                    )
                })?;

                let second_part_string = splited.next().map_or(
                    Err(GetComicNavigationError::CantGetSplitedDash(
                        0,
                        file_stem.to_string(),
                    )),
                    |x| Ok(x),
                )?;

                let second_part = second_part_string.parse::<usize>().map_err(|err| {
                    GetComicNavigationError::CantConvertStringFromPathToInt(
                        err,
                        first_part_string.to_string(),
                        path.clone(),
                    )
                })?;

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

        navigation_cache_lock.insert(id, result.clone());
        Ok(result)
    }

    pub fn keywords(&self) -> &HashMap<String, HashMap<String, Vec<usize>>> {
        &self.keywords
    }
}
