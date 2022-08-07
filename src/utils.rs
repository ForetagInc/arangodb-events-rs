pub(crate) fn get_string_between(str: &str, start: usize, count: usize) -> String {
	String::from_iter(str.chars().skip(start).take(count))
}
