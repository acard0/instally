use std::collections::HashMap;

use serde::{Serialize, Deserialize};

pub fn format_string(template: &str, replacements: &HashMap<String, String>, pattern: &str) -> String {
    let mut result = template.to_string();
    for (key, value) in replacements {
        let placeholder = pattern.replace("{}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TemplateFormat {
    replacements: HashMap<String, String>,
    pattern: String,
}

impl TemplateFormat {
    pub fn new() -> Self {
        Self {
            replacements: HashMap::new(),
            pattern: "@{{}}".to_string(),
        }
    }

    pub fn new_with_pattern(pattern: &str) -> Self {
        Self {
            replacements: HashMap::new(),
            pattern: pattern.to_string(),
        }
    }

    pub fn add_replacement(mut self, key: &str, value: &str) -> Self {
        self.replacements.insert(key.to_string(), value.to_string());
        self
    }

    pub fn format(&self, template: &str) -> String {
        format_string(template, &self.replacements, &self.pattern)
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<String, String> {
        self.replacements.iter()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::helpers::formatter::format_string;

    #[test]
    fn test_formatter() {
        let mut replacements = HashMap::new();
        replacements.insert("AnyApplication".to_string(), "Some Name".to_string());
        replacements.insert("AnyVersion".to_string(), "2.0.0".to_string());

        let pattern = "@{{}}";

        let template = "
        <Updates>
        <ApplicationName>@{AnyApplication}</ApplicationName>
        <ApplicationVersion>@{AnyVersion}</ApplicationVersion>
        <Checksum>true</Checksum>
        </Updates>
        ";

        let expected = "
        <Updates>
        <ApplicationName>Some Name</ApplicationName>
        <ApplicationVersion>2.0.0</ApplicationVersion>
        <Checksum>true</Checksum>
        </Updates>
        ";

        let r = format_string(&template, &replacements, &pattern);
        assert_eq!(r, expected, "Template formatter failed");
    }
}