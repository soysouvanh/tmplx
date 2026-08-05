use std::cell::UnsafeCell;

struct SourceArena {
    sources: UnsafeCell<Vec<Box<str>>>,
}

impl SourceArena {
    fn new() -> Self {
        Self { sources: UnsafeCell::new(Vec::new()) }
    }
    #[allow(clippy::mut_from_ref)]
    fn add(&self, source: String) -> &str {
        let b = source.into_boxed_str();
        let ptr = b.as_ref() as *const str;
        unsafe {
            (*self.sources.get()).push(b);
            &*ptr
        }
    }
}

fn main() {
    let arena = SourceArena::new();
    let s1 = arena.add("hello".to_string());
    let s2 = arena.add("world".to_string());
    println!("{} {}", s1, s2);
}
