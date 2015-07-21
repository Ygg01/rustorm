use query::Query;
use table::Table;
use dao::{Dao,DaoResult, Value};
use writer::SqlFrag;
use query::{Connector, Equality, Operand, Field};
use query::{Direction, Modifier, JoinType};
use query::{Filter, Condition};
use query::SqlType;


/// SqlOption, contains the info about the features and quirks of underlying database
#[derive(PartialEq)]
pub enum SqlOption{
    /// use the numbered parameters, as the case with rust-postgres
    UsesNumberedParam,
    /// sqlite, jdbc
    UsesQuestionMark,
    /// postgresql supports returning clause on insert and update
    SupportsReturningClause,
    /// support CTE (common table expression ie. WITH) (postgresql, sqlite)
    SupportsCTE,
    /// supports inheritance (postgresql)
    SupportsInheritance,
    /// whether the database uses schema (postgresl, oracle)
    UsesSchema,
    /// wheter the returned rows in a query included Meta columns for easy extraction of records
    /// (postgres returns this), sqlite does not return meta columns, so you have to extract it by index yourself.
    ReturnMetaColumns,
}

/// Generic Database interface
/// This is the database interface which will should be implemented to you the specifics of each database platform
/// At least all methods on this trait should be implemented for target deployment database
/// A lower level API for manipulating objects in the database
/// 
/// TODO: acquire only a connection until a query is about to be executed.
/// generating query don't really need database connection just yet.

pub trait Database{

    /// return the version of the database
    /// lower version of database has fewer supported features
    fn version(&mut self)->String;
    
    /// begin database transaction
    fn begin(&mut self);

    /// commit database transaction
    fn commit(&mut self);

    /// rollback data changes executed prior to calling the begin method
    fn rollback(&mut self);

    /// determine if this transaction has been committed or rolledback
    fn is_transacted(&mut self)->bool;

    /// determine if the database connection closed
    fn is_closed(&mut self)->bool;

    /// check if the database is still connected
    fn is_connected(&mut self)->bool;

    /// close the database connection
    fn close(&mut self);

    /// determine if the database connection is still valid
    fn is_valid(&mut self)->bool;

    /// reset the database connection
    fn reset(&mut self);

    /// select
    /// returns an array to the qualified records
    fn select(&mut self, query:&Query)->DaoResult{
        self.execute_with_return(query)
    }

    /// insert
    /// insert an object, returns the inserted Dao value
    /// including the value generated via the defaults
    fn insert(&mut self, query:&Query)->Dao{
        let sql_frag = self.build_insert(query);
        self.execute_sql_with_one_return(&sql_frag.sql, &sql_frag.params)
    }

    /// update
    /// returns the updated Dao
    fn update(&mut self,query:&Query)->Dao;

    /// delete records
    /// returns the number of deleted records
    fn delete(&mut self, query:&Query)->Result<usize, String>;

    /// execute query with return dao,
    /// use the enumerated column for data extraction when db doesn't support returning the records column names
    fn execute_with_return(&mut self, query:&Query)->DaoResult{
        let sql_frag = self.build_query(query);
        let result = if self.sql_options().contains(&SqlOption::ReturnMetaColumns){
            self.execute_sql_with_return(&sql_frag.sql, &sql_frag.params)
        }else{
            let mut columns:Vec<&str> = vec![];
            for c in query.get_enumerated_columns(){
                columns.push(&c.column);//TODO deal with the renames, and functions, fields
            }
            self.execute_sql_with_return_columns(&sql_frag.sql, &sql_frag.params, columns)
        };
        
        DaoResult{
            dao: result,
            renamed_columns:query.get_renamed_columns(),
            total:None,
            page:None,
            page_size:None,
        }
    }

    /// execute query with 1 return dao
    fn execute_with_one_return(&mut self, query:&Query)->Dao{
        let sql_frag = self.build_query(query);
        self.execute_sql_with_one_return(&sql_frag.sql, &sql_frag.params)
    }
    
    /// execute query with no return dao
    fn execute(&mut self, query:&Query)->Result<usize, String>{
        let sql_frag = self.build_query(query);
        self.execute_sql(&sql_frag.sql, &sql_frag.params)
    }

    /// execute insert with returning clause, update with returning clause
    fn execute_sql_with_return(&mut self, sql:&str, params:&Vec<Value>)->Vec<Dao>;
    
    /// specify which return columns to get, ie. sqlite doesn't support getting the meta data of the return
    fn execute_sql_with_return_columns(&mut self, sql:&str, params:&Vec<Value>, return_columns:Vec<&str>)->Vec<Dao>;

    fn execute_sql_with_one_return(&mut self, sql:&str, params:&Vec<Value>)->Dao{
        let dao = self.execute_sql_with_return(sql, params);
        assert!(dao.len() == 1, "There should be 1 and only 1 record return here");
        dao[0].clone()
    }
    
    /// everything else, no required return other than error or affected number of records
    fn execute_sql(&mut self, sql:&str, param:&Vec<Value>)->Result<usize, String>;

    /// build a query, return the sql string and the parameters.
    /// use by select to build the select query
    /// build all types of query
    /// TODO: need to supply the number of parameters where to start the numbering of the number parameters
    fn build_query(&mut self, query:&Query)->SqlFrag{
        match query.sql_type{
            SqlType::SELECT => self.build_select(query),
            SqlType::INSERT => self.build_insert(query),
            SqlType::UPDATE => self.build_update(query),
            SqlType::DELETE => self.build_delete(query),
        }
    }
    
    /// build operand, i.e: columns, query, function, values
    fn build_operand(&mut self, w: &mut SqlFrag, parent_query:&Query, operand:&Operand){
        match operand{
            &Operand::ColumnName(ref column_name) => {
                if parent_query.joins.is_empty(){
                    w.append(&column_name.column);
                }else{
                    w.append(&column_name.complete_name());
                }
            }, 
            &Operand::TableName(ref table_name) => {
                if self.sql_options().contains(&SqlOption::UsesSchema){
                    w.append(&table_name.complete_name());
                }else{
                    w.append(&table_name.name);
                }
            },
            &Operand::Function(ref function)=>{
                    w.append("(");
                    let mut do_comma = false;
                    for param in &function.params{
                        if do_comma{ w.commasp(); }else{ do_comma = true;}
                        self.build_operand(w, parent_query, param);
                    }
                    w.append(")");
                },
            &Operand::Query(ref q) => {
                let sql_frag = self.build_query(q);
                w.append(&sql_frag.sql);
            },
            &Operand::Value(ref value) => {
                w.parameter(value.clone());
            },
            &Operand::Vec(ref operands) => {
                let mut do_comma = false;
                if !operands.is_empty(){
                    w.append("(");
                    for op in operands{
                        println!("op: {:?}",op);
                        if do_comma {w.commasp();}else{do_comma = true;}
                        self.build_operand(w, parent_query, op);
                    }
                    w.append(")");
                }
            },
        };
    }
    
    fn build_condition(&mut self, w: &mut SqlFrag, parent_query:&Query, cond:&Condition){
        self.build_operand(w, parent_query, &cond.left);
        w.append(" ");
        match cond.equality{
            Equality::EQ => w.append("= "),
            Equality::NE => w.append("!= "),
            Equality::LT => w.append("< "),
            Equality::LTE => w.append("<= "),
            Equality::GT => w.append("> "),
            Equality::GTE => w.append(">= "),
            Equality::IN => w.append("IN "),
            Equality::NOT_IN => w.append("NOT IN "),
            Equality::LIKE => w.append("LIKE "),
            Equality::NULL => w.append("IS NULL "),
            Equality::IS_NOT_NULL => w.append("IS NOT NULL "),
            Equality::IS_NULL => w.append("IS NULL "),
        };
        self.build_operand(w, parent_query, &cond.right);
    }
    
    fn build_field(&mut self, w: &mut SqlFrag, parent_query:&Query, field:&Field){
        self.build_operand(w, parent_query, &field.operand);
        match field.name{
            Some(ref name) => {
                w.append(" AS ");
                w.append(name);
            }
            None => (),
        };
    }
    
    
    fn build_filter(&mut self, w: &mut SqlFrag, parent_query:&Query, filter:&Filter){
        if !filter.subfilters.is_empty(){
            w.append("( ");
        }
        self.build_condition(w, parent_query, &filter.condition);
        for filt in &filter.subfilters{
            match filt.connector{
                Connector::And =>{
                    w.append("AND ");
                }
                Connector::Or => {
                    w.append("OR ");
                }
            }
            self.build_filter(w, parent_query, filt);// build sub filters as well
        }
        if !filter.subfilters.is_empty(){
            w.append(" )");
        }
    }
    
    /// build the filter clause or the where clause of the query
    /// TODO: add the sub filters
    fn build_filters(&mut self, w: &mut SqlFrag, parent_query:&Query, filters: &Vec<Filter>){
        let mut do_and = false;
        for filter in filters{
            if do_and{
                w.ln_tabs(2);
                w.append("AND ");
            }else{
                do_and = true;
            }
            self.build_filter(w, parent_query, filter);
        }
    }

    /// build the enumerated, distinct, *, columns
    fn build_enumerated_fields(&mut self, w: &mut SqlFrag, parent_query:&Query, enumerated_fields: &Vec<Field>){
        let mut do_comma = false;
        let mut cnt = 0;
        for field in enumerated_fields{
            if do_comma{w.commasp();}else{do_comma=true;}
            cnt += 1;
            if cnt % 4 == 0{//break at every 4 columns to encourage sql tuning/revising
                w.ln_tab();
            }
            self.build_field(w, parent_query, field);
        }
    }

    /// build the select statment from the query object
    fn build_select(&mut self, query: &Query)->SqlFrag{
        let mut w = SqlFrag::new(self.sql_options());
        w.append("SELECT ");
        self.build_enumerated_fields(&mut w, query, &query.enumerated_fields); //TODO: add support for column_sql, fields, functions
        w.ln();
        w.append(" FROM ");
        
        assert!(query.from.is_some(), "There should be table, query, function to select from");
        
        match query.from{
            Some(ref field) => {
                self.build_field(&mut w, query, field);
            }
            None => println!("Warning: No from in this query"),
        };
        if !query.joins.is_empty(){
            for join in &query.joins{
                w.ln_tab();
                match join.modifier{
                    Some(ref modifier) => {
                            match modifier{
                                &Modifier::LEFT => w.append("LEFT "),
                                &Modifier::RIGHT => w.append("RIGHT "),
                                &Modifier::FULL => w.append("FULL "),
                            };
                        },
                    None => ()
                };
                
                match join.join_type{
                    JoinType::CROSS => w.append("CROSS "),
                    JoinType::INNER => w.append("INNER "),
                    JoinType::OUTER => w.append("OUTER "),
                };
                w.append("JOIN ");
                w.append(&join.table_name.complete_name());
                w.append(" ");
                assert!(join.column1.len() == join.column2.len(), "There should be equal number of corresponding columns to join");
                let mut cnt = 0;
                let mut do_and = false;
                for jc in &join.column1{
                    w.ln_tabs(2);
                    if do_and {
                        w.append("AND ");
                    }else{
                        w.append("ON ");
                        do_and = true;
                    }
                    w.append(jc);
                    w.append(" = ");
                    w.append(&join.column2[cnt]);
                    w.append(" ");
                    cnt += 1;
                }
            }
        }
        
        if !query.filters.is_empty() {
            w.ln_tab();
            w.append("WHERE ");
            self.build_filters(&mut w, query, &query.filters);
        }
        
        if !query.group_by.is_empty() {
            w.ln_tab();
            w.append("GROUP BY ");
            let mut do_comma = false;
            for operand in &query.group_by{
                if do_comma{ w.comma(); }else{ do_comma = true;}
                self.build_operand(&mut w, query, operand);
                w.append(" ");
            }
        };
        
        if !query.having.is_empty() {
            w.ln_tab();
            w.append("HAVING ");
            let mut do_comma = false;
            for hav in &query.having{
                if do_comma { w.commasp(); }else{ do_comma=true; }
                self.build_condition(&mut w, query, hav);
            }
        }
        
        if !query.order_by.is_empty(){
            w.ln_tab();
            w.append("ORDER BY ");
            let mut do_comma = false;
            for &(ref column, ref direction) in &query.order_by{
                if do_comma { w.commasp();} else { do_comma = true;}
                w.append(&column);
                match direction{
                    &Direction::ASC => w.append(" ASC"),
                    &Direction::DESC => w.append(" DESC")
                };
            }
        };
        
        match query.page_size{
            Some(page_size) => {
                w.ln_tab();
                w.append("LIMIT ");
                w.append(&format!("{}",page_size));
            },
            None => (),
        };
        
        match query.page{
            Some(page) =>{
                w.ln_tab();
                w.append("OFFSET ");
                assert!(query.page_size.is_some(), "Page size should be specified when paging");
                let page_size = query.page_size.unwrap();
                let offset = page * page_size;
                w.append(&format!("{}",offset));
            },
            None => (),
        };
        w
    }
    
    /// TODO complete this
    fn build_insert(&mut self, query: &Query)->SqlFrag{
        let mut w = SqlFrag::new(self.sql_options());
        w.append("INSERT INTO ");
        let into_table = query.get_from_table();
        assert!(into_table.is_some(), "There should be table to insert to");
        if into_table.is_some(){
            let table_name = into_table.unwrap();
            if self.sql_options().contains(&SqlOption::UsesSchema){
                w.append(&table_name.complete_name());
            }else{
                w.append(&table_name.name);
            }
        }
        
        
        w.append("(");
        self.build_enumerated_fields(&mut w, query, &query.enumerated_fields); //TODO: add support for column_sql, fields, functions
        w.append(") ");
        assert!(!query.values.is_empty(), "values should not be empty, when inserting records");
        if !query.values.is_empty(){
            w.append("VALUES( ");
            let mut do_comma = false;
            for vo in &query.values{
                if do_comma{ w.commasp(); } else{do_comma=true;}
                self.build_operand(&mut w, query, vo);
            }
            w.append(") ");
        }
        if !query.enumerated_returns.is_empty() {
            if self.sql_options().contains(&SqlOption::SupportsReturningClause) {
                w.append("RETURNING ");
                let mut do_comma = false;
                for field in &query.enumerated_returns{
                    if do_comma{ w.commasp(); }else {do_comma = true;}
                    self.build_field(&mut w, query, field);
                }
            }
        }
        w.ln();
        w
    }

    
    fn build_update(&mut self, query: &Query)->SqlFrag{
        let mut w = SqlFrag::new(self.sql_options());
        w.append("UPDATE ");
        let from_table = query.get_from_table();
        assert!(from_table.is_some(), "There should be table to update from");
        if from_table.is_some(){
            w.append(&from_table.unwrap().complete_name());
        }
        w.ln();
        let enumerated_columns = query.get_enumerated_columns();
        let mut do_comma = false;
        if !enumerated_columns.is_empty(){
            w.append("SET ");
        }
        let mut column_index = 0;
        for ec in &enumerated_columns{
            if do_comma{ w.commasp(); } else{do_comma = true;}
            w.append(&ec.column);
            w.append(" = ");
            let value = &query.values[column_index];
            match value{
                &Operand::Value(ref value) => {
                    w.parameter(value.clone());
                },
                _ => {}
            }
            column_index += 1;
        }
       
        if !query.filters.is_empty() {
            w.ln_tab();
            w.append("WHERE ");
            self.build_filters(&mut w, query, &query.filters);
        }
        if !query.enumerated_returns.is_empty() {
            if self.sql_options().contains(&SqlOption::SupportsReturningClause) {
                w.append("RETURNING ");
                let mut do_comma = false;
                for field in &query.enumerated_returns{
                    if do_comma{ w.commasp(); }else {do_comma = true;}
                    self.build_field(&mut w, query, field);
                }
            }
        }
        w
    }

    fn build_delete(&mut self, query: &Query)->SqlFrag{
        let mut w = SqlFrag::new(self.sql_options());
        w.append("DELETE FROM ");
        let from_table = query.get_from_table();
        assert!(from_table.is_some(), "There should be table to delete from");
        if from_table.is_some(){
            w.append(&from_table.unwrap().complete_name());
        }
        if !query.filters.is_empty() {
            w.ln_tab();
            w.append("WHERE ");
            self.build_filters(&mut w, query, &query.filters);
        }
        w
    }

    fn sql_options(&mut self)->Vec<SqlOption>;

}


/// Deployment Database should implement this trait,
/// to enable automated installation of the app, regardless what database platform
/// the app is developed from.
pub trait DatabaseDDL{
    //////////////////////////////////////////
    /// The following methods involves DDL(Data definition language) operation
    ////////////////////////////////////////

    /// create a database schema
    fn create_schema(&mut self, schema:&str);

    /// drop the database schema
    fn drop_schema(&mut self, schema:&str);

    /// create a database table based on the Model Definition
    fn create_table(&mut self, model:&Table);
    
    /// build sql for create table
    fn build_create_table(&mut self, table:&Table)->SqlFrag;

    /// rename table, in the same schema
    fn rename_table(&mut self, table:&Table, new_tablename:String);

    /// drop table
    fn drop_table(&mut self, table:&Table);

    /// set the foreign key constraint of a table
    fn set_foreign_constraint(&mut self, model:&Table);

    /// set the primary key constraint of a table
    fn set_primary_constraint(&mut self, model:&Table);
}


/// implement this for database that you use as your development platform, to extract meta data information
/// about the tables and their relationship to each other
pub trait DatabaseDev{

////////////////////////////////////////
/// Database interface use for the development process
////////////////////////////////////////////

    /// applicable to later version of postgresql where there is inheritance
    fn get_table_sub_class(&mut self, schema:&str, table:&str)->Vec<String>;

    fn get_parent_table(&mut self, schema:&str, table:&str)->Option<String>;

    ////
    /// Build the Table object based on the extracted meta data info from database
    /// This is queries directly from the database, so this will be costly. Only used this on initialization processes
    ///
    fn get_table_metadata(&mut self, schema:&str, table:&str)->Table;

    /// get all the tables in this database
    fn get_all_tables(&mut self)->Vec<(String, String)>;

    /// get the comment of this table
    fn get_table_comment(&mut self, schema:&str, table:&str)->Option<String>;

    /// get the inherited columns of this table
    fn get_inherited_columns(&mut self, schema:&str, table:&str)->Vec<String>;

    ///get the equivalent postgresql database data type to rust data type
    /// returns (module, type)
    fn dbtype_to_rust_type(&self, db_type: &str)->(Vec<String>, String);
    
    fn rust_type_to_dbtype(&self, rust_type: &str)->String;

}
