use std::collections::HashMap;

pub fn format_string(template: &str, replacements: &HashMap<String, String>, pattern: &str) -> String {
    let mut result = template.to_string();
    for (key, value) in replacements {
        let placeholder = pattern.replace("{}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

pub struct TemplateFormat {
    replacements: HashMap<String, String>,
    value_transformer: Option<Box<dyn Fn(&str) -> String>>,
    pattern: String,
}

impl TemplateFormat {
    pub fn new(value_transformer: Option<Box<dyn Fn(&str) -> String>>) -> Self {
        Self {
            replacements: HashMap::new(),
            value_transformer,
            pattern: "@{{}}".to_string(),
        }
    }

    pub fn new_with_pattern(pattern: &str, value_transformer: Option<Box<dyn Fn(&str) -> String>>) -> Self {
        Self {
            replacements: HashMap::new(),
            value_transformer,
            pattern: pattern.to_string(),
        }
    }

    pub fn add_replacement(mut self, key: &str, value: &str) -> Self {
        let transformed_value = self.value_transformer.as_ref().map_or(value.to_owned(), |transformer| transformer(value));
        self.replacements.insert(key.to_string(), transformed_value);
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

        let template_json = r#"{"ApplicationName":"@{AnyApplication}","ApplicationVersion":@{AnyVersion}}"#;
        let expected_json = r#"{"ApplicationName":"Some Name","ApplicationVersion":2.0.0}"#;

        let r = format_string(&template, &replacements, &pattern);
        let r2 = format_string(&template_json, &replacements, pattern);

        assert_eq!(r, expected, "Template formatter failed for xml");
        assert_eq!(r2, expected_json, "Template formatter failed for json");
    }
}