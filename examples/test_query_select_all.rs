extern crate rustorm;
extern crate uuid;
extern crate chrono;
extern crate rustc_serialize;


use rustorm::db::postgres::Postgres;
use rustorm::codegen;
use uuid::Uuid;
use chrono::datetime::DateTime;
use chrono::offset::utc::UTC;
use rustc_serialize::json;

use rustorm::em::EntityManager;
use rustorm::table::IsTable;
use rustorm::dao::IsDao;
use rustorm::query::Query;
use rustorm::dao::Type;
use rustorm::query::{Filter,Equality,Operand};
use gen::bazaar::Product;
use gen::bazaar::ProductAvailability;
use gen::bazaar::product;
use gen::bazaar::product_availability;

mod gen;
 

fn main(){
    let pg:Result<Postgres,&str> = Postgres::new("postgres://postgres:p0stgr3s@localhost/bazaar_v6");
       match pg{
        Ok(pg) => {
            let mut query = Query::select();
            query.from::<Product>();
            query.select_all();
            query.filter(product::name, Equality::LIKE, &"iphone");
            query.add_filter(
                Filter::new(product::description, Equality::LIKE, 
                    Operand::Value(Type::String("%Iphone%".to_string())))
                );
            //let products: Vec<Product> = query.collect(&pg);
            let products = query.collect::<Product>(&pg);
            for prod in products{
                println!("\n\nprod: {:?}", prod)
            }
        }
        Err(error) =>{
            println!("{}",error);
        }
    }
}
