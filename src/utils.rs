pub(crate) fn get_string_between(str: &str, start: usize, count: usize) -> String {
    let mut out = String::with_capacity(&start - &count);

    let mut n = 0;

    for c in str.chars().skip(start) {
        if n == count {
            break;
        }

        out.push(c);

        n += 1;
    }

    out
}