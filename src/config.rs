use regex::{Captures, regex};
use std::{collections::HashMap, fs::read_to_string};

#[cfg(all(feature = "yaml-config", not(feature = "toml-config")))]
use crate::feat::server::VetisConfig;

pub(super) fn parse_file(file_contents: &str) -> Result<VetisConfig, Box<dyn std::error::Error>> {
    let empty = String::default();

    let mut resolved_files = HashMap::new();
    let includes_regex = regex!(r"\$\(include_file\s*=\s*(.*)\)");
    let mut matches = includes_regex.captures_iter(file_contents);
    while let Some(matches) = matches.next() {
        let (full, [file_path]) = matches.extract();
        match read_to_string(file_path) {
            Ok(file_contents) => {
                resolved_files.insert(full, file_contents);
            }
            Err(e) => {
                panic!("Could not include file: {}", e.to_string());
            }
        }
    }
    let file_contents = includes_regex.replace_all(file_contents, |caps: &Captures| {
        resolved_files
            .get(&caps[0])
            .unwrap_or(&empty)
    });

    let mut resolved_vars = HashMap::new();
    let env_vars = regex!("\\$\\(([a-zA-Z_]*):{0,1}([\\\"\\\']{0,1}.*[\\\"\\\']{0,1})\\)");
    let mut matches = env_vars.captures_iter(&file_contents);
    while let Some(matches) = matches.next() {
        let (full, [name, default]) = matches.extract();
        let value = match std::env::var_os(name) {
            Some(var_value) => {
                format!("{:?}", var_value)
            }
            None => default.to_string(),
        };
        resolved_vars.insert(full, value);
    }

    let file_contents = env_vars.replace_all(&file_contents, |caps: &Captures| {
        resolved_vars
            .get(&caps[0])
            .unwrap_or(&empty)
    });

    #[cfg(all(feature = "yaml-config", feature = "toml-config"))]
    let config = match serde_saphyr::from_str::<VetisConfig>(&file_contents) {
        Ok(config) => config,
        Err(yaml_error) => toml::from_str::<VetisConfig>(&file_contents).map_err(|toml_error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Could not parse config as YAML ({yaml_error}) or TOML ({toml_error})"),
            )
        })?,
    };
    #[cfg(all(feature = "yaml-config", not(feature = "toml-config")))]
    let config = serde_saphyr::from_str::<VetisConfig>(&file_contents)?;
    #[cfg(all(feature = "toml-config", not(feature = "yaml-config")))]
    let config = toml::from_str::<VetisConfig>(&file_contents)?;
    Ok(config)
}
