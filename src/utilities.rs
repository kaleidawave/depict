use std::borrow::Cow;
use std::ffi::OsString;

#[derive(Clone, Debug)]
pub struct Sorting {
    pub field: String,
    pub direction: Direction,
}

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Ascending,
    Descending,
}

impl Direction {
    pub fn compare<T: std::cmp::Ord>(self, a: &T, b: &T) -> std::cmp::Ordering {
        let order = a.cmp(b);
        if let Self::Ascending = self {
            order.reverse()
        } else {
            order
        }
    }
}

/// Used for printing numbers in SDE
#[must_use]
pub fn count_with_seperator(value: usize) -> String {
    const NON_BREAKING_SPACE: &str = "\u{00A0}";

    to_denary(value, NON_BREAKING_SPACE)
}

fn to_denary(value: usize, seperator: &str) -> String {
    if value == 0 {
        return "0".to_owned();
    }
    let mut buf = String::new();
    for i in (0..=value.ilog10()).rev() {
        let j = (value / 10i32.pow(i) as usize) % 10;
        buf.push(b"0123456789"[j] as char);
        if i > 0 && i % 3 == 0 {
            buf.push_str(seperator);
        }
    }
    buf
}

pub struct PairedWriter<W1, W2>(Option<W1>, Option<W2>);

impl<W1, W2> PairedWriter<W1, W2> {
    pub fn new_from_option(first: Option<W1>, second: Option<W2>) -> Option<Self> {
        if first.is_some() || second.is_some() {
            Some(Self(first, second))
        } else {
            None
        }
    }
}

impl<W1, W2> std::io::Write for PairedWriter<W1, W2>
where
    W1: std::io::Write,
    W2: std::io::Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(first) = self.0.as_mut() {
            let result = std::io::Write::write(first, buf)?;
            if let Some(second) = self.1.as_mut() {
                std::io::Write::write(second, buf)
            } else {
                Ok(result)
            }
        } else {
            std::io::Write::write(self.1.as_mut().unwrap(), buf)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(first) = self.0.as_mut() {
            std::io::Write::flush(first)?;
        }
        if let Some(second) = self.1.as_mut() {
            std::io::Write::flush(second)?;
        }
        Ok(())
    }
}

/// n-ary Cartesian product
pub struct BigCartesianProduct<T> {
    choices: Vec<Vec<T>>,
    remaining: usize,
}

impl<T> BigCartesianProduct<T> {
    #[must_use]
    pub fn new(choices: Vec<Vec<T>>) -> Self {
        let remaining = choices.iter().map(std::vec::Vec::len).product();
        Self { choices, remaining }
    }
}

impl<T> Iterator for BigCartesianProduct<T>
where
    T: Clone,
{
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining > 0 {
            let mut cur = self.remaining - 1;
            let mut items: Vec<T> = Vec::with_capacity(self.choices.len());
            for choices in &self.choices {
                // invert the index purely for aesthetics
                let idx = choices.len() - ((cur % choices.len()) + 1);
                let item = choices[idx].clone();
                cur /= choices.len();
                items.push(item);
            }
            self.remaining -= 1;
            Some(items)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

pub struct BigCartesianProductMap<K, V> {
    keys: Vec<K>,
    product: BigCartesianProduct<V>,
}

impl<K, V> BigCartesianProductMap<K, V> {
    pub fn new(map: impl IntoIterator<Item = (K, Vec<V>)>) -> Self {
        let (keys, values): (Vec<K>, Vec<Vec<V>>) = map.into_iter().unzip();
        let product = BigCartesianProduct::new(values);
        Self { keys, product }
    }
}

impl<K, V> Iterator for BigCartesianProductMap<K, V>
where
    K: Clone,
    V: Clone,
{
    type Item = Vec<(K, V)>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.product.next()?;
        let paired: Vec<_> = self.keys.iter().map(Clone::clone).zip(item).collect();
        Some(paired)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.product.size_hint()
    }
}

/// ```
/// let template = depict::utilities::Template::new("Hello $name!", '$');
/// let values = std::collections::HashMap::from([("name", "Ben")]);
/// assert_eq!(
///     &template.interpolate(|key| values[key].into()),
///     "Hello Ben!"
/// );
/// ```
pub struct Template<'t> {
    pub items: Vec<&'t str>,
}

const EMPTY_SLOT: &str = "";

impl<'t> Template<'t> {
    #[must_use]
    pub fn new(on: &'t str, interpolate_character: char) -> Self {
        let mut items = Vec::new();
        let mut start = 0;
        for (idx, _matched) in on.match_indices(interpolate_character) {
            // Double interpolate character => escaping
            let escaped = on[..idx].ends_with(interpolate_character);
            if escaped {
                items.push(&on[start..=idx]);
                items.push(EMPTY_SLOT);
                start = idx + 1;
            } else {
                items.push(&on[start..idx]);
                let rest = &on[idx..][1..];
                let name_end = rest
                    .find(|c: char| !(c.is_alphanumeric() || matches!(c, '_')))
                    .unwrap_or(rest.len());

                items.push(&rest[..name_end]);
                start = idx + 1 + name_end;
            }
        }
        items.push(&on[start..]);
        Self { items }
    }

    /// FUTURE maybe `Cow`?
    pub fn interpolate<'a>(&'a self, mut cb: impl FnMut(&'t str) -> Cow<'a, str>) -> String {
        let mut buf = String::new();
        for (idx, item) in self.items.iter().enumerate() {
            if idx % 2 == 0 {
                buf.push_str(item);
            } else if item != &EMPTY_SLOT {
                buf.push_str(&cb(item));
            }
        }
        buf
    }

    /// FUTURE maybe `Cow`?
    pub fn interpolate_os<'a>(&'a self, mut cb: impl FnMut(&'t str) -> Cow<'a, str>) -> OsString {
        let mut buf = OsString::new();
        for (idx, item) in self.items.iter().enumerate() {
            if idx % 2 == 0 {
                buf.push(item);
            } else if item != &EMPTY_SLOT {
                buf.push(&*cb(item));
            }
        }
        buf
    }
}

#[cfg(test)]
mod template_tests {
    use super::Template;

    #[test]
    fn basic() {
        let template = Template::new("Hello $name!", '$');
        let values = std::collections::HashMap::from([("name", "Ben")]);
        assert_eq!(
            &template.interpolate(|key| values[key].into()),
            "Hello Ben!"
        );
    }

    #[test]
    fn escape() {
        let template = Template::new("Hello $name! Here is $$20!", '$');
        let values = std::collections::HashMap::from([("name", "Ben")]);
        assert_eq!(
            &template.interpolate(|key| values[key].into()),
            "Hello Ben! Here is $20!"
        );
    }
}
