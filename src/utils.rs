/// Retrieves a string between a start index and a number of characters given as `count` for a
/// given string
///
/// # Arguments
///
/// * `str`: The string
/// * `start`: The number of characters to skip from the beginning of the string
/// * `count`: The number of characters to be collected after the `start` parameter
///
/// returns: [`String`]
///
/// # Examples
///
/// ```
/// let data = "My car is red";
///
/// let out = get_string_between(data, 3, 6);
///
/// println!("{:?}", out) // "car is"
/// ```
pub(crate) fn get_string_between(str: &str, start: usize, count: usize) -> String {
	String::from_iter(str.chars().skip(start).take(count))
}

/// Retrieves a string between a start index and a specified character
///
/// # Arguments
///
/// * `str`: The string
/// * `start`: The number of characters to skip from the beginning of the string
/// * `char`: The character with which the string output collection will stop
///
/// returns: [`String`]
///
/// # Examples
///
/// ```
/// let data = "My car is red";
///
/// let out = get_string_until(data, 3, 'e');
///
/// println!("{:?}", out) // "car is r"
/// ```
pub(crate) fn get_string_until(str: &str, start: usize, char: char) -> String {
	String::from_iter(str.chars().skip(start).take_while(|c| *c != char))
}
