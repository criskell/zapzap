//! The address book the ui asks about. The ui keeps room for a few dozen contacts only; the whole book
//! lives here and the ui asks for the names that match what the user typed (`CONTACTS query`), so its
//! memory does not grow with the number of contacts.

/// Letters folded to lowercase ASCII, the way the ui matches: case and accents do not count.
fn fold(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'Á' | 'À' | 'Â' | 'Ã' => 'a',
        'é' | 'ê' | 'É' | 'Ê' => 'e',
        'í' | 'Í' => 'i',
        'ó' | 'ô' | 'õ' | 'Ó' | 'Ô' | 'Õ' => 'o',
        'ú' | 'ü' | 'Ú' | 'Ü' => 'u',
        'ç' | 'Ç' => 'c',
        other => other.to_ascii_lowercase(),
    }
}

/// Numbers and symbols come before the letters, then names in alphabetical order without accents.
fn sort_key(name: &str) -> (bool, String) {
    let folded: String = name.chars().map(fold).collect();
    (folded.chars().next().is_some_and(|c| c.is_ascii_lowercase()), folded)
}

fn matches(name: &str, query: &str) -> bool {
    let (name, query): (String, String) = (name.chars().map(fold).collect(), query.chars().map(fold).collect());
    name.contains(&query)
}

#[derive(Default)]
pub struct ContactBook {
    /// Sorted by `sort_key`, one entry per name, with whatever identifies the contact to the connection
    /// (its JID) when that is known.
    entries: Vec<(String, Option<String>)>,
}

impl ContactBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `name`; a name already there, or an empty one, changes nothing.
    pub fn insert(&mut self, name: &str) {
        self.insert_with_id(name, None);
    }

    /// Like `insert`, and remembers `id` for the name when it had none.
    pub fn insert_with_id(&mut self, name: &str, id: Option<&str>) {
        let name = name.trim();
        if name.is_empty() || name.contains(['\t', '\n']) {
            return;
        }
        let key = sort_key(name);
        match self.entries.binary_search_by(|(known, _)| sort_key(known).cmp(&key)) {
            Ok(at) => {
                if self.entries[at].1.is_none() {
                    self.entries[at].1 = id.map(str::to_string);
                }
            }
            Err(at) => self.entries.insert(at, (name.to_string(), id.map(str::to_string))),
        }
    }

    /// What identifies the contact called exactly `name`, if it is known.
    pub fn id_of(&self, name: &str) -> Option<&str> {
        self.entries.iter().find(|(known, _)| known == name).and_then(|(_, id)| id.as_deref())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The first `limit` names, in order, that contain `query`.
    pub fn search(&self, query: &str, limit: usize) -> Vec<&str> {
        self.entries.iter().map(|(name, _)| name.as_str()).filter(|name| matches(name, query)).take(limit).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(names: &[&str]) -> ContactBook {
        let mut book = ContactBook::new();
        for name in names {
            book.insert(name);
        }
        book
    }

    #[test]
    fn numbers_come_first_then_letters_without_regard_to_accents_or_case() {
        let book = book(&["Zé", "Ana", "álvaro", "996985554", "bruno", "Ângela"]);
        assert_eq!(book.search("", 10), ["996985554", "álvaro", "Ana", "Ângela", "bruno", "Zé"]);
    }

    #[test]
    fn a_contact_remembers_the_id_it_was_first_given() {
        let mut book = ContactBook::new();
        book.insert("Ana");
        assert_eq!(book.id_of("Ana"), None);
        book.insert_with_id("Ana", Some("5511@s.whatsapp.net"));
        book.insert_with_id("Ana", Some("other"));
        assert_eq!(book.id_of("Ana"), Some("5511@s.whatsapp.net"));
        assert_eq!(book.id_of("Bia"), None);
    }

    #[test]
    fn a_name_is_kept_once() {
        let book = book(&["Ana", "Ana", " Ana "]);
        assert_eq!(book.len(), 1);
    }

    #[test]
    fn empty_and_unsafe_names_are_ignored() {
        let book = book(&["", "  ", "a\tb", "a\nb"]);
        assert!(book.is_empty());
    }

    #[test]
    fn searching_ignores_accents_and_case_and_finds_the_middle_of_a_name() {
        let book = book(&["João Silva", "Maria", "Sílvia"]);
        assert_eq!(book.search("SILV", 10), ["João Silva", "Sílvia"]);
        assert_eq!(book.search("joao", 10), ["João Silva"]);
    }

    #[test]
    fn the_limit_keeps_the_first_names() {
        let book = book(&["a", "b", "c", "d"]);
        assert_eq!(book.search("", 2), ["a", "b"]);
    }
}
