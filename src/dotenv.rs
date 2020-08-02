// Copyright (c) 2020 DarkWeb Design
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::{env, fs};
use std::collections::HashMap;

use regex::Regex;

use crate::Exception;

/// Dotenv file loader
pub struct Dotenv {
    path: String,
    data: String,
    line_number: usize,
    cursor: usize,
    end: usize,
    state: usize,
}

impl Dotenv {
    const STATE_VARNAME: usize = 0;
    const STATE_VALUE: usize = 1;

    ///
    /// Creates a new instance of the Dotenv file loader.
    ///
    /// # Examples
    ///
    /// ```dotenv
    /// # .env
    /// DB_USER=root
    /// DB_PASS=pass
    /// ```
    ///
    /// ```rust
    /// use darkweb_dotenv::Dotenv;
    ///
    /// let mut dotenv = Dotenv::new();
    /// dotenv.load(".env").unwrap();
    ///
    /// let db_user = std::env::var("DB_USER").unwrap();
    /// ```
    ///
    pub fn new() -> Self {
        Self {
            path: "".to_string(),
            data: "".to_string(),
            line_number: 0,
            cursor: 0,
            end: 0,
            state: Self::STATE_VARNAME,
        }
    }

    ///
    /// Loads environment variables from file a `.env` file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use darkweb_dotenv::Dotenv;
    ///
    /// let mut dotenv = Dotenv::new();
    /// dotenv.load(".env").unwrap();
    /// ```
    ///
    /// # Exceptions
    ///
    /// * `Exception::FormatException`
    /// * `Exception::PathException`
    ///
    pub fn load<Path>(&mut self, path: Path) -> Result<(), Exception>
        where
            Path: AsRef<str> {

        let path = path.as_ref().to_string();
        let data = self.read_file(&path)?;

        let values = self.parse(data, path)?;

        self.populate(&values, false);

        Ok(())
    }

    ///
    /// Loads environment variables from a `.env` file and overwrites exiting environment variables.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use darkweb_dotenv::Dotenv;
    ///
    /// let mut dotenv = Dotenv::new();
    /// dotenv.overload(".env").unwrap();
    /// ```
    ///
    /// # Exceptions
    ///
    /// * `Exception::FormatException`
    /// * `Exception::PathException`
    ///
    pub fn overload<Path>(&mut self, path: Path) -> Result<(), Exception>
        where
            Path: AsRef<str> {

        let path = path.as_ref().to_string();
        let data = self.read_file(&path)?;

        let values = self.parse(data, path)?;

        self.populate(&values, true);

        Ok(())
    }

    ///
    /// Loads environment-specific environment variables from multiple `.env` files in an hierarchy.
    ///
    /// The following files are loaded if they exist, the latter taking precedence over the former:
    /// * `.env` --> committed environment defaults
    /// * `.env.local` --> uncommitted file with local overrides
    /// * `.env.{APP_ENV}` --> committed environment-specific defaults
    /// * `.env.{APP_ENV}.local` --> uncommitted environment-specific local overrides
    ///
    /// # Examples
    ///
    /// ```rust
    /// use darkweb_dotenv::Dotenv;
    ///
    /// let mut dotenv = Dotenv::new();
    /// dotenv.load_env(".env", "APP_ENV", "dev").unwrap();
    /// ```
    ///
    /// # Exceptions
    ///
    /// * `Exception::FormatException`
    /// * `Exception::PathException`
    ///
    pub fn load_env<Path, EnvKey, DefaultEnv>(&mut self, path: Path, env_key: EnvKey, default_env: DefaultEnv) -> Result<(), Exception>
        where
            Path: AsRef<str>,
            EnvKey: AsRef<str>,
            DefaultEnv: AsRef<str> {

        let path = path.as_ref().to_string();
        let env_key = env_key.as_ref().to_string();
        let default_env = default_env.as_ref().to_string();

        let mut values = HashMap::new();

        if let Ok(data) = self.read_file(&path) {
            values.extend(self.parse(data, &path)?)
        }

        let local_path = format!("{}.local", path);

        if let Ok(data) = self.read_file(&local_path) {
            values.extend(self.parse(data, local_path)?)
        }

        self.populate(&values, false);
        values.clear();

        let env = match env::var_os(env_key) {
            Some(value) => value.to_string_lossy().to_string(),
            None => default_env,
        };

        if &env == "local" {
            return Ok(());
        }

        let env_path = format!("{}.{}", path, env);

        if let Ok(data) = self.read_file(&env_path) {
            values.extend(self.parse(data, env_path)?)
        }

        let env_local_path = format!("{}.{}.local", path, env);

        if let Ok(data) = self.read_file(&env_local_path) {
            values.extend(self.parse(data, env_local_path)?)
        }

        self.populate(&values, false);

        Ok(())
    }

    fn read_file<Path>(&mut self, path: Path) -> Result<String, Exception>
        where
            Path: AsRef<str> {

        let path = path.as_ref();

        match fs::read_to_string(path) {
            Ok(data) => Ok(data),
            Err(_) => Err(Exception::PathException(path.to_string())),
        }
    }

    fn parse<Data, Path>(&mut self, data: Data, path: Path) -> Result<HashMap<String, String>, Exception>
        where
            Data: AsRef<str>,
            Path: AsRef<str> {

        self.path = path.as_ref().to_string();
        self.data = data.as_ref().replace("\r\n", "\n");
        self.line_number = 1;
        self.cursor = 0;
        self.end = self.data.len();
        self.state = Self::STATE_VARNAME;

        let mut values = HashMap::new();

        let mut name = "".to_string();

        self.skip_empty_lines();

        while self.cursor < self.end {
            match self.state {
                Self::STATE_VARNAME => {
                    name = self.lex_varname()?;
                    self.state = Self::STATE_VALUE;
                },
                Self::STATE_VALUE => {
                    let value = self.lex_value()?;
                    values.insert(name.clone(), value);
                    self.state = Self::STATE_VARNAME;
                },
                _ => unreachable!("invalid state"),
            }
        }

        if self.state == Self::STATE_VALUE {
            values.insert(name.clone(), "".to_string());
        }

        Ok(values)
    }

    fn lex_varname(&mut self) -> Result<String, Exception> {
        let regex = Regex::new(r"^(export[ \t]++)?((?i:[A-Z][A-Z0-9_]*+))").unwrap();
        let regex_value = self.data.clone().chars().skip(self.cursor).collect::<String>();
        let regex_captures = regex.captures(&regex_value);

        if regex_captures.is_none() {
            return Err(self.create_format_exception("Invalid character in variable name".to_string()));
        }

        let captures = regex_captures.unwrap();

        self.move_cursor(&captures[0].to_string());

        let token = &self.get_token();

        if self.cursor == self.end || token == "\n" || token == "#" {
            if captures.get(1).is_some() {
                return Err(self.create_format_exception("Unable to unset an environment variable".to_string()));
            }

            return Err(self.create_format_exception("Missing = in the environment variable declaration".to_string()));
        }

        if token == " " || token == "\t" {
            return Err(self.create_format_exception("Whitespace characters are not supported after the variable name".to_string()));
        }

        if token != "=" {
            return Err(self.create_format_exception("Missing = in the environment variable declaration".to_string()));
        }

        self.cursor += 1;

        Ok(captures[2].to_string())
    }

    fn lex_value(&mut self) -> Result<String, Exception> {
        let regex = Regex::new(r"^[ \t]*+(?:#.*)?$").unwrap();
        let regex_value = self.data.clone().chars().skip(self.cursor).collect::<String>();
        let regex_match = regex.find(&regex_value);

        if regex_match.is_some() {
            self.move_cursor(regex_match.unwrap().as_str());
            self.skip_empty_lines();

            return Ok("".to_string());
        }

        if &self.get_token() == " " || &self.get_token() == "\t" {
            return Err(self.create_format_exception("Whitespace are not supported before the value".to_string()));
        }

        let mut value = "".to_string();

        loop {
            if &self.get_token() == "'" {
                let mut len = 0;

                loop {
                    len += 1;
                    if self.cursor + len == self.end {
                        self.cursor += len;

                        return Err(self.create_format_exception("Missing quote to end the value".to_string()));
                    }

                    if &self.get_token_at(self.cursor + len) == "'" {
                        break;
                    }
                }

                value = format!("{}{}", value, self.data.chars().skip(self.cursor + 1).take(len - 1).collect::<String>());
                self.cursor += 1 + len;
            } else if &self.get_token() == "\"" {
                let mut len = 0;

                loop {
                    len += 1;
                    if self.cursor + len == self.end {
                        self.cursor += len;

                        return Err(self.create_format_exception("Missing quote to end the value".to_string()));
                    }

                    if &self.get_token_at(self.cursor + len) == "\"" && &self.get_token_at(self.cursor + len - 1) != "\\" && &self.get_token_at(self.cursor + len - 2) != "\"" {
                        break;
                    }
                }

                let mut resolved_value = format!("{}{}", value, self.data.chars().skip(self.cursor + 1).take(len - 1).collect::<String>());
                resolved_value = resolved_value.replace("\\\"", "\"");
                resolved_value = resolved_value.replace("\\r", "\r");
                resolved_value = resolved_value.replace("\\n", "\n");
                resolved_value = resolved_value.replace("\\\\", "\\");

                value = format!("{}{}", value, resolved_value);
                self.cursor += 1 + len;
            } else {
                let mut resolved_value = "".to_string();
                let mut previous_character = self.get_token_at(self.cursor - 1);

                loop {
                    if self.cursor == self.end || self.get_token() == "\n" || self.get_token() == "\"" || self.get_token() == "'" || ((previous_character == " " || previous_character == "\t") && self.get_token() == "#") {
                        break;
                    }

                    if self.get_token() == "\\" && self.cursor + 1 < self.end && (self.get_token_at(self.cursor + 1) == "\"" || self.get_token_at(self.cursor + 1) == "'") {
                        self.cursor += 1;
                    }

                    previous_character = self.get_token();
                    resolved_value = format!("{}{}", resolved_value, previous_character);

                    self.cursor += 1;
                }

                resolved_value = resolved_value.trim_end().to_string();
                resolved_value = resolved_value.replace("\\\\", "\\");

                if resolved_value.contains(" ") || resolved_value.contains("\t") {
                    return Err(self.create_format_exception("A value containing spaces must be surrounded by quotes".to_string()));
                }

                value = format!("{}{}", value, resolved_value);

                if self.cursor < self.end && self.get_token() == "#" {
                    break;
                }
            }

            if self.cursor == self.end || &self.get_token() == "\n" {
                break;
            }
        }

        self.skip_empty_lines();

        Ok(value.to_string())
    }

    fn skip_empty_lines(&mut self) {
        let regex = Regex::new(r"^(?:\s*+(?:#[^\n]*+)?+)++").unwrap();
        let regex_value = self.data.clone().chars().skip(self.cursor).collect::<String>();

        if let Some(regex_match) = regex.find(&regex_value) {
            self.move_cursor(regex_match.as_str());
        }
    }

    fn move_cursor(&mut self, text: &str) {
        self.cursor += text.len();
        self.line_number += text.matches("\n").count();
    }

    fn get_token(&self) -> String {
        self.get_token_at(self.cursor)
    }

    fn get_token_at(&self, position: usize) -> String {
        self.data.chars().skip(position).take(1).collect::<String>()
    }

    fn create_format_exception(&self, message: String) -> Exception {
        Exception::FormatException(message, self.path.clone(), self.line_number)
    }

    fn populate(&self, values: &HashMap<String, String>, override_existing: bool) {
        for (key, value) in values.iter() {
            if override_existing && env::var_os(key).is_some() {
                continue;
            }
            env::set_var(key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Dotenv;

    #[test]
    fn parse_no_quotes() {
        let mut dotenv = Dotenv::new();
        let values = dotenv.parse("FOO=bar", ".env").unwrap();
        assert_eq!(values.get("FOO").unwrap(), "bar");
    }

    #[test]
    fn parse_single_quotes() {
        let mut dotenv = Dotenv::new();
        let values = dotenv.parse("FOO='bar'", ".env").unwrap();
        assert_eq!(values.get("FOO").unwrap(), "bar");
    }

    #[test]
    fn parse_single_quotes_concatenation() {
        let mut dotenv = Dotenv::new();
        let values = dotenv.parse("FOO='bar'\\''baz'", ".env").unwrap();
        assert_eq!(values.get("FOO").unwrap(), "bar'baz");
    }

    #[test]
    fn parse_double_quotes() {
        let mut dotenv = Dotenv::new();
        let values = dotenv.parse("FOO=\"bar\"", ".env").unwrap();
        assert_eq!(values.get("FOO").unwrap(), "bar");
    }

    #[test]
    fn parse_double_quotes_escaped_quotes() {
        let mut dotenv = Dotenv::new();
        let values = dotenv.parse("FOO=\"bar\\\"baz\"", ".env").unwrap();
        assert_eq!(values.get("FOO").unwrap(), "bar\"baz");
    }

    #[test]
    fn parse_double_quotes_newlines() {
        let mut dotenv = Dotenv::new();
        let values = dotenv.parse("FOO=\"bar\\r\\nbaz\"", ".env").unwrap();
        assert_eq!(values.get("FOO").unwrap(), "bar\r\nbaz");
    }

    #[test]
    fn parse_double_quotes_slashes() {
        let mut dotenv = Dotenv::new();
        let values = dotenv.parse("FOO=\"bar\\\\baz\"", ".env").unwrap();
        assert_eq!(values.get("FOO").unwrap(), "bar\\baz");
    }

    #[test]
    fn parse_export() {
        let mut dotenv = Dotenv::new();
        let values = dotenv.parse("export FOO=bar", ".env").unwrap();
        assert_eq!(values.get("FOO").unwrap(), "bar");
    }
}
