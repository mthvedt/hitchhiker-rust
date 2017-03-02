use std::mem;

/*
Safe array rotation functions. In the long run, we want to replace most usages with memmoves
and uninitialized data.
*/
pub fn rotate_right<T>(arr: &mut [T], pos: usize) {
	if arr.len() < pos {
		panic!("{} out of bounds of array length {}", pos, arr.len())
	}
	// The 'swapping hands' algorithm. There are algorithms with faster constant time factors for large input,
	// but we don't have large input. TODO: investigate the above claim.

	// Suppose we start with abcde and want to end up with cdeab. pos = 2.
	// arr = abcde
	arr.reverse();
	// arr = edcba
	arr[..pos].reverse();
	// arr = decba
	arr[pos..].reverse();
	// arr = deabc
}

// pub fn rotate_left<T>(arr: &mut [T], pos: usize) {
// 	// borrow checker...
// 	let len = arr.len() - pos;
// 	rotate_right(arr, len);
// }

pub fn swap<T>(a: &mut [T], b: &mut [T]) {
	if a.len() != b.len() {
		panic!("mismatched slice swap");
	}

	// (Probably) autovectorized. TODO: check.
	for i in 0..a.len() {
		mem::swap(&mut a[i], &mut b[i]);
	}
}

/// Helper fn for inserting into an array. We assume there is room in the array, and it is ok to overwrite
/// the T at position arrsize.
// TODO: should be utility function.
pub fn rotate_in<T>(item: &mut T, arr: &mut [T]) {
	if arr.len() == 0 {
		// because rotate_in_slice can't panic
		panic!("cannot rotate into array length 0")
	}

	let mut dummy: [T; 1] = unsafe { mem::uninitialized() };
	mem::swap(item, &mut dummy[0]);
	rotate_in_slice(dummy.as_mut(), arr);
	mem::swap(item, &mut dummy[0]);
	mem::forget(dummy); // How odd this isn't unsafe...
}

// pub fn rotate_out<T>(arr: &mut [T], item: &mut T) {
// 	// Unfortunately we must be a little unsafe here, even though this is supposed to be a foolproof fn
// 	// Optimizer should remove the extra mem copies
// 	let mut dummy: [T; 1] = unsafe { mem::uninitialized() };
// 	mem::swap(item, &mut dummy[0]);
// 	rotate_out_slice(arr, dummy.as_mut());
// 	mem::swap(item, &mut dummy[0]);
// 	mem::forget(dummy); // How odd this isn't unsafe...
// }

pub fn rotate_in_slice<T>(src: &mut [T], dst: &mut [T]) {
	// for borrow checker
	let srclen = src.len();

	rotate_right(dst, srclen);
	swap(src, &mut dst[..srclen]);
}

// pub fn rotate_out_slice<T>(src: &mut [T], dst: &mut [T]) {
// 	// Borrow checker tricks
// 	let dstlen = dst.len();
// 	// Rotating forward len - n is equivalent to rotating backward n
// 	// let len = src.len() - dstlen;

// 	swap(&mut src[..dstlen], dst);
// 	rotate_left(src, dstlen);
// }
