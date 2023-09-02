// Grabbed from `https://stackoverflow.com/questions/3407012/rounding-up-to-the-nearest-multiple-of-a-number`
pub fn round_up_to_multiple(n: i64, mutliple: i64) -> i64 {
	if mutliple == 0 {
		return n;
	};

	let remainder = n.abs() % mutliple;
	if remainder == 0 {
		return n;
	};

	if n < 0 {
		-(n.abs() - remainder)
	} else {
		n + mutliple - remainder
	}
}
