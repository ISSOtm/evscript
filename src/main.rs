use string_interner::StringInterner;

fn main() {
	let mut identifiers = StringInterner::new();
	evscript::parsing::parse("", &mut identifiers).unwrap();
}
