use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        default_parsers_primitives: { feature = "default_parsers_primitives" },
        default_parsers_models: { feature = "default_parsers_models" },
        default_parsers_time: { feature = "default_parsers_time" },
        default_parsers: { any(default_parsers_primitives, default_parsers_models, default_parsers_time) },
    }
}
