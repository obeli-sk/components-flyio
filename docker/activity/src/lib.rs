mod containers;
mod docker_cli;
mod networks;
mod volumes;

mod generated {
    #![allow(clippy::empty_line_after_outer_attr)]
    include!(concat!(env!("OUT_DIR"), "/any.rs"));
}

use generated::export;

struct Component;

export!(Component with_types_in generated);
