//! Rustorm is a simple ORM implemented in rust.
//!
//! ## Example
//! [..](https://github.com/ivanceras/rustorm/blob/master/examples/show_one_product.rs)
//!
//! ```rust
//! extern crate rustorm;
//! extern crate uuid;
//! extern crate chrono;
//! extern crate rustc_serialize;
//!
//! use uuid::Uuid;
//! use chrono::datetime::DateTime;
//! use chrono::offset::utc::UTC;
//! use rustc_serialize::json;
//! use rustorm::query::Query;
//! use rustorm::query::{Filter,Equality};
//! use rustorm::dao::{Dao,IsDao};
//! use rustorm::pool::ManagedPool;
//! use rustorm::table::{IsTable,Table};
//! #[derive(Debug, Clone)]
//! pub struct Product {
//!     pub product_id:Uuid,
//!     pub name:Option<String>,
//!     pub description:Option<String>,
//! }
//! impl IsDao for Product{
//!     fn from_dao(dao:&Dao)->Self{
//!         Product{
//!             product_id: dao.get("product_id"),
//!             name: dao.get_opt("name"),
//!             description: dao.get_opt("description"),
//!         }
//!     }
//!    fn to_dao(&self)->Dao{
//!        let mut dao = Dao::new();
//!        dao.set("product_id", &self.product_id);
//!        match self.name{
//!            Some(ref _value) => dao.set("name", _value),
//!            None => dao.set_null("name"),
//!        };
//!        match self.description{
//!            Some(ref _value) => dao.set("description", _value),
//!            None => dao.set_null("description"),
//!        };
//!        dao
//!    }
//! }
//! impl IsTable for Product{
//!     fn table()->Table{
//!         Table{
//!             schema:"bazaar".to_string(),
//!             name:"product".to_string(),
//!             parent_table:None,
//!             sub_table:vec![],
//!             comment:None,
//!             columns:vec![],
//!             is_view: false
//!         }
//!     }
//! }
//! fn main(){
//!     let url = "postgres://postgres:p0stgr3s@localhost/bazaar_v6";
//!     let mut pool = ManagedPool::init(&url, 1).unwrap();
//!     let db = pool.connect().unwrap();
//!     let prod: Product = Query::select_all()
//!             .from_table("bazaar.product")
//!             .filter("name", Equality::EQ, &"GTX660 Ti videocard")
//!             .collect_one(db.as_ref()).unwrap();
//!     println!("{}  {}  {:?}", prod.product_id, prod.name.unwrap(), prod.description);
//! }
//!
//!
//! ```
//!
//!
//!

extern crate postgres;
#[cfg(feature = "sqlite")]
extern crate rusqlite;
extern crate mysql;
extern crate rustc_serialize;
extern crate uuid;
extern crate chrono;
extern crate regex;
extern crate url;
extern crate r2d2;
extern crate r2d2_postgres;
#[cfg(feature = "sqlite")]
extern crate r2d2_sqlite;


pub mod em;
pub mod query;
pub mod dao;
pub mod database;
pub mod platform;
pub mod table;
pub mod writer;
pub mod config;
pub mod pool;
