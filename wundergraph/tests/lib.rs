extern crate diesel;
extern crate diesel_migrations;
extern crate juniper;
extern crate serde_json;
extern crate wundergraph;
extern crate wundergraph_bench;
extern crate wundergraph_example;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate insta;

mod helper;

mod query;
mod query_nested;
mod simple;
mod order;
mod limit_offset;
mod type_checking;
mod alias;
mod mutations;

#[cfg(feature = "postgres")]
type DbConnection = diesel::pg::PgConnection;

#[cfg(feature = "sqlite")]
type DbConnection = diesel::sqlite::SqliteConnection;

#[cfg(not(any(feature = "postgres", feature = "sqlite")))]
compile_error!("At least one feature of \"sqlite\" or \"postgres\" needs to be enabled");
