use std::ops::Add;

/// Slicing string by spaces with maximum row length
///
/// Parameters:
///  - text: input string
///  - max_row_ln: maximum row length
///
/// Return: vector of string rows
pub fn batch(text: &str, max_row_ln: usize) -> Vec<String> {
    text.split(" ").fold(Vec::new(), |mut buffer, val| {
        if let Some(last) = buffer.last_mut() {
            if last.count() + val.count() + 1 > max_row_ln {
                buffer.push(truncated(val.to_string(), max_row_ln))
            } else if last.count() > max_row_ln {
                truncate(buffer.last_mut().unwrap(), max_row_ln);
            } else {
                last.push_str(" ");
                last.push_str(val);
            }
        } else {
            buffer.push(truncated(val.to_string(), max_row_ln));
        }
        buffer
    })
}

fn truncate(s: &mut String, max_ln: usize) {
    let result = truncated(s.to_string(), max_ln);
    s.clear();
    s.push_str(result.as_str());
}

fn truncated(s: String, max_ln: usize) -> String {
    if s.count() <= max_ln {
        return s;
    }
    let _ln = max_ln - 3;
    s.chars()
        .collect::<Vec<char>>()
        .drain(0.._ln)
        .collect::<String>()
        .add("...")
}

/// Check contains trigger words in message
///
/// Parameters:
///  - words: comma separated trigger words
///  - message: message
///
/// Return: contains trigger words in vector
pub fn contains_in(words: String, message: String) -> Vec<String> {
    let l_msg = message.to_lowercase();
    words
        .to_lowercase()
        .split(",")
        .filter(|i| !i.is_empty())
        .filter_map(|i| l_msg.contains(&i).then_some(i.to_string()))
        .collect::<Vec<String>>()
}

/// Normalize input string with trigger words
///
/// Example:
///     normalize_words(String::from("test,, lol,")) // -> test,lol
///
/// Parameters:
///  - s: input string
///
/// Return: normalized string
pub fn normalize_words(s: String) -> String {
    s.trim()
        .to_lowercase()
        .replace(", ", ",")
        .split(",")
        .filter_map(|i| {
            if !i.is_empty() {
                Some(i.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<String>>()
        .join(",")
}

/// Universal string chars length
pub trait StringLn {
    fn count(&self) -> usize;
}

impl StringLn for str {
    fn count(&self) -> usize {
        self.chars().count()
    }
}

impl StringLn for String {
    fn count(&self) -> usize {
        self.chars().count()
    }
}
