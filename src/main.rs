#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use maud::{html, Markup, DOCTYPE};

use rocket_contrib::serve::StaticFiles;

use rocket::{State, response::status::{NotFound, Forbidden}};

use marblecomic::{Comic, ComicDatabase, Tracker};

use std::fs::File;
use std::path::PathBuf;

fn present_page(content: Markup, title: &str) -> Markup {
    html!(
        (DOCTYPE)
        head {
            meta charset = "utf-8" {}
            title { (title) }
            link rel="stylesheet" href="/static/marble.css" {}
        }
        body {
            div id="header" {
                h1 { (title) }
                ul id="mainmenu" {
                    li { a href="/" { "main page" }}
                    li { a href="/list" { "comic list" }}
                    li { a href="/keywords" { "keywords" }}
                }
            }
            (content)
        }
    )
}

fn present_error(error_message: &str, internal: bool) -> Markup {
    present_page(html!(
        div class="errormessage" {
            p {
                @if internal {
                    "An internal error occured."
                    br {}
                    "That mean it is an error from the programmer or the server admin (or this error is eroneously reported as being an internal error)"
                    br {}
                    "please content the admin with this page url"
                    br {}
                }
                @else {
                    "An problem occured."
                    br {}
                    "If you think this is an error with the server, please contact the server administrater with the page url"
                    br {}
                }
                "error message : " (error_message)


            }
        }
    ), if internal {"internal error"} else {"error"})
}

fn create_link_to_comic(
    comic: &Comic,
    tracker: &Tracker,
    comic_database: &ComicDatabase,
) -> Markup {
    let progress = tracker.get_progress(comic.id);
    let have_progress = progress != (0, 0);
    let navigation = comic_database.get_comic_navigation(comic.id).unwrap(); //TODO: proper error handling
    let finished = if have_progress {
        if navigation.len() == progress.0 + 1 {
            let navigation_progress_chapter = navigation.get(progress.0).unwrap();
            if navigation_progress_chapter.len() == progress.1 + 1 {
                true
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };
    html!(
        a href=(format!("/comic/{}", comic.id)) {
            @if let Some(name) = &comic.comic_name {
                (name)
            } @else {
                "unnamed"
            }
            @if have_progress {
                " "
                @if finished {
                    "(finished)"
                } @else {
                    " (currently at chapter " (progress.0) " image " (progress.1) ")"
                }
            }
        }
    )
}

#[get("/list")]
fn list_comic(comic_database: State<ComicDatabase>, tracker: State<Tracker>) -> Markup {
    present_page(
        html!(
            ul {
                @for (_, (_, comic)) in comic_database.comics().iter() {
                    @if comic.found {
                        li { (create_link_to_comic(comic, &*tracker, &*comic_database)) }
                    }
                }
            }
        ),
        "comic list",
    )
}

#[get("/comic/<comic_id>")]
fn display_comic_page(comic_database: State<ComicDatabase>, comic_id: u64) -> Option<Markup> {
    let comic = if let Some(comic) = comic_database.get_comic(comic_id) {
        comic
    } else {
        return None;
    };
    Some(present_page(
        html!(
            ul {
                @for translation in &comic.translations {
                    li {
                        a href=(format!("/comic/{}", translation.1)) {
                            (translation.0)
                        }
                    }
                }
            }

            @if let Some(description) = &comic.description {
                h2 { "description" }

                (description)
            }

            h2 { "parts" }

            ul {
                @for (chap_id, _) in comic_database.get_comic_navigation(comic.id).unwrap().iter().enumerate() {
                    li {
                        a href=(format!("/comic/{}/chap/{}", comic.id, chap_id)) {
                            "chapter " (chap_id)
                        }
                    }
                }
            }


        ),
        if let Some(name) = &comic.comic_name {
            name
        } else {
            "unnamed"
        },
    ))
}

#[get("/comic/<comic_id>/chap/<chap_id>")]
fn display_chapter_page(
    comic_id: u64,
    chap_id: usize,
    comic_database: State<ComicDatabase>,
    options: State<MarbleOptions>,
) -> Result<Markup, NotFound<Markup>> {
    let comic = if let Some(comic) = comic_database.get_comic(comic_id) {
        comic
    } else {
        return Err(NotFound(present_error("comic not found", false)))
    };
    let navigation = comic_database.get_comic_navigation(comic.id).unwrap();
    let chap_navigation = if let Some(chap_navigation) = navigation.get(chap_id) {
        chap_navigation
    } else {
        return Err(NotFound(present_error("chapter not found", false)));
    };

    let previous_chapter_id = chap_id.checked_sub(1);
    let next_chapter_id = if let Some(_) = navigation.get(chap_id + 1) {
        Some(chap_id + 1)
    } else {
        None
    };

    Ok(present_page(
        html!(
            @for (page_id, option_path) in chap_navigation.iter().enumerate() {
                @if let Some(file_path) = option_path {
                    div class="page" {
                        p { "page " (page_id) }
                        img src=(format!("/image/comic/{}/chap/{}/{}.{}", comic.id, chap_id, page_id, file_path.extension().unwrap().to_str().unwrap())) {} //TODO: do not use unwrap
                        @if options.enable_progress_writing {
                            br {}
                            a href=(format!("/set_progress/{}/{}/{}", comic.id, chap_id, page_id)) {
                                "set progress to this page"
                            }
                        }
                    }
                }
            }

            @if let Some(previous_chapter_id) = previous_chapter_id {
                p {
                    a href=(format!("/comic/{}/chap/{}", comic.id, previous_chapter_id)) { "previous chapter" }
                }
            }

            @if let Some(next_chapter_id) = next_chapter_id {
                p {
                    a href=(format!("/comic/{}/chap/{}", comic.id, next_chapter_id)) { "next chapter" }
                }
            }
        ),
        (if let Some(name) = &comic.comic_name {
            format!("{}, chap {}", name, chap_id)
        } else {
            format!("chap {} of an unnamed comic", chap_id)
        })
        .as_ref(),
    ))
}

#[get("/image/comic/<comic_id>/chap/<chap_id>/<page_id_and_extension>")]
fn send_picture(
    comic_database: State<ComicDatabase>,
    comic_id: u64,
    chap_id: usize,
    page_id_and_extension: String,
) -> Result<File, NotFound<Markup>> {
    //TODO: get rid of unwrap
    let navigation = comic_database.get_comic_navigation(comic_id).unwrap();
    let navigation_chapter = navigation.get(chap_id).unwrap();

    let page_id_and_extension_path = PathBuf::from(page_id_and_extension);
    let page_stem = page_id_and_extension_path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap();
    let page_id = page_stem.parse::<usize>().unwrap();

    let page_path = navigation_chapter.get(page_id).unwrap().as_ref().unwrap();
    if page_path.extension() != page_id_and_extension_path.extension() {
        Err(NotFound(present_error("the extension does not match the expected one", false)))
    } else {
        Ok(File::open(page_path).unwrap())
    }
}

#[get("/")]
fn index() -> Markup {
    present_page(
        html!(p { "main page, nothing here (yet), use the upper menu to navigate" }),
        "MarbleComic",
    )
}

#[get("/keywords")]
fn list_keywords(comic_database: State<ComicDatabase>) -> Markup {
    let keywords = comic_database.keywords();
    present_page(
        html!(
            @for (keyword_section_name, keyword_section_data) in keywords {
                h2 { (keyword_section_name) }
                ul class="keyword_list" {
                    @for (keyword, _) in keyword_section_data {
                        li {
                            a href=(format!("keyword/{}/{}", keyword_section_name, keyword)) {
                                (keyword)
                            }
                        }
                    }
                }
            }
        ),
        "keywords",
    )
}

#[get("/keyword/<keyword_section>/<keyword>")]
fn keyword_page(
    comic_database: State<ComicDatabase>,
    tracker: State<Tracker>,
    keyword_section: String,
    keyword: String,
) -> Result<Markup, NotFound<Markup>> {
    //TODO: get rid of unwrap
    let keywords = comic_database.keywords();
    let keyword_comic_list = keywords
        .get(&keyword_section)
        .map_or(Err(NotFound(present_error("keyword section is unknown", false))), |x| Ok(x))?
        .get(&keyword)
        .map_or(Err(NotFound(present_error("keyword is unknwon", false))), |x| Ok(x))?;

    Ok(present_page(
        html!(
            ul {
                @for comic_id in keyword_comic_list {
                    @let comic = comic_database.get_comic(*comic_id).unwrap();
                    li {
                        (create_link_to_comic(&comic, &tracker, &*comic_database))
                    }
                }
            }
        ),
        &format!("keyword {} ({})", keyword, keyword_section),
    ))
}

#[get("/set_progress/<comic_id>/<chapter_id>/<image_id>")]
fn set_progress(
    tracker: State<Tracker>,
    option: State<MarbleOptions>,
    comic_id: u64,
    chapter_id: usize,
    image_id: usize,
) -> Result<Markup, Forbidden<Markup>> {
    if option.enable_progress_writing {
        tracker.set_progress(comic_id, chapter_id, image_id);
        tracker.save(&option.tracker_path).unwrap();
        Ok(present_page(
            html!(
                "the progess is sucessfully save." br {}
                a href=(format!("/comic/{}/chap/{}", comic_id, chapter_id)) {
                    "return to this comic page"
                }
            ),
            "progress saved",
        ))
    } else {
        Err(Forbidden(Some(present_error("progress saving are disabled on this server", false))))
    }
}
pub struct MarbleOptions {
    pub enable_progress_writing: bool,
    pub tracker_path: PathBuf,
}

fn main() {
    let tracker_path = PathBuf::from("./progress.json");

    let tracker = if let Ok(tracker_file) = File::open(&tracker_path) {
        Tracker::new_from_reader(tracker_file).unwrap()
    } else {
        Tracker::default()
    };

    let mut comic_database = ComicDatabase::default();
    let option = MarbleOptions {
        enable_progress_writing: true,
        tracker_path,
    };

    comic_database
        .load_from_dir(PathBuf::from("/run/media/marius/f0785b86-0e54-43be-9bb0-03da4436baec/canterlotcomics/backup"))
        .unwrap();

    rocket::ignite()
        .manage(comic_database)
        .manage(option)
        .manage(tracker)
        .mount("/static", StaticFiles::from("static"))
        .mount(
            "/",
            routes![
                list_comic,
                display_comic_page,
                display_chapter_page,
                send_picture,
                index,
                list_keywords,
                keyword_page,
                set_progress
            ],
        )
        .launch();
}
