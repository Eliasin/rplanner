use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub type DBConnection = Arc<Mutex<Connection>>;
