use std::{collections::HashMap, path::PathBuf};

use lsp_types::InitializeParams;
use serde_json::Value;

const SNIPPETS_STR: &'static str = include_str!("../snippets.toml");

lazy_static! {
    #[derive(Debug)]
    static ref SNIPPETS: LangSnippets = toml::from_str(&SNIPPETS_STR).unwrap();
}

type Snippets = HashMap<String, String>;
type LangSnippets = HashMap<String, Snippets>;

pub fn parse(params: InitializeParams) -> LangSnippets {
    let common_snippets = SNIPPETS.clone();

    match params.initialization_options {
        Some(initialization_options) => {
            let file_snippets = file_snippets(&initialization_options);
            let config_snippets = config_snippets(&initialization_options);

            let common_snippets = merge_snippets(common_snippets, file_snippets);

            merge_snippets(common_snippets, config_snippets)
        }
        None => common_snippets,
    }
}

fn file_snippets(options: &Value) -> LangSnippets {
    match options.get("snippets_file") {
        Some(path_value) => match serde_json::from_value::<String>(path_value.to_owned()) {
            Ok(path) => {
                let path = if path.starts_with("~/") {
                    match directories::UserDirs::new() {
                        Some(user_dirs) => user_dirs
                            .home_dir()
                            .join(&path[2..])
                            .into_os_string()
                            .into_string()
                            .unwrap_or(path),
                        None => path,
                    }
                } else {
                    path
                };

                let path = PathBuf::from(path);

                parse_file(path)
            }
            Err(_) => {
                eprintln!("snippets_file key is not a string");

                LangSnippets::new()
            }
        },
        None => match directories::UserDirs::new() {
            Some(user_dirs) => {
                let path = user_dirs
                    .home_dir()
                    .join(".config")
                    .join("snippets-ls")
                    .join("snippets.toml");

                parse_file(path)
            }
            None => LangSnippets::new(),
        },
    }
}
fn config_snippets(options: &Value) -> LangSnippets {
    match options.get("snippets") {
        Some(config_snippets_value) => {
            match serde_json::from_value(config_snippets_value.to_owned()) {
                Ok(config_snippets) => config_snippets,
                Err(_) => {
                    eprintln!("snippets are invalid");

                    LangSnippets::new()
                }
            }
        }
        None => LangSnippets::new(),
    }
}

fn parse_file(path: PathBuf) -> LangSnippets {
    match std::fs::read_to_string(path) {
        Ok(content) => match toml::from_str::<LangSnippets>(&content) {
            Ok(lang_snippets) => lang_snippets,
            Err(_) => LangSnippets::new(),
        },

        Err(_) => LangSnippets::new(),
    }
}
fn merge_snippets(mut s: LangSnippets, snippets: LangSnippets) -> LangSnippets {
    for (lang, lang_snippets) in snippets.into_iter() {
        if let Some(snippets) = s.get_mut(&lang) {
            for (snippet_key, snippet) in lang_snippets {
                snippets.insert(snippet_key, snippet);
            }
        } else {
            s.insert(lang, lang_snippets);
        }
    }

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        assert_eq!(
            SNIPPETS.clone(),
            parse(InitializeParams {
                ..Default::default()
            })
        );
    }

    #[test]
    fn merge_empty_snippets() {
        assert_eq!(
            LangSnippets::new(),
            merge_snippets(LangSnippets::new(), LangSnippets::new())
        );
    }

    #[test]
    fn add_lang() {
        let mut common_snippets = LangSnippets::new();
        let mut common_snippet = Snippets::new();
        common_snippet.insert("clo".to_string(), "console.log('$1:', $1)".to_string());
        common_snippets.insert("javascript".to_string(), common_snippet);

        let mut snippets = LangSnippets::new();
        let mut snippet = Snippets::new();
        snippet.insert("irb".to_string(), "binding.irb".to_string());
        snippets.insert("ruby".to_string(), snippet);

        let mut assert_snippets = LangSnippets::new();
        let mut ruby_snippet = Snippets::new();
        let mut javascript_snippet = Snippets::new();
        ruby_snippet.insert("irb".to_string(), "binding.irb".to_string());
        javascript_snippet.insert("clo".to_string(), "console.log('$1:', $1)".to_string());
        assert_snippets.insert("ruby".to_string(), ruby_snippet);
        assert_snippets.insert("javascript".to_string(), javascript_snippet);
        assert_eq!(assert_snippets, merge_snippets(common_snippets, snippets));
    }

    #[test]
    fn update_lang() {
        let mut common_snippets = LangSnippets::new();
        let mut common_snippet = Snippets::new();
        common_snippet.insert("pry".to_string(), "binding.pry".to_string());
        common_snippets.insert("ruby".to_string(), common_snippet);

        let mut snippets = LangSnippets::new();
        let mut snippet = Snippets::new();
        snippet.insert("pry".to_string(), "require 'pry'; binding.pry".to_string());
        snippets.insert("ruby".to_string(), snippet);

        let mut assert_snippets = LangSnippets::new();
        let mut ruby_snippet = Snippets::new();
        ruby_snippet.insert("pry".to_string(), "require 'pry'; binding.pry".to_string());
        assert_snippets.insert("ruby".to_string(), ruby_snippet);
        assert_eq!(assert_snippets, merge_snippets(common_snippets, snippets));
    }
}
