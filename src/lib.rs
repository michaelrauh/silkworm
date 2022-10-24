pub trait Registry {
    type Database;
    type Location;
    fn worker_name(&self) -> String;
    fn create_db(&self) -> Self::Database;
    fn db_location(&self, worker_name: String) -> Result<Self::Location, anyhow::Error>;
    fn write_db(&self, loc: Self::Location, db: Self::Database) -> Result<(), anyhow::Error>;
}


fn life(reg: impl Registry) -> Result<(), anyhow::Error> {
    let name = reg.worker_name();
    let db = reg.create_db();
    let loc = reg.db_location(name)?;
    reg.write_db(loc, db)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
