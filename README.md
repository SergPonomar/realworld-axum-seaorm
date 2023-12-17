# ![RealWorld Example App](logo.png)

> ### Axum - Sea Orm codebase containing real world examples (CRUD, auth, advanced patterns, etc) that adheres to the [RealWorld](https://github.com/gothinkster/realworld) spec and API.


### [Demo](https://demo.realworld.io/)&nbsp;&nbsp;&nbsp;&nbsp;[RealWorld](https://github.com/gothinkster/realworld)


This codebase was created to demonstrate a fully fledged backend application built with **Axum - Sea Orm** including CRUD operations, authentication, routing, pagination, and more.

We've gone to great lengths to adhere to the **Axum - Sea Orm** community styleguides & best practices.

For more information on how to this works with other frontends/backends, head over to the [RealWorld](https://github.com/gothinkster/realworld) repo.


# How it works

> App use [Axum](https://github.com/tokio-rs/axum) framework as backend server and [Sea Orm](https://www.sea-ql.org/SeaORM/) as database querying tool. Can be used with any of the following:

- [MySql](https://www.mysql.com/)
- [PostgreSQL](https://www.postgresql.org/)
- [SQLite](https://www.sqlite.org/index.html)

# Preparation

Please add environment variables for database connection in `.env` file. Sample `.env.example` can be finded in root folder. `sqlite::memory:` can be used for fast start.

# Getting started

> To run app

```sh
cargo run
```

> Optional flag may use for seeding database with sample data

```sh
cargo run --features seed
```

# Testing

> To run tests

```sh
cargo test
```

Tests uses [sqlite](https://www.sqlite.org/inmemorydb.html) in-memory database for fast test with real database.