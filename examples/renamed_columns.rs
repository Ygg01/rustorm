extern crate rustorm;
extern crate uuid;
extern crate chrono;
extern crate rustc_serialize;

use uuid::Uuid;
use chrono::datetime::DateTime;
use chrono::offset::utc::UTC;
use rustc_serialize::json;

use rustorm::query::Query;
use rustorm::query::{Filter, Equality};
use rustorm::dao::{Dao, IsDao};
use rustorm::pool::ManagedPool;

#[derive(Debug, Clone)]
pub struct Photo {
    pub photo_id: Uuid,
    pub url: Option<String>,
}

impl IsDao for Photo{
    fn from_dao(dao: &Dao) -> Self {
        Photo {
            photo_id: dao.get("photo_id"),
            url: dao.get_opt("url"),
        }
    }
    fn to_dao(&self) -> Dao {
        let mut dao = Dao::new();
        dao.set("photo_id", &self.photo_id);
        match self.url {
            Some(ref _value) => dao.set("url", _value),
            None => dao.set_null("url"),
        }
        dao
    }
}

fn main() {
    let url = "postgres://postgres:p0stgr3s@localhost/bazaar_v6";
    let mut pool = ManagedPool::init(&url, 1).unwrap();
    let db = pool.connect().unwrap();

    let mut query = Query::select();
    query.columns(vec!["product.product_id",
                       "product.name",
                       "category.product_id",
                       "category.name",
                       "photo.url"]);
    query.from_table("bazaar.product")
         .left_join_table("bazaar.product_category",
                          "product_category.product_id",
                          "product.product_id")
         .left_join_table("bazaar.category",
                          "category.category_id",
                          "product_category.category_id")
         .left_join_table("product_photo",
                          "product.product_id",
                          "product_photo.product_id")
         .left_join_table("bazaar.photo", "product_photo.photo_id", "photo.photo_id")
         .filter("product.name", Equality::EQ, &"GTX660 Ti videocard")
         .filter("category.name", Equality::EQ, &"Electronic")
         .group_by(vec!["category.name"])
         .having("count(*)", Equality::GT, &1)
         .asc("product.name")
         .desc("product.created");
    let frag = query.build(db.as_ref());

    let expected = "
SELECT product.product_id AS product_product_id, product.name AS product_name, category.product_id AS category_product_id, 
    category.name AS category_name, photo.url
 FROM bazaar.product
    LEFT JOIN bazaar.product_category 
        ON product_category.product_id = product.product_id 
    LEFT JOIN bazaar.category 
        ON category.category_id = product_category.category_id 
    LEFT JOIN product_photo 
        ON product.product_id = product_photo.product_id 
    LEFT JOIN bazaar.photo 
        ON product_photo.photo_id = photo.photo_id 
    WHERE product.name = $1 
        AND category.name = $2 
    GROUP BY category.name 
    HAVING count(*) > $3 
    ORDER BY product.name ASC, product.created DESC".to_string();
    println!("actual:   {{{}}} [{}]", frag.sql, frag.sql.len());
    println!("expected: {{{}}} [{}]", expected, expected.len());
    assert!(frag.sql.trim() == expected.trim());

}
