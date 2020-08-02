# Dotenv component

The Dotenv component parses `.env` files to make environment variables stored in them accessible via `std::env`.

## Installation

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
darkweb-dotenv = "^1.0"
```

## Usage

Sensitive information and environment-dependent settings should be defined as environment variables (as recommended for
[twelve-factor applications](http://www.12factor.net/)). Using a `.env` file to store those environment variables eases
development and CI management by keeping them in one "standard" place and agnostic of the technology stack you are
using.

```dotenv
# .env
DB_USER=root
DB_PASS=pass
```

Load a `.env` file in your application via `Dotenv::load()`:

```rust
use darkweb_dotenv::Dotenv;

let mut dotenv = Dotenv::new();
dotenv.load(".env").unwrap();
```

Access the values with `std::env` in your code:

```rust
let db_user = std::env::var("DB_USER").unwrap();
```

The `load()` method never overwrites existing environment variables. Use the `overload()` method if you need to
overwrite them:

```rust
// ...
dotenv.overload(".env").unwrap();
```

As you're working with the Dotenv component you'll notice that you might want to have different files depending on the
environment you're working in. Typically this happens for local development or Continuous Integration where you might
want to have different files for your `test` and `dev` environments.

You can use `Dotenv::load_env()` to ease this process:

```rust
// ...
dotenv.load_env(".env", "APP_ENV", "dev").unwrap();
```

The Dotenv component will then look for the correct `.env` files to load. If the environment variable `APP_ENV` is
defined, its value is used to load environment-specific files. If the variable is not defined, `dev` is assumed for
`APP_ENV`.

The following files are loaded if they exist, the latter taking precedence over the former:

* `.env` --> committed environment defaults
* `.env.local` --> uncommitted file with local overrides
* `.env.{APP_ENV}` --> committed environment-specific defaults
* `.env.{APP_ENV}.local` --> uncommitted environment-specific local overrides

## Links

* Documentation: https://docs.rs/darkweb-dotenv
* Repository: https://github.com/darkwebdesign/rust-dotenv
* Issue Tracker: https://github.com/darkwebdesign/rust-dotenv/issues

## License

Dotenv is licensed under the MIT License - see the `LICENSE` file for details.
