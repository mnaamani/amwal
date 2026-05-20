# core banking platform

*Experimental* rust based banking/finance platform, with a ledger service at the core that does (double-entry) book-keeping.

## Tech
- [Rust](https://rust-lang.org) - Memory safety, Performance, Fearless concurrency, Small binary sizes
- [PostgresDB](https://www.postgresql.org/) - Reliability + Performance
- [Diesel](https://diesel.rs/) - SQL Query builder Type safety, composability, 
- [actix-web](https://github.com/actix/actix-web) - Powerful, Pragmatic, Extremely fast web framework

!! Intended for learning and educational purpouses only !!


## Setup
```sh
docker compose -f docker/docker-compose.yaml up -d

# Wait for db to be ready
timeout 5

pushd crates/ledger
diesel setup # incase database was not created
diesel migration run
popd

pushd crates/transfers
diesel setup # incase database was not created
diesel migration run
popd
```