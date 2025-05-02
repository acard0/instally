use std:: collections::HashMap;

use rust_i18n::SimpleBackend;

#[derive(Debug, Clone)]
pub struct I18n {
    additional: HashMap<String, HashMap<String, String>>,
}

impl I18n {
    pub fn new() -> Self {
        Self { 
            additional: HashMap::new()
        }
    }

    pub fn get(&self, locale: &str, key: &str) -> String {
        self.loose_translation(locale, key)
            .unwrap_or_else(|| key.to_owned())
    }

    fn loose_translation(&self, locale: &str, key: &str) -> Option<String> {
        let additional = self.additional.get(locale);
        let existing = self.get_translations().get(locale);
        self.loose_translation_internal(existing, additional, key)
    }

    fn loose_translation_internal(&self, existing: Option<&HashMap<String, String>>, additional: Option<&HashMap<String, String>>, query: &str) -> Option<String> {
        let mut parts: Vec<&str> = query.split('.').collect();
    
        while !parts.is_empty() {
            let current_key = parts.join(".");
            
            if let Some(existing_map) = existing {
                if let Some(attempt) = existing_map.get(&current_key) {
                    return Some(attempt.to_owned());
                }
            }
            
            if let Some(additional_map) = additional {
                if let Some(attempt) = additional_map.get(&current_key) {
                    return Some(attempt.to_owned());
                }
            }
            
            parts.remove(0);
        }
    
        None
    }    

    fn in_place(&self, locale: &str, key: &str) -> Option<String> {
        self.additional.get(locale)?.get(key).cloned()
    }

    fn get_translations(&self) -> &'static HashMap<String, HashMap<String, String>> {
        // rust_i18n does not expose translations resolved by it

        // what is the point of using rust anyway?
        let simple_backend: &SimpleBackend = unsafe {
            if size_of::<usize>() == 4 {
                &*(((self as *const I18n as usize) - size_of::<usize>() - 28) as *const SimpleBackend)
            } else {
                &*(((self as *const I18n as usize) - size_of::<usize>() - 40) as *const SimpleBackend)
            }
        };

        let translations: &HashMap<String, HashMap<String, String>> = unsafe {
            &*(simple_backend as *const SimpleBackend as *const HashMap<String, HashMap<String, String>>)
        };

        translations
    }
}

impl rust_i18n::Backend for I18n {
    fn available_locales(&self) -> Vec<String> {
        self.additional.keys().cloned().collect()
    }

    fn translate(&self, locale: &str, key: &str) -> Option<String> {
        Some(self.get(locale, key))
    }

    fn add(&mut self, locale: &str, key: &str, value: &str) {
        let locale = self.additional.entry(locale.to_string()).or_insert_with(HashMap::new);
        locale.insert(key.to_string(), value.to_string());
    }
}