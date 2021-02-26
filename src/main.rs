#![feature(proc_macro_hygiene, decl_macro)]
use rusqlite::{ Connection, params };
use rocket_contrib::serve::StaticFiles;

use std::error::Error;
use std::sync::{ Mutex, Arc };

mod internal_error;
mod notes;

use notes::endpoints;

#[macro_use] extern crate rocket;

fn main() -> Result<(), Box<dyn Error>> {
    let connection = Connection::open("rplanner.db")?;

    connection.execute("CREATE TABLE IF NOT EXISTS notes (date TEXT)", params![])?;
    connection.execute("CREATE TABLE IF NOT EXISTS note_text_elements (note_id INTEGER, content TEXT, num INTEGER)", params![])?;
    connection.execute("CREATE TABLE IF NOT EXISTS note_image_elements (note_id INTEGER, path TEXT, num INTEGER)", params![])?;
    let connection = Arc::new(Mutex::new(connection));

    rocket::ignite()
        .manage(connection.clone())
        .mount("/api", routes![endpoints::get_notes,
                               endpoints::set_note,
                               endpoints::add_note,
                               endpoints::delete_note,
                               endpoints::upload_image,
                               endpoints::get_image_list,
                               endpoints::insert_image,
                               endpoints::delete_fragment,
        ])
        .mount("/images", StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/images")))
        .mount("/", StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/web")).rank(15))
        .launch();

    Ok(())
}
