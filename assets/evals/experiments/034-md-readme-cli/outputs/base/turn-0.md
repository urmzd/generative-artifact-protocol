# dbmigrate

[![Build Status](https://img.shields.io/github/actions/workflow/status/example/dbmigrate/ci.yml?branch=main)](https://github.com/example/dbmigrate/actions)
[![Version](https://img.shields.io/npm/v/dbmigrate)](https://www.npmjs.com/package/dbmigrate)
[![License](https://img.shields.io/github/license/example/dbmigrate)](./LICENSE)

A fast, reliable CLI for managing database schema migrations and seed data across development, staging, and production environments.

## Overview

`dbmigrate` helps you create, apply, track, rollback, and inspect database migrations from the command line or as a library in your application. It is designed to be simple enough for small projects and flexible enough for production workflows.

## Installation

### npm

```bash
npm install -g dbmigrate
```

### Homebrew

```bash
brew install dbmigrate
```

### Cargo

```bash
cargo install dbmigrate
```

## Usage

Run `dbmigrate --help` to see all available commands and options.

### Common commands

```bash
dbmigrate init
```

Initialize a new migration workspace and create the default project structure.

```bash
dbmigrate create add-users-table
```

Create a new migration file with a timestamped name.

```bash
dbmigrate create add-indexes --seed
```

Create a migration file and a matching seed file.

```bash
dbmigrate up
```

Apply all pending migrations.

```bash
dbmigrate up --to 20240501123000
```

Apply migrations up to a specific version or timestamp.

```bash
dbmigrate down
```

Rollback the most recent migration.

```bash
dbmigrate down --steps 2
```

Rollback the last two applied migrations.

```bash
dbmigrate rollback --to 20240401090000
```

Rollback to a specific migration version.

```bash
dbmigrate status
```

Show the current migration state, applied versions, and pending files.

```bash
dbmigrate status --verbose
```

Show detailed migration metadata, including checksums and execution time.

```bash
dbmigrate seed
```

Apply all pending seed files.

```bash
dbmigrate seed --file seeds/initial_users.sql
```

Run a specific seed file.

```bash
dbmigrate init --force
```

Re-initialize the project structure, overwriting existing configuration files.

```bash
dbmigrate up --dry-run
```

Preview migrations without executing them.

## Configuration

`dbmigrate` supports configuration via a config file and environment variables.

### Config file

Default config file names:

- `dbmigrate.yml`
- `dbmigrate.yaml`
- `dbmigrate.json`

Example `dbmigrate.yml`:

```yaml
database:
  client: postgres
  host: localhost
  port: 5432
  name: app_db
  user: app_user
  password: secret
migrations:
  directory: ./migrations
  table: schema_migrations
  extension: .sql
seeds:
  directory: ./seeds
logging:
  level: info
```

Example `dbmigrate.json`:

```json
{
  "database": {
    "client": "postgres",
    "host": "localhost",
    "port": 5432,
    "name": "app_db",
    "user": "app_user",
    "password": "secret"
  },
  "migrations": {
    "directory": "./migrations",
    "table": "schema_migrations",
    "extension": ".sql"
  },
  "seeds": {
    "directory": "./seeds"
  },
  "logging": {
    "level": "info"
  }
}
```

### Environment variables

Environment variables override config file values.

- `DBMIGRATE_DATABASE_URL`
- `DBMIGRATE_DATABASE_CLIENT`
- `DBMIGRATE_DATABASE_HOST`
- `DBMIGRATE_DATABASE_PORT`
- `DBMIGRATE_DATABASE_NAME`
- `DBMIGRATE_DATABASE_USER`
- `DBMIGRATE_DATABASE_PASSWORD`
- `DBMIGRATE_MIGRATIONS_DIRECTORY`
- `DBMIGRATE_MIGRATIONS_TABLE`
- `DBMIGRATE_MIGRATIONS_EXTENSION`
- `DBMIGRATE_SEEDS_DIRECTORY`
- `DBMIGRATE_LOGGING_LEVEL`

Example:

```bash
export DBMIGRATE_DATABASE_URL=postgres://app_user:secret@localhost:5432/app_db
export DBMIGRATE_MIGRATIONS_DIRECTORY=./db/migrations
export DBMIGRATE_LOGGING_LEVEL=debug
```

## Library / API usage

You can also use `dbmigrate` programmatically in Node.js applications.

```js
const { Migrator } = require('dbmigrate');

async function main() {
  const migrator = new Migrator({
    database: {
      client: 'postgres',
      host: 'localhost',
      port: 5432,
      name: 'app_db',
      user: 'app_user',
      password: 'secret'
    },
    migrations: {
      directory: './migrations'
    }
  });

  await migrator.connect();
  await migrator.status();
  await migrator.up();
  await migrator.seed();
  await migrator.close();
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
```

### Common API methods

- `new Migrator(options)`
- `migrator.connect()`
- `migrator.status()`
- `migrator.up()`
- `migrator.down()`
- `migrator.rollback({ to, steps })`
- `migrator.seed()`
- `migrator.create(name, { seed })`
- `migrator.close()`

## Contributing

Contributions are welcome.

Summary:

- Fork the repository and create a feature branch
- Make your changes with tests when applicable
- Ensure linting and test suites pass
- Open a pull request with a clear description of the change
- Be respectful and follow the project code of conduct

## License

Released under the MIT License. See [LICENSE](./LICENSE) for details.