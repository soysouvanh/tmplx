pub mod models;

pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/template_gen.rs"));
}
