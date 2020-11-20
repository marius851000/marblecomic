#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use maud::{html, Markup, DOCTYPE};

use rocket_contrib::serve::StaticFiles;

use rocket::State;

use marblecomic::{Comic, ComicDatabase};

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

fn create_link_to_comic(comic: &Comic) -> Markup {
    html!(
        a href=(format!("/comic/{}", comic.id)) {
            @if let Some(name) = &comic.comic_name {
                (name)
            } @else {
                "unnamed"
            }
        }
    )
}

#[get("/list")]
fn list_comic(comic_database: State<ComicDatabase>) -> Markup {
    present_page(
        html!(
            ul {
                @for (_, (_, comic)) in comic_database.comics().iter() {
                    @if comic.found {
                        li { (create_link_to_comic(comic)) }
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
) -> Option<Markup> {
    let comic = if let Some(comic) = comic_database.get_comic(comic_id) {
        comic
    } else {
        return None;
    };
    let navigation = comic_database.get_comic_navigation(comic.id).unwrap();
    let chap_navigation = if let Some(chap_navigation) = navigation.get(chap_id) {
        chap_navigation
    } else {
        return None;
    };

    let previous_chapter_id = chap_id.checked_sub(1);
    let next_chapter_id = if let Some(_) = navigation.get(chap_id + 1) {
        Some(chap_id + 1)
    } else {
        None
    };

    Some(present_page(
        html!(
            @for (page_id, option_path) in chap_navigation.iter().enumerate() {
                @if let Some(file_path) = option_path {
                    div class="page" {
                        p { "page " (page_id) }
                        img src=(format!("/image/comic/{}/chap/{}/{}.{}", comic.id, chap_id, page_id, file_path.extension().unwrap().to_str().unwrap())) {} //TODO: do not use unwrap
                        @if options.enable_progress_writing {
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
) -> File {
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
        todo!();
    } else {
        File::open(page_path).unwrap()
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
    keyword_section: String,
    keyword: String,
) -> Markup {
    //TODO: get rid of unwrap
    let keywords = comic_database.keywords();
    let keyword_comic_list = keywords
        .get(&keyword_section)
        .unwrap()
        .get(&keyword)
        .unwrap();
    present_page(
        html!(
            ul {
                @for comic_id in keyword_comic_list {
                    @let comic = comic_database.get_comic(*comic_id).unwrap();
                    li {
                        (create_link_to_comic(&comic))
                    }
                }
            }
        ),
        &format!("keyword {} ({})", keyword, keyword_section),
    )
}

pub struct MarbleOptions {
    pub enable_progress_writing: bool,
}

fn main() {
    let mut comic_database = ComicDatabase::default();
    let option = MarbleOptions {
        enable_progress_writing: false,
    };

    comic_database
        .load_from_dir(PathBuf::from("./comics"))
        .unwrap();

    rocket::ignite()
        .manage(comic_database)
        .manage(option)
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
                keyword_page
            ],
        )
        .launch();
}
