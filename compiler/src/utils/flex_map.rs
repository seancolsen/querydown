use std::collections::HashMap;

fn ascii_alphanumeric(s: &str) -> impl Iterator<Item = u8> + '_ {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| (c as u8))
}

/// Substitute for https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.eq_by which is
/// not yet stabilized.
fn eq_by<T>(
    mut a: impl Iterator<Item = T>,
    mut b: impl Iterator<Item = T>,
    eq: impl Fn(T, T) -> bool,
) -> bool {
    loop {
        match (a.next(), b.next()) {
            (None, None) => return true,
            (Some(_), None) | (None, Some(_)) => return false,
            (Some(a), Some(b)) => {
                if !eq(a, b) {
                    return false;
                }
            }
        }
    }
}

fn flex_eq(str_a: &str, str_b: &str) -> bool {
    eq_by(
        ascii_alphanumeric(str_a),
        ascii_alphanumeric(str_b),
        |a, b| a.eq_ignore_ascii_case(&b),
    )
}

pub trait FlexMap<T> {
    fn flex_get(&self, key: &str) -> Option<&T>;
}

impl<T> FlexMap<T> for HashMap<String, T> {
    fn flex_get(&self, search_key: &str) -> Option<&T> {
        let mut winning_value: Option<&T> = self.get(search_key);
        if winning_value.is_some() {
            return winning_value;
        }

        for (key, value) in self.iter() {
            if !flex_eq(key, search_key) {
                // Ignore entries that do not match the search key
                continue;
            }
            if winning_value.is_none() {
                // If this is the first match we have found, we store it
                winning_value = Some(value);
            } else {
                // If we have duplicate matches then we return None because we want to ensure that
                // the map is unambiguous
                return None;
            }
        }

        winning_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flex_eq() {
        // Equal
        assert!(flex_eq("foo", "foo"));
        assert!(flex_eq("", ""));
        assert!(flex_eq(
            "foo_bar",
            "FOO \"'`^#$%@~&|()[]{}<>/\\*+-_=!?.,:; \t\n\r BAR"
        ));
        assert!(flex_eq("foobar", "foo bar"));
        assert!(flex_eq("FOO Bar!", "foo bar."));

        // Not equal
        assert!(!flex_eq("a", "b"));
        assert!(!flex_eq("a", "aa"));
        assert!(!flex_eq("aa", "a"));
        assert!(!flex_eq("one", "onetwo"));
        assert!(!flex_eq("onetwo", "one"));
        assert!(!flex_eq("four", "föúr")); // Unicode -> ASCII transliteration is not supported
    }

    #[test]
    fn test_flex_map() {
        let mut map = HashMap::new();
        map.insert("one".to_string(), 1);
        map.insert("one two".to_string(), 12);
        map.insert("Two three".to_string(), 23);
        map.insert("two_three".to_string(), 23);
        map.insert("FIVE SIX".to_string(), 56);

        assert_eq!(map.flex_get("one"), Some(&1));
        assert_eq!(map.flex_get("ONE"), Some(&1));

        assert_eq!(map.flex_get("one two"), Some(&12));
        assert_eq!(map.flex_get("one_two"), Some(&12));
        assert_eq!(map.flex_get("oneTwo"), Some(&12));

        assert_eq!(map.flex_get("Two three"), Some(&23));
        assert_eq!(map.flex_get("two_three"), Some(&23));
        assert_eq!(map.flex_get("twoThree"), None);

        assert_eq!(map.flex_get("nope"), None);
    }
}
