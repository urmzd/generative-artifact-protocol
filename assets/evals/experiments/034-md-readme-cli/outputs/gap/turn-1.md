<gap:target id="readme-document">
# <gap:target id="project-name">dbmigrate</gap:target>

<gap:target id="overview-section">
![<gap:target id="build-badge-alt">Build</gap:target>](<gap:target id="build-badge-url">https://img.shields.io/github/actions/workflow/status/example/dbmigrate/ci.yml?branch=main</gap:target>)
![<gap:target id="version-badge-alt">Version</gap:target>](<gap:target id="version-badge-url">https://img.shields.io/npm/v/dbmigrate.svg</gap:target>)
![<gap:target id="license-badge-alt">License</gap:target>](<gap:target id="license-badge-url">https://img.shields.io/npm/l/dbmigrate.svg</gap:target>)

<gap:target id="brief-description">dbmigrate is a fast, opinionated database migration CLI for managing schema changes, running migrations safely, and keeping environments in sync.</gap:target>
</gap:target>

<gap:target id="installation-section">
## Installation

<gap:target id="installation-npm-section">
### npm
<gap:target id="installation-npm-command">npm install -g dbmigrate</gap:target>
</gap:target>

<gap:target id="installation-brew-section">
### Homebrew
<gap:target id="installation-brew-command">brew install dbmigrate</gap:target>
</gap:target>

<gap:target id="installation-cargo-section">
### Cargo
<gap:target id="installation-cargo-command">cargo install dbmigrate</gap:target>
</gap:target>
</gap:target>

<gap:target id="usage-section">
## Usage

<gap:target id="usage-intro">Run `dbmigrate --help` to see all available commands and options.</gap:target>

<gap:target id="usage-examples-list">
- <gap:target id="usage-example-init">dbmigrate init</gap:target>
- <gap:target id="usage-example-create">dbmigrate create add_users_table</gap:target>
- <gap:target id="usage-example-create-timestamped">dbmigrate create add_email_index --timestamp</gap:target>
- <gap:target id="usage-example-up">dbmigrate up</gap:target>
- <gap:target id="usage-example-up-specific">dbmigrate up --to 20240501120000</gap:target>
- <gap:target id="usage-example-down">dbmigrate down</gap:target>
- <gap:target id="usage-example-down-count">dbmigrate down --steps 2</gap:target>
- <gap:target id="usage-example-status">dbmigrate status</gap:target>
- <gap:target id="usage-example-rollback">dbmigrate rollback</gap:target>
- <gap:target id="usage-example-rollback-to">dbmigrate rollback --to 20240401093000</gap:target>
- <gap:target id="usage-example-seed">dbmigrate seed</gap:target>
- <gap:target id="usage-example-seed-specific">dbmigrate seed --file seeds/demo.sql</gap:target>
- <gap:target id="usage-example-config">dbmigrate up --config ./dbmigrate.config.json</gap:target>
</gap:target>

<gap:target id="usage-note">Use `dbmigrate status` to verify pending, applied, and failed migrations before deploying changes.</gap:target>
</gap:target>

<gap:target id="configuration-section">
## Configuration

<gap:target id="configuration-file-section">
### Config file format

<gap:target id="configuration-file-description">`dbmigrate` supports JSON, YAML, and TOML configuration files.</gap:target>

<gap:target id="configuration-json-example">
```json
{
  "dialect": "postgres",
  "url": "postgres://user:pass@localhost:5432/app_db",
  "migrationsDir": "./migrations",
  "seedsDir": "./seeds",
  "tableName": "schema_migrations",
  "ssl": false
}
```
</gap:target>

<gap:target id="configuration-yaml-example">
```yaml
dialect: postgres
url: postgres://user:pass@localhost:5432/app_db
migrationsDir: ./migrations
seedsDir: ./seeds
tableName: schema_migrations
ssl: false
```
</gap:target>
</gap:target>

<gap:target id="configuration-env-section">
### Environment variables

<gap:target id="configuration-env-list">
- <gap:target id="env-dbmigrate-url">DBMIGRATE_URL</gap:target>: <gap:target id="env-dbmigrate-url-description">database connection string</gap:target>
- <gap:target id="env-dbmigrate-dialect">DBMIGRATE_DIALECT</gap:target>: <gap:target id="env-dbmigrate-dialect-description">database dialect such as `postgres`, `mysql`, or `sqlite`</gap:target>
- <gap:target id="env-dbmigrate-migrations-dir">DBMIGRATE_MIGRATIONS_DIR</gap:target>: <gap:target id="env-dbmigrate-migrations-dir-description">path to migration files</gap:target>
- <gap:target id="env-dbmigrate-seeds-dir">DBMIGRATE_SEEDS_DIR</gap:target>: <gap:target id="env-dbmigrate-seeds-dir-description">path to seed files</gap:target>
- <gap:target id="env-dbmigrate-table-name">DBMIGRATE_TABLE_NAME</gap:target>: <gap:target id="env-dbmigrate-table-name-description">migration tracking table name</gap:target>
- <gap:target id="env-dbmigrate-log-level">DBMIGRATE_LOG_LEVEL</gap:target>: <gap:target id="env-dbmigrate-log-level-description">log verbosity such as `info`, `debug`, or `error`</gap:target>
</gap:target>
</gap:target>
</gap:target>

<gap:target id="api-section">
## API / Library Usage

<gap:target id="api-intro">You can also use `dbmigrate` as a library from Node.js applications to run migrations programmatically.</gap:target>

<gap:target id="api-example">
```js
import { migrate, rollback, status, createMigration } from 'dbmigrate';

const result = await migrate({
  url: process.env.DBMIGRATE_URL,
  migrationsDir: './migrations'
});

console.log(result.applied);

await status({
  url: process.env.DBMIGRATE_URL
});

await rollback({
  url: process.env.DBMIGRATE_URL,
  steps: 1
});

await createMigration({
  name: 'add_audit_columns',
  migrationsDir: './migrations'
});
```
</gap:target>

<gap:target id="api-methods-list">
- <gap:target id="api-method-migrate">`migrate(options)`</gap:target>: <gap:target id="api-method-migrate-description">applies pending migrations</gap:target>
- <gap:target id="api-method-rollback">`rollback(options)`</gap:target>: <gap:target id="api-method-rollback-description">reverts the last applied migration or a specified number of steps</gap:target>
- <gap:target id="api-method-status">`status(options)`</gap:target>: <gap:target id="api-method-status-description">returns applied, pending, and failed migration information</gap:target>
- <gap:target id="api-method-create-migration">`createMigration(options)`</gap:target>: <gap:target id="api-method-create-migration-description">creates a new migration file</gap:target>
</gap:target>
## Troubleshooting

- **`dbmigrate` cannot connect to the database**: Verify `DBMIGRATE_URL` or your config file, confirm the database is running, and check network/firewall settings.
- **Migrations fail with a syntax error**: Inspect the generated migration file for invalid SQL or schema changes that are not supported by your database dialect.
- **A migration was applied but not recorded correctly**: Check the migrations tracking table name in your config and ensure the application user has permission to read/write it.
- **Seed files are not being found**: Confirm `DBMIGRATE_SEEDS_DIR` or `seedsDir` points to the correct folder and that the file path used in the command is valid.
- **Rollback stops partway through**: Make sure each migration has a valid down/rollback path, then retry with a smaller number of steps or a specific target version.
</gap:target>

<gap:target id="contributing-section">
## Contributing

<gap:target id="contributing-summary">Contributions are welcome. Please open an issue for discussion before making larger changes, follow the existing code style, add tests for new behavior, and ensure the test suite passes before submitting a pull request.</gap:target>

<gap:target id="contributing-checklist">
- <gap:target id="contributing-fork">Fork the repository</gap:target>
- <gap:target id="contributing-branch">Create a feature branch</gap:target>
- <gap:target id="contributing-tests">Add or update tests</gap:target>
- <gap:target id="contributing-lint">Run linting and formatting</gap:target>
- <gap:target id="contributing-pr">Open a pull request</gap:target>
</gap:target>
</gap:target>

<gap:target id="license-section">
## License

<gap:target id="license-name">MIT</gap:target>
</gap:target>
</gap:target>