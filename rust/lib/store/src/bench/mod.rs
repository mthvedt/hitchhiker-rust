extern crate test;
extern crate time;

use std::clone::Clone;
use std::convert::TryFrom;
use std::io::Write;

use self::time::*;

#[derive(Clone)]
pub enum BenchResult {
	Ok(Duration, u64),
	Fail(String),
	None,
}

pub struct Bencher {
	result: BenchResult,
}

impl Bencher {
	pub fn new() -> Bencher {
		Bencher {
			result: BenchResult::None,
		}
	}

	pub fn bench<T, F>(&mut self, count: u64, f: F) where F: FnOnce() -> T {
		let start = PreciseTime::now();

		let t = f();

		let end = PreciseTime::now();

		test::black_box(&t);

		self.result = BenchResult::Ok(start.to(end), count);
	}

	fn result(&self) -> BenchResult {
		self.result.clone()
	}
}

// TODO hide details up a module. User should only know macros and Bencher
pub trait Verifier {
	fn run<F, I>(f: F) where F: FnOnce() -> I;
	fn verify<F, MF, S>(message: MF, f: F) where F: FnOnce() -> bool, MF: FnOnce() -> S, S: ToString;
	fn verify_custom<F, S>(f: F) where F: FnOnce() -> Option<S>, S: ToString;
}

/// A verifier that does nothing.
pub struct NullVerifier {}

#[allow(unused_variables)]
impl Verifier for NullVerifier {
	fn run<F, I>(f: F) where F: FnOnce() -> I {}
	fn verify<F, MF, S>(message: MF, f: F) where F: FnOnce() -> bool, MF: FnOnce() -> S, S: ToString {}
	fn verify_custom<F, S>(f: F) where F: FnOnce() -> Option<S>, S: ToString {}
}

pub struct RealVerifier {}

impl Verifier for RealVerifier {
	fn run<F, I>(f: F) where F: FnOnce() -> I { f(); }

	fn verify<F, MF, S>(message: MF, f: F) where F: FnOnce() -> bool, MF: FnOnce() -> S, S: ToString {
		if !(f()) {
			panic!(message().to_string());
		}
	}

	fn verify_custom<F, S>(f: F) where F: FnOnce() -> Option<S>, S: ToString {
		match f() {
			Some(s) => panic!(s.to_string()),
			None => (),
		}
	}
}

/// A benchmarkable closure of some kind, post-parameterization.
// TODO: rename.
pub trait Benchable {
	fn name(&self) -> (String, String);
	fn bench(&self, b: &mut Bencher);
	fn verify(&self);
}

#[macro_export]
macro_rules! defbench {
	{ $name:ident, $id1:ident: $id1trait:ident, $idbencher:ident, $idverifier:ident, $e:expr } => {
		// Basically, we want to 'bundle' Testables into Benchables, parameterized by different kinds of Testable.
		// This ugly macro is the easiest way to do it: we get a nice bundle of Benchable at the end, parameterizable by
		// a statically-checked type. If we had HKTs or typeclasses like Haskell,
		// we could imagine a more elegant way using fn composition... but we have neither of those.

		fn $name<_T: $id1trait + Testable + 'static>() -> Box<Benchable> {
			// We have to have this local type to get around a limitation: rustc can't capture _T.
			struct _AnonBenchable<_Tcap: $id1trait + Testable + 'static> {
				_phantom: PhantomData<_Tcap>,
			}

			impl<_Tcap: $id1trait + Testable + 'static> _AnonBenchable<_Tcap> {
				fn _voldemort_bench<$idverifier: Verifier>(&self, $id1: &mut _Tcap, $idbencher: &mut Bencher) {
					$e
				}
			}

			impl<_Tcap: $id1trait + Testable + 'static> Benchable for _AnonBenchable<_Tcap> {
				fn name(&self) -> (String, String) { (String::from(stringify!($name)), <_Tcap as Testable>::name()) }

				fn bench(&self, b: &mut Bencher) {
					let mut t = _Tcap::setup();
					self._voldemort_bench::<NullVerifier>(&mut t, b);
					t.teardown();
				}

				fn verify(&self) {
					// Unused bencher
					let mut t = _Tcap::setup();
					let mut b = Bencher::new();
					self._voldemort_bench::<RealVerifier>(&mut t, &mut b);
					t.teardown();
				}
			}

			Box::new(_AnonBenchable::<_T> {
				_phantom: PhantomData,
			})
		}
	};
}

#[macro_export]
macro_rules! _create_benchmarks_helper {
	{ $vec:ident, $testable:ty, [ $($benchf:ident,)* ] } => {
		$(
	        $vec.push($benchf::<$testable>());
	    )*
    };
}

/// Create a vec of boxed deparameterized benchmarks, to be run at one's leisure.
#[macro_export]
macro_rules! create_benchmarks {
	// Rust allows us to put the separator inside or outside the parens,
	// but going outside breaks trailing commas. We put it inside because elegance > terseness.
	{ $([ $($testable:ty,)* ] => $benchf_list:tt,)* } => {
		{
			let mut _r: Vec<Box<Benchable>> = Vec::new();

			$($(
				_create_benchmarks_helper! {_r, $testable, $benchf_list}
	        )*)*

	        _r
    	}
    };
}

/// Needed becuase duration format is unstable/undocumented.
fn format_duration(d: &Duration) -> String {
	let c: f64;
	let s: &str;
	// If over 4 trillion nanoseconds, panic...
	let ns = d.num_nanoseconds().unwrap();
	let nsf = ns as f64;

	if ns < 1000 {
		c = nsf;
		s = "ns";
	} else if ns < 1000000 {
		c = nsf / 1000.0;
		s = "µs";
	} else if ns < 1000000000 {
		c = nsf / 1000000.0;
		s = "ms";
	} else {
		c = nsf / 1000000000.0;
		s = "s ";
	}

	format!("{:8.3} {}", c, s)
}

// TODO: catch panics
// TODO: pretty output
pub fn run_benchmark<W: Write>(benchmark: &Benchable, out: &mut W) {
	let (sa, sb) = benchmark.name();
	write!(out, "Benchmarking {:32} for {:16}...", sa, sb).unwrap();
	out.flush().unwrap();

	let mut b = Bencher::new();
	benchmark.bench(&mut b);
	match b.result() {
		// TODO: a little unsafe if there's over 2 billion iterations for some reason
		BenchResult::Ok(dur, count) => {
			if count <= 0 {
				panic!("zero or negative count");
			}
			write!(out, " {:10} iterations {}", count, format_duration(&(dur / i32::try_from(count).unwrap()))).unwrap();
			write!(out, " ...").unwrap();
			out.flush().unwrap();
			benchmark.verify();
			writeln!(out, " verified").unwrap();
		},
		BenchResult::Fail(s) => writeln!(out, "FAILED: {}", s).unwrap(),
		BenchResult::None => panic!(),
	}
}

pub fn run_benchmarks<W: Write>(benchmarks: &Vec<Box<Benchable>>, out: &mut W) {
	for b in benchmarks {
		run_benchmark(&**b, out);
	}
}
